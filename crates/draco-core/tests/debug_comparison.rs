use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::geometry_indices::{PointIndex, FaceIndex};
use draco_core::EncoderOptions;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::draco_types::DataType;
use std::process::Command;
use std::path::Path;
use std::fs::File;
use std::io::Write;

fn get_cpp_tools_path() -> Option<std::path::PathBuf> {
    let path = Path::new("../../build/Debug");
    if path.exists() {
        Some(path.to_path_buf())
    } else {
        let path = Path::new("../../build/Release");
        if path.exists() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }
}

fn create_simple_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    mesh.set_num_points(3);
    mesh.set_num_faces(1);
    
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(GeometryAttributeType::Position, 3, DataType::Float32, false, 3);
    
    let points = [
        [0.0f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    
    for i in 0..3 {
        let offset = i * 3 * 4;
        pos_attr.buffer_mut().update(&points[i][0].to_le_bytes(), Some(offset));
        pos_attr.buffer_mut().update(&points[i][1].to_le_bytes(), Some(offset + 4));
        pos_attr.buffer_mut().update(&points[i][2].to_le_bytes(), Some(offset + 8));
    }
    mesh.add_attribute(pos_attr);
    
    mesh.set_face(FaceIndex(0), [PointIndex(0), PointIndex(1), PointIndex(2)]);
    
    mesh
}

fn write_obj(mesh: &Mesh, path: &Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let pos_attr = mesh.attribute(0);
    
    for i in 0..mesh.num_points() {
        let offset = i * 3 * 4;
        let bytes = &pos_attr.buffer().data()[offset..offset+12];
        let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        writeln!(file, "v {} {} {}", x, y, z)?;
    }
    
    for i in 0..mesh.num_faces() {
        let face = mesh.face(FaceIndex(i as u32));
        writeln!(file, "f {} {} {}", face[0].0 + 1, face[1].0 + 1, face[2].0 + 1)?;
    }
    Ok(())
}

fn print_hex(label: &str, data: &[u8]) {
    println!("{}:", label);
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:04X}: ", i * 16);
        for b in chunk {
            print!("{:02X} ", b);
        }
        println!();
    }
}

#[test]
fn compare_encodings() {
    let tools_path = match get_cpp_tools_path() {
        Some(p) => p,
        None => {
            println!("Skipping comparison: C++ tools not found");
            return;
        }
    };
    let encoder_path = tools_path.join("draco_encoder.exe");
    if !encoder_path.exists() {
        println!("Skipping comparison: draco_encoder.exe not found");
        return;
    }

    let mesh = create_simple_mesh();
    
    // Rust Encode
    let mut options = EncoderOptions::new();
    options.set_global_int("encoding_method", 1); // Edgebreaker
    options.set_attribute_int(0, "quantization_bits", 10);
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut encoder_buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut encoder_buffer).expect("Encode failed");
    let rust_data = encoder_buffer.data().to_vec();
    
    // C++ Encode
    let obj_path = Path::new("temp_simple.obj");
    write_obj(&mesh, obj_path).expect("Failed to write obj");
    let drc_path = Path::new("temp_simple_cpp.drc");
    
    let output = Command::new(&encoder_path)
        .arg("-i")
        .arg(obj_path)
        .arg("-o")
        .arg(drc_path)
        .arg("-method")
        .arg("edgebreaker")
        .arg("-cl")
        .arg("0")
        .arg("-qp")
        .arg("10")
        .output()
        .expect("Failed to run draco_encoder");
        
    if !output.status.success() {
        println!("C++ encoder failed: {}", String::from_utf8_lossy(&output.stderr));
        return;
    }
    
    let mut file = File::open(drc_path).expect("Failed to open cpp drc");
    let mut cpp_data = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut cpp_data).expect("Failed to read cpp drc");
    
    print_hex("Rust Output", &rust_data);
    print_hex("C++ Output", &cpp_data);
    
    // Clean up
    let _ = std::fs::remove_file(obj_path);
    let _ = std::fs::remove_file(drc_path);
}
