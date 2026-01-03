use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::geometry_indices::{PointIndex, FaceIndex};
use draco_core::EncoderOptions;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::draco_types::DataType;
use std::process::Command;
use std::path::Path;
use std::fs::File;
use std::io::Write;

fn read_ply_header(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    // PLY header is always ASCII and terminated by end_header\n.
    let marker = b"end_header\n";
    let end = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .map(|pos| pos + marker.len())
        .unwrap_or(bytes.len());
    Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn get_cpp_tools_path() -> Option<std::path::PathBuf> {
    let path = Path::new("../../build/Debug");
    if path.exists() {
        Some(path.to_path_buf())
    } else {
        // Try Release
        let path = Path::new("../../build/Release");
        if path.exists() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }
}

fn create_torus_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    mesh.set_num_points(4);
    mesh.set_num_faces(2);
    
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(GeometryAttributeType::Position, 3, DataType::Float32, false, 4);
    
    let coords: Vec<f32> = vec![
        0.0, 0.0, 0.0,
        1.0, 0.0, 0.0,
        1.0, 1.0, 0.0,
        0.0, 1.0, 0.0,
    ];
    
    for i in 0..4 {
        let offset = i * 3 * 4;
        pos_attr.buffer_mut().update(&coords[i*3].to_le_bytes(), Some(offset));
        pos_attr.buffer_mut().update(&coords[i*3+1].to_le_bytes(), Some(offset + 4));
        pos_attr.buffer_mut().update(&coords[i*3+2].to_le_bytes(), Some(offset + 8));
    }
    mesh.add_attribute(pos_attr);
    
    // f 0 1 2
    // f 0 2 3
    mesh.set_face(FaceIndex(0), [PointIndex(0), PointIndex(1), PointIndex(2)]);
    mesh.set_face(FaceIndex(1), [PointIndex(0), PointIndex(2), PointIndex(3)]);
    
    mesh
}

fn write_obj(mesh: &Mesh, path: &Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let pos_attr = mesh.attribute(0);
    
    for i in 0..mesh.num_points() {
        let _val_idx = draco_core::geometry_indices::AttributeValueIndex(i as u32);
        // Assuming float32 position
        // We need a way to read typed data from attribute. 
        // For now, let's just assume we can get bytes and cast, or use a helper if available.
        // The current API might be limited. Let's use the buffer directly.
        let offset = i * 3 * 4; // 3 floats * 4 bytes
        let bytes = &pos_attr.buffer().data()[offset..offset+12];
        let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        writeln!(file, "v {} {} {}", x, y, z)?;
    }
    
    for i in 0..mesh.num_faces() {
        let face = mesh.face(FaceIndex(i as u32));
        // OBJ is 1-based
        writeln!(file, "f {} {} {}", face[0].0 + 1, face[1].0 + 1, face[2].0 + 1)?;
    }
    Ok(())
}

#[test]
fn test_rust_encode_cpp_decode() {
    let tools_path = match get_cpp_tools_path() {
        Some(p) => p,
        None => {
            println!("Skipping compatibility test: C++ tools not found");
            return;
        }
    };
    let decoder_path = tools_path.join("draco_decoder.exe");
    let encoder_path = tools_path.join("draco_encoder.exe");
    if !decoder_path.exists() || !encoder_path.exists() {
        println!("Skipping compatibility test: tools not found");
        return;
    }

    let mesh = create_torus_mesh();
    
    // Write OBJ for C++ encoder
    let obj_path = Path::new("torus.obj");
    write_obj(&mesh, obj_path).expect("Failed to write OBJ");
    
    // Run C++ encoder
    let cpp_drc_path = Path::new("cpp_encoded.drc");
    let status = Command::new(&encoder_path)
        .arg("-i")
        .arg(obj_path)
        .arg("-o")
        .arg(cpp_drc_path)
        .arg("-method")
        .arg("edgebreaker")
        .arg("-qp")
        .arg("10")
        .status()
        .expect("Failed to run draco_encoder");
    assert!(status.success(), "C++ encoder failed");
    
    let mut options = EncoderOptions::new();
    options.set_global_int("encoding_method", 1); // Edgebreaker
    options.set_attribute_int(0, "quantization_bits", 10);
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut encoder_buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut encoder_buffer).expect("Encode failed");
    
    let drc_path = Path::new("rust_encoded.drc");
    let mut file = File::create(drc_path).expect("Failed to create drc file");
    file.write_all(encoder_buffer.data()).expect("Failed to write drc file");
    
    // Compare files
    // We don't compare binary data directly because different encoders might produce different valid streams.
    // Instead we verify the C++ decoder can decode our output.
    
    let output = Command::new(&decoder_path)
        .arg("-i")
        .arg(drc_path)
        .output()
        .expect("Failed to run draco_decoder");
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("Decoder stdout: {}", stdout);
    println!("Decoder stderr: {}", stderr);
    
    assert!(output.status.success(), "C++ decoder failed");

    // Verify decoded geometry by reading the generated PLY file
    let ply_path = Path::new("rust_encoded.drc.ply");
    if ply_path.exists() {
        let ply_content = read_ply_header(ply_path).expect("Failed to read PLY file header");
        assert!(ply_content.contains("element vertex 4"), "Decoded mesh has incorrect number of points");
        assert!(ply_content.contains("element face 2"), "Decoded mesh has incorrect number of faces");
    } else {
        // If PLY file is not generated, check stdout for stats (older draco versions)
        // But since we saw "saved to .ply", it should be there.
        // If not found, maybe CWD issue?
        // Try to find it in current dir
        let current_dir = std::env::current_dir().unwrap();
        println!("Current dir: {:?}", current_dir);
        panic!("PLY file not found at {:?}", ply_path);
    }
    // let _ = std::fs::remove_file(drc_path);
    // let _ = std::fs::remove_file(cpp_drc_path);
    // let _ = std::fs::remove_file(obj_path);
    
    assert!(output.status.success(), "C++ decoder failed");
}

