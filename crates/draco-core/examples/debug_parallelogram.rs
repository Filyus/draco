use draco_core::mesh::Mesh;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::draco_types::DataType;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_options::EncoderOptions;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::geometry_indices::{PointIndex, FaceIndex};

fn main() {
    let mut mesh = Mesh::new();
    let mut pos_att = PointAttribute::new();
    
    // Create a 5x5 grid
    let width = 5;
    let height = 5;
    let num_points = width * height;
    pos_att.init(GeometryAttributeType::Position, 3, DataType::Float32, false, num_points);
    
    let buffer = pos_att.buffer_mut();
    let mut positions_vec = Vec::new();
    
    for y in 0..height {
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;
            let pz = 0.0f32;
            positions_vec.push([px, py, pz]);
            
            let bytes = [
                px.to_le_bytes(),
                py.to_le_bytes(),
                pz.to_le_bytes(),
            ].concat();
            buffer.write((y * width + x) * 12, &bytes);
        }
    }
    
    mesh.add_attribute(pos_att);
    
    // Faces
    mesh.set_num_faces((width - 1) * (height - 1) * 2);
    let mut face_idx = 0;
    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            // v00 -- v10
            //  |      |
            // v01 -- v11
            let v00 = (y * width + x) as u32;
            let v10 = (y * width + (x + 1)) as u32;
            let v01 = ((y + 1) * width + x) as u32;
            let v11 = ((y + 1) * width + (x + 1)) as u32;
            
            // Triangle 1: v00, v10, v01
            mesh.set_face(FaceIndex(face_idx), [PointIndex(v00), PointIndex(v10), PointIndex(v01)]);
            face_idx += 1;
            
            // Triangle 2: v10, v11, v01
            mesh.set_face(FaceIndex(face_idx), [PointIndex(v10), PointIndex(v11), PointIndex(v01)]);
            face_idx += 1;
        }
    }
    
    println!("=== Input mesh ===");
    println!("Num points: {}", mesh.num_points());
    println!("Num faces: {}", mesh.num_faces());
    
    // Encode
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);
    
    let mut options = EncoderOptions::new();
    options.set_attribute_int(0, "quantization_bits", 10);
    options.set_prediction_scheme(1); // Force Parallelogram prediction
    
    let mut enc_buffer = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc_buffer);
    println!("Encoding status: {:?}", status);
    
    if status.is_err() {
        println!("Encoding failed: {:?}", status.err());
        return;
    }
    
    println!("Encoded size: {} bytes", enc_buffer.data().len());
    
    // Decode
    let mut dec_buffer = DecoderBuffer::new(enc_buffer.data());
    let mut decoded_mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut dec_buffer, &mut decoded_mesh);
    println!("Decoding status: {:?}", status);
    
    if status.is_err() {
        println!("Decoding failed: {:?}", status.err());
        return;
    }
    
    println!("=== Decoded mesh ===");
    println!("Num points: {}", decoded_mesh.num_points());
    println!("Num faces: {}", decoded_mesh.num_faces());
    
    // Check decoded positions
    let att = decoded_mesh.attribute(0);
    let dec_buffer_data = att.buffer();
    println!("\n=== Decoded positions ===");
    for i in 0..decoded_mesh.num_points() {
        let mut bytes = [0u8; 12];
        dec_buffer_data.read(i * 12, &mut bytes);
        let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        println!("Point {}: ({}, {}, {})", i, x, y, z);
    }
    
    // Check that all original positions exist
    println!("\n=== Validation ===");
    let mut missing_count = 0;
    for (i, orig_pos) in positions_vec.iter().enumerate() {
        let mut found = false;
        for j in 0..decoded_mesh.num_points() {
            let mut bytes = [0u8; 12];
            dec_buffer_data.read(j * 12, &mut bytes);
            let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
            let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
            let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
            if (x - orig_pos[0]).abs() < 0.01 
                && (y - orig_pos[1]).abs() < 0.01 
                && (z - orig_pos[2]).abs() < 0.01 {
                found = true;
                if cfg!(feature = "debug_logs") {
                    println!("Original point {} ({:?}) found at decoded point {}", i, orig_pos, j);
                }
                break;
            }
        }
        if !found {
            println!("MISSING: Original point {} ({:?}) NOT found in decoded mesh!", i, orig_pos);
            missing_count += 1;
        }
    }
    
    if missing_count == 0 {
        println!("SUCCESS: All {} points found.", num_points);
    } else {
        println!("FAILURE: {} points missing.", missing_count);
    }
}
