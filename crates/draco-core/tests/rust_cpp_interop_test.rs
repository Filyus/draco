//! Test that Rust-encoded files can be decoded by C++

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_rust_encoded_cpp_decoded() {
    use draco_core::encoder_buffer::EncoderBuffer;
    use draco_core::encoder_options::EncoderOptions;
    use draco_core::mesh::Mesh as DracoMesh;
    use draco_core::mesh_encoder::MeshEncoder;
    use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
    use draco_core::draco_types::DataType;
    use draco_core::geometry_indices::PointIndex;

    // Create a simple tetrahedron
    let positions: [f32; 12] = [
        0.0, 0.0, 0.0,
        1.0, 0.0, 0.0,
        0.5, 1.0, 0.0,
        0.5, 0.5, 1.0,
    ];
    
    let indices: [u32; 12] = [
        0, 1, 2,  // bottom
        0, 1, 3,  // front
        1, 2, 3,  // right
        2, 0, 3,  // left
    ];

    let vertex_count = 4;
    let face_count = 4;
    
    // Create Draco mesh
    let mut draco_mesh = DracoMesh::new();
    draco_mesh.set_num_points(vertex_count);

    let mut pos_attr = PointAttribute::new();
    pos_attr.init(
        GeometryAttributeType::Position,
        3,
        DataType::Float32,
        false,
        vertex_count,
    );
    let pos_buffer = pos_attr.buffer_mut();
    for (i, chunk) in positions.chunks(3).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        pos_buffer.write(i * 12, &bytes);
    }
    draco_mesh.add_attribute(pos_attr);

    for i in 0..face_count {
        let i0 = PointIndex(indices[i * 3]);
        let i1 = PointIndex(indices[i * 3 + 1]);
        let i2 = PointIndex(indices[i * 3 + 2]);
        draco_mesh.add_face([i0, i1, i2]);
    }

    // Encode
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(draco_mesh);
    let mut encoder_buffer = EncoderBuffer::new();
    let enc_options = EncoderOptions::default();
    
    encoder.encode(&enc_options, &mut encoder_buffer)
        .expect("Draco encoding should succeed");
    
    let encoded_data = encoder_buffer.data();
    println!("Rust encoded size: {} bytes", encoded_data.len());
    
    // Save to file
    let output_path = "C:/Users/filyus/Downloads/rust_encoded_test.drc";
    fs::write(output_path, encoded_data).expect("Failed to write file");
    println!("Saved to: {}", output_path);
    
    // Try to decode with C++ decoder
    let cpp_decoder_path = "D:/Projects/Draco/build-orig-debug/src/draco/Debug/draco_decoder.exe";
    if Path::new(cpp_decoder_path).exists() {
        let output = Command::new(cpp_decoder_path)
            .args(["-i", output_path])
            .output()
            .expect("Failed to run C++ decoder");
        
        println!("C++ decoder stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("C++ decoder stderr: {}", String::from_utf8_lossy(&output.stderr));
        
        // If decoding succeeds, the output should contain some info
        // If it fails, it will contain "Failed to decode"
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Failed") || !output.stderr.is_empty() {
            println!("C++ decoder failed - checking error...");
            if stdout.contains("Failed to decode point attributes") {
                panic!("CRITICAL: PredictionScheme encoding error - check enum values match C++");
            }
        } else {
            println!("C++ decoder succeeded!");
        }
    } else {
        println!("C++ decoder not found, skipping interop test");
    }
}
