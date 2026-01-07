//! Debug test to compare Rust decode vs C++ decode

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::draco_types::DataType;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::encoder_options::EncoderOptions;
use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
use draco_core::geometry_indices::PointIndex;
use draco_core::mesh::Mesh;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::mesh_encoder::MeshEncoder;

fn cpp_decoder() -> Option<PathBuf> {
    if let Ok(build_dir) = std::env::var("DRACO_CPP_BUILD_DIR") {
        let dec = PathBuf::from(&build_dir).join("draco_decoder.exe");
        if dec.exists() {
            return Some(dec);
        }
    }
    let build_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("build")
        .join("Debug");
    let dec = build_dir.join("draco_decoder.exe");
    if dec.exists() { Some(dec) } else { None }
}

fn parse_obj_positions(obj_content: &str) -> Vec<[f32; 3]> {
    let mut positions = Vec::new();
    for line in obj_content.lines() {
        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let x: f32 = parts[1].parse().unwrap_or(0.0);
                let y: f32 = parts[2].parse().unwrap_or(0.0);
                let z: f32 = parts[3].parse().unwrap_or(0.0);
                positions.push([x, y, z]);
            }
        }
    }
    positions
}

fn read_position_from_buffer(buffer: &[u8], index: usize) -> [f32; 3] {
    let offset = index * 12;
    let x = f32::from_le_bytes([buffer[offset], buffer[offset+1], buffer[offset+2], buffer[offset+3]]);
    let y = f32::from_le_bytes([buffer[offset+4], buffer[offset+5], buffer[offset+6], buffer[offset+7]]);
    let z = f32::from_le_bytes([buffer[offset+8], buffer[offset+9], buffer[offset+10], buffer[offset+11]]);
    [x, y, z]
}