#[test]
fn test_rust_encode_rust_decode() {
    let mesh = create_torus_mesh();
    
    let mut options = EncoderOptions::new();
    options.set_global_int("encoding_method", 0); // Sequential
    options.set_attribute_int(0, "quantization_bits", 10);
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut encoder_buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut encoder_buffer).expect("Encode failed");
    
    let data = encoder_buffer.data();
    let mut decoder_buffer = DecoderBuffer::new(data);
    
    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = Mesh::new();
    let status = decoder.decode(&mut decoder_buffer, &mut decoded_mesh);
    
    assert!(status.is_ok(), "Rust decoder failed: {:?}", status);
    assert_eq!(decoded_mesh.num_points(), 4);
    assert_eq!(decoded_mesh.num_faces(), 2);
}

#[test]
fn test_cpp_encode_rust_decode() {
    let tools_path = match get_cpp_tools_path() {
        Some(p) => p,
        None => {
            println!("Skipping compatibility test: C++ tools not found");
            return;
        }
    };
    let encoder_path = tools_path.join("draco_encoder.exe");
    if !encoder_path.exists() {
        println!("Skipping compatibility test: draco_encoder.exe not found");
        return;
    }

    let mesh = create_torus_mesh();
    let obj_path = Path::new("temp_input.obj");
    write_obj(&mesh, obj_path).expect("Failed to write obj");
    
    let drc_path = Path::new("temp_cpp_out.drc");
    
    let output = Command::new(&encoder_path)
        .arg("-i")
        .arg(obj_path)
        .arg("-o")
        .arg(drc_path)
        .arg("-method")
        .arg("edgebreaker") // or "1"
        .arg("-cl")
        .arg("0")
        .arg("-qp")
        .arg("10") // quantization bits
        .output()
        .expect("Failed to run draco_encoder");
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Encoder stdout: {}", stdout);
    println!("Encoder stderr: {}", stderr);
    
    assert!(output.status.success(), "C++ encoder failed");
    
    let metadata = std::fs::metadata(drc_path).unwrap();
    println!("C++ encoded size: {}", metadata.len());
    
    // Decode with Rust
    let mut file = File::open(drc_path).expect("Failed to open drc file");
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer).expect("Failed to read drc file");
    
    let mut decoder = MeshDecoder::new();
    let mut decoder_buffer = DecoderBuffer::new(&buffer);
    let mut decoded_mesh = Mesh::new();
    
    let status = decoder.decode(&mut decoder_buffer, &mut decoded_mesh);
    match status {
        Ok(_) => {
            assert_eq!(decoded_mesh.num_faces(), mesh.num_faces());
            assert_eq!(decoded_mesh.num_points(), mesh.num_points());
            println!("Rust decoder successfully decoded C++ stream");
        },
        Err(e) => {
            panic!("Rust decoder failed: {:?}", e);
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file(obj_path);
    let _ = std::fs::remove_file(drc_path);
}

#[test]
fn test_rust_encode_rust_decode_edgebreaker() {
    let mesh = create_torus_mesh();
    
    let mut options = EncoderOptions::new();
    options.set_global_int("encoding_method", 1); // Edgebreaker
    options.set_attribute_int(0, "quantization_bits", 10);
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut encoder_buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut encoder_buffer).expect("Encode failed");
    
    let data = encoder_buffer.data();
    let mut decoder_buffer = DecoderBuffer::new(data);
    
    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = Mesh::new();
    let status = decoder.decode(&mut decoder_buffer, &mut decoded_mesh);
    
    assert!(status.is_ok(), "Rust decoder failed: {:?}", status);
    assert_eq!(decoded_mesh.num_points(), 4);
    assert_eq!(decoded_mesh.num_faces(), 2);
}
