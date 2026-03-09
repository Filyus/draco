// Debug test for Speed 1 decode failure
mod common;

use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::geometry_indices::{PointIndex, FaceIndex};
use draco_core::EncoderOptions;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::draco_types::DataType;

fn create_test_mesh() -> Mesh {
    let grid_size = 100;  // Use same size as comprehensive test
    let num_points = grid_size * grid_size;
    let num_faces = (grid_size - 1) * (grid_size - 1) * 2;
    
    // Create positions
    let mut positions = Vec::with_capacity(num_points * 3);
    for y in 0..grid_size {
        for x in 0..grid_size {
            let px = x as f32;
            let py = y as f32;
            let pz = (x as f32 * 0.2).sin() * (y as f32 * 0.2).cos() * 2.0;
            positions.push(px);
            positions.push(py);
            positions.push(pz);
        }
    }
    
    // Create faces
    let mut faces = Vec::with_capacity(num_faces * 3);
    for y in 0..grid_size - 1 {
        for x in 0..grid_size - 1 {
            let p0 = (y * grid_size + x) as u32;
            let p1 = (y * grid_size + x + 1) as u32;
            let p2 = ((y + 1) * grid_size + x) as u32;
            let p3 = ((y + 1) * grid_size + x + 1) as u32;
            
            faces.push(p0);
            faces.push(p1);
            faces.push(p2);
            
            faces.push(p1);
            faces.push(p3);
            faces.push(p2);
        }
    }
    
    let mut mesh = Mesh::new();
    mesh.set_num_points(num_points);
    mesh.set_num_faces(num_faces);
    
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(GeometryAttributeType::Position, 3, DataType::Float32, false, num_points);
    
    for i in 0..num_points {
        let offset = i * 3 * 4;
        pos_attr.buffer_mut().update(&positions[i * 3].to_le_bytes(), Some(offset));
        pos_attr.buffer_mut().update(&positions[i * 3 + 1].to_le_bytes(), Some(offset + 4));
        pos_attr.buffer_mut().update(&positions[i * 3 + 2].to_le_bytes(), Some(offset + 8));
    }
    mesh.add_attribute(pos_attr);
    
    for i in 0..num_faces {
        mesh.set_face(
            FaceIndex(i as u32),
            [
                PointIndex(faces[i * 3]),
                PointIndex(faces[i * 3 + 1]),
                PointIndex(faces[i * 3 + 2]),
            ]
        );
    }
    
    mesh
}

#[test]
fn test_speed1_encode_decode() {
    common::disable_noisy_debug_env();
    
    for speed in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
        println!("\n=== Testing Speed {} Encode/Decode ===\n", speed);
        
        let mesh = create_test_mesh();
        let orig_points = mesh.num_points();
        let orig_faces = mesh.num_faces();
        
        // Encode with given speed
        let mut options = EncoderOptions::new();
        options.set_global_int("encoding_speed", speed);
        options.set_global_int("decoding_speed", speed);
        options.set_attribute_int(0, "quantization_bits", 10);
        
        let mut encoder = MeshEncoder::new();
        encoder.set_mesh(mesh.clone());
        let mut encoder_buffer = EncoderBuffer::new();
        
        match encoder.encode(&options, &mut encoder_buffer) {
            Ok(_) => {
                let encoded_data = encoder_buffer.data();
                println!("Speed {}: Encoding successful: {} bytes", speed, encoded_data.len());
                
                // Decode
                let mut decoder_buffer = DecoderBuffer::new(encoded_data);
                let mut out_mesh = Mesh::new();
                let mut decoder = MeshDecoder::new();
                
                match decoder.decode(&mut decoder_buffer, &mut out_mesh) {
                    Ok(_) => {
                        println!("Speed {}: DECODE SUCCESS! {} points, {} faces", speed, out_mesh.num_points(), out_mesh.num_faces());
                        assert_eq!(out_mesh.num_points(), orig_points, "Speed {}: Point count mismatch", speed);
                        assert_eq!(out_mesh.num_faces(), orig_faces, "Speed {}: Face count mismatch", speed);
                    }
                    Err(e) => {
                        panic!("Speed {}: DECODE FAILED: {}", speed, e);
                    }
                }
            }
            Err(e) => {
                panic!("Speed {}: ENCODE FAILED: {}", speed, e);
            }
        }
    }
}