#[test]
fn compare_rust_vs_cpp_decode() {
    let decoder_exe = cpp_decoder().expect("C++ decoder not found");

    // Create a simple cube mesh
    let positions: Vec<f32> = vec![
        // Front face
        -1.0, -1.0,  1.0,
         1.0, -1.0,  1.0,
         1.0,  1.0,  1.0,
        -1.0,  1.0,  1.0,
        // Back face
        -1.0, -1.0, -1.0,
        -1.0,  1.0, -1.0,
         1.0,  1.0, -1.0,
         1.0, -1.0, -1.0,
    ];

    let normals: Vec<f32> = vec![
        0.0, 0.0, 1.0,
        0.0, 0.0, 1.0,
        0.0, 0.0, 1.0,
        0.0, 0.0, 1.0,
        0.0, 0.0, -1.0,
        0.0, 0.0, -1.0,
        0.0, 0.0, -1.0,
        0.0, 0.0, -1.0,
    ];

    let uvs: Vec<f32> = vec![
        0.0, 0.0,
        1.0, 0.0,
        1.0, 1.0,
        0.0, 1.0,
        1.0, 0.0,
        1.0, 1.0,
        0.0, 1.0,
        0.0, 0.0,
    ];

    let indices: Vec<u32> = vec![
        0, 1, 2, 2, 3, 0,
        4, 5, 6, 6, 7, 4,
    ];

    let vertex_count = positions.len() / 3;
    let face_count = indices.len() / 3;

    // Build mesh
    let mut draco_mesh = Mesh::new();

    let mut pos_attr = PointAttribute::new();
    pos_attr.init(GeometryAttributeType::Position, 3, DataType::Float32, false, vertex_count);
    for (i, chunk) in positions.chunks(3).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        pos_attr.buffer_mut().write(i * 12, &bytes);
    }
    draco_mesh.add_attribute(pos_attr);

    let mut norm_attr = PointAttribute::new();
    norm_attr.init(GeometryAttributeType::Normal, 3, DataType::Float32, false, vertex_count);
    for (i, chunk) in normals.chunks(3).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        norm_attr.buffer_mut().write(i * 12, &bytes);
    }
    draco_mesh.add_attribute(norm_attr);

    let mut uv_attr = PointAttribute::new();
    uv_attr.init(GeometryAttributeType::TexCoord, 2, DataType::Float32, false, vertex_count);
    for (i, chunk) in uvs.chunks(2).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        uv_attr.buffer_mut().write(i * 8, &bytes);
    }
    draco_mesh.add_attribute(uv_attr);

    for i in 0..face_count {
        draco_mesh.add_face([
            PointIndex(indices[i * 3]),
            PointIndex(indices[i * 3 + 1]),
            PointIndex(indices[i * 3 + 2]),
        ]);
    }

    // Encode with quantization
    let mut encoder = MeshEncoder::new();
    let mut enc_buffer = EncoderBuffer::new();
    let mut enc_options = EncoderOptions::default();
    
    let pos_id = draco_mesh.named_attribute_id(GeometryAttributeType::Position);
    enc_options.set_attribute_int(pos_id, "quantization_bits", 14);
    let norm_id = draco_mesh.named_attribute_id(GeometryAttributeType::Normal);
    enc_options.set_attribute_int(norm_id, "quantization_bits", 10);
    let uv_id = draco_mesh.named_attribute_id(GeometryAttributeType::TexCoord);
    enc_options.set_attribute_int(uv_id, "quantization_bits", 12);

    encoder.set_mesh(draco_mesh);
    encoder.encode(&enc_options, &mut enc_buffer).expect("Encode failed");

    let draco_bytes = enc_buffer.data().to_vec();
    println!("Encoded {} bytes", draco_bytes.len());

    // Rust decode
    let mut dec_buffer = DecoderBuffer::new(&draco_bytes);
    let mut rust_decoder = MeshDecoder::new();
    let mut rust_mesh = Mesh::new();
    rust_decoder.decode(&mut dec_buffer, &mut rust_mesh).expect("Rust decode failed");

    let rust_pos_buffer = rust_mesh.attribute(0).buffer().data();
    
    println!("\n=== RUST DECODED POSITIONS ===");
    for i in 0..rust_mesh.num_points() {
        let pos = read_position_from_buffer(rust_pos_buffer, i);
        println!("  Point {}: {:?}", i, pos);
    }

    // C++ decode
    let tmp = std::env::temp_dir().join("draco_compare_test");
    fs::create_dir_all(&tmp).ok();
    let drc_path = tmp.join("test.drc");
    let obj_path = tmp.join("test.obj");
    
    fs::write(&drc_path, &draco_bytes).expect("Failed to write DRC");
    
    let output = Command::new(&decoder_exe)
        .args(["-i", drc_path.to_string_lossy().as_ref(), "-o", obj_path.to_string_lossy().as_ref()])
        .output()
        .expect("Failed to run C++ decoder");
    
    assert!(output.status.success(), "C++ decoder failed: {}", 
        String::from_utf8_lossy(&output.stderr));

    let obj_content = fs::read_to_string(&obj_path).expect("Failed to read OBJ");
    let cpp_positions = parse_obj_positions(&obj_content);

    println!("\n=== C++ DECODED POSITIONS ===");
    for (i, pos) in cpp_positions.iter().enumerate() {
        println!("  Point {}: {:?}", i, pos);
    }

    println!("\n=== COMPARISON ===");
    println!("Original   -> Rust decode  -> C++ decode");
    for i in 0..vertex_count {
        let orig = [positions[i*3], positions[i*3+1], positions[i*3+2]];
        
        let rust_pos = read_position_from_buffer(rust_pos_buffer, i);
        let cpp_pos = if i < cpp_positions.len() { cpp_positions[i] } else { [999.0; 3] };
        
        let rust_ok = (orig[0] - rust_pos[0]).abs() < 0.01 
                   && (orig[1] - rust_pos[1]).abs() < 0.01 
                   && (orig[2] - rust_pos[2]).abs() < 0.01;
        let cpp_ok = (orig[0] - cpp_pos[0]).abs() < 0.01 
                  && (orig[1] - cpp_pos[1]).abs() < 0.01 
                  && (orig[2] - cpp_pos[2]).abs() < 0.01;
        
        let rust_status = if rust_ok { "✓" } else { "✗" };
        let cpp_status = if cpp_ok { "✓" } else { "✗" };
        
        println!("  {}: {:?} -> {:?} {} -> {:?} {}", 
            i, orig, rust_pos, rust_status, cpp_pos, cpp_status);
    }
}
