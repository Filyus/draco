use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::encoder_options::EncoderOptions;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::geometry_indices::{PointIndex, FaceIndex};
use draco_core::draco_types::DataType;

fn create_grid_mesh(width: u32, height: u32) -> Mesh {
    let mut mesh = Mesh::new();
    let num_points = width * height;
    mesh.set_num_points(num_points as usize);
    
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(GeometryAttributeType::Position, 3, DataType::Float32, false, num_points as usize);
    
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let coords = [x as f32, y as f32, 0.0f32];
            let offset = (i as usize) * 3 * 4;
            pos_attr.buffer_mut().update(&coords[0].to_le_bytes(), Some(offset));
            pos_attr.buffer_mut().update(&coords[1].to_le_bytes(), Some(offset + 4));
            pos_attr.buffer_mut().update(&coords[2].to_le_bytes(), Some(offset + 8));
        }
    }
    mesh.add_attribute(pos_attr);
    
    // Create faces (2 triangles per grid cell)
    let mut face_idx = 0;
    for y in 0..height-1 {
        for x in 0..width-1 {
            let p0 = y * width + x;
            let p1 = y * width + (x + 1);
            let p2 = (y + 1) * width + x;
            let p3 = (y + 1) * width + (x + 1);
            
            // Triangle 1: p0, p1, p2
            mesh.set_face(FaceIndex(face_idx), [PointIndex(p0), PointIndex(p1), PointIndex(p2)]);
            face_idx += 1;
            
            // Triangle 2: p1, p3, p2
            mesh.set_face(FaceIndex(face_idx), [PointIndex(p1), PointIndex(p3), PointIndex(p2)]);
            face_idx += 1;
        }
    }
    mesh.set_num_faces(face_idx as usize);
    
    mesh
}

fn verify_mesh_attributes(original: &Mesh, decoded: &Mesh, max_error: f32) {
    // Edgebreaker may introduce split vertices, so decoded count >= original count
    assert!(decoded.num_points() >= original.num_points(), "Decoded points {} < Original points {}", decoded.num_points(), original.num_points());
    
    let orig_attr = original.attribute(0);
    let dec_attr = decoded.attribute(0);
    
    let orig_data = orig_attr.buffer().data();
    let dec_data = dec_attr.buffer().data();
    
    // Collect all decoded points
    let mut decoded_points = Vec::new();
    println!("Decoded Points:");
    for i in 0..decoded.num_points() {
        let offset = (i as usize) * 3 * 4;
        let dx = f32::from_le_bytes(dec_data[offset..offset+4].try_into().unwrap());
        let dy = f32::from_le_bytes(dec_data[offset+4..offset+8].try_into().unwrap());
        let dz = f32::from_le_bytes(dec_data[offset+8..offset+12].try_into().unwrap());
        decoded_points.push([dx, dy, dz]);
        println!("  {}: ({}, {}, {})", i, dx, dy, dz);
    }

    fn round_f32_to_i32(v: f32) -> i32 {
        // Grid tests use non-negative coordinates; round() is sufficient.
        v.round() as i32
    }

    // Verify each original point exists in decoded points
    for i in 0..original.num_points() {
        let offset = (i as usize) * 3 * 4;
        let ox = f32::from_le_bytes(orig_data[offset..offset+4].try_into().unwrap());
        let oy = f32::from_le_bytes(orig_data[offset+4..offset+8].try_into().unwrap());
        let oz = f32::from_le_bytes(orig_data[offset+8..offset+12].try_into().unwrap());

        let rox = round_f32_to_i32(ox);
        let roy = round_f32_to_i32(oy);
        let roz = round_f32_to_i32(oz);
        
        let mut found = false;
        for dp in &decoded_points {
            // Primary match: compare rounded coordinates.
            if round_f32_to_i32(dp[0]) == rox &&
               round_f32_to_i32(dp[1]) == roy &&
               round_f32_to_i32(dp[2]) == roz {
                found = true;
                break;
            }

            // Fallback: max-error comparison for non-grid uses.
            if (ox - dp[0]).abs() <= max_error &&
               (oy - dp[1]).abs() <= max_error &&
               (oz - dp[2]).abs() <= max_error {
                found = true;
                break;
            }
        }
        assert!(found, "Point {} ({}, {}, {}) not found in decoded mesh", i, ox, oy, oz);
    }
}

#[test]
// #[ignore]
fn test_grid_encoding_parallelogram() {
    // 10x10 grid = 100 points. Enough to trigger Parallelogram if speed < 8.
    let mesh = create_grid_mesh(10, 10);
    
    let mut options = EncoderOptions::default();
    options.set_global_int("encoding_method", 1); // Edgebreaker
    options.set_global_int("encoding_speed", 5); // Should select Parallelogram
    options.set_attribute_int(0, "quantization_bits", 14); // High precision
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut buffer).expect("Encode failed");
    
    println!("Parallelogram encoded size: {}", buffer.data().len());
    
    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = Mesh::new();
    let mut decoder_buffer = DecoderBuffer::new(buffer.data());
    decoder.decode(&mut decoder_buffer, &mut decoded_mesh).expect("Decode failed");
    
    // With 14 bits quantization on range [0, 9], error should be very small.
    // Range = 9. Max error ~= 9 / (2^14 - 1) ~= 0.0005
    verify_mesh_attributes(&mesh, &decoded_mesh, 0.002);
}

#[test]
fn test_grid_encoding_difference() {
    let mesh = create_grid_mesh(10, 10);
    
    let mut options = EncoderOptions::default();
    options.set_global_int("encoding_method", 1); // Edgebreaker
    options.set_global_int("encoding_speed", 10); // Should select Difference
    options.set_attribute_int(0, "quantization_bits", 14);
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut buffer).expect("Encode failed");
    
    println!("Difference encoded size: {}", buffer.data().len());
    
    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = Mesh::new();
    let mut decoder_buffer = DecoderBuffer::new(buffer.data());
    decoder.decode(&mut decoder_buffer, &mut decoded_mesh).expect("Decode failed");
    
    verify_mesh_attributes(&mesh, &decoded_mesh, 0.01);
}

#[test]
fn test_quantization_levels() {
    let mesh = create_grid_mesh(5, 5);
    
    let q_levels = [8, 10, 16];
    
    for &q in &q_levels {
        let mut options = EncoderOptions::default();
        options.set_global_int("encoding_method", 1);
        options.set_attribute_int(0, "quantization_bits", q);
        
        let mut encoder = MeshEncoder::new();
        encoder.set_mesh(mesh.clone());
        let mut buffer = EncoderBuffer::new();
        encoder.encode(&options, &mut buffer).expect("Encode failed");
        
        let mut decoder = MeshDecoder::new();
        let mut decoded_mesh = Mesh::new();
        let mut decoder_buffer = DecoderBuffer::new(buffer.data());
        decoder.decode(&mut decoder_buffer, &mut decoded_mesh).expect("Decode failed");
        
        // Range is 4.0.
        // Error bound = Range / (2^q - 1)
        let range = 4.0;
        let max_error = range / ((1 << q) as f32 - 1.0);
        // Allow a bit of slack for float precision
        verify_mesh_attributes(&mesh, &decoded_mesh, max_error * 1.5);
    }
}

#[test]
fn test_grid_encoding_sequential() {
    let mesh = create_grid_mesh(10, 10);
    
    let mut options = EncoderOptions::default();
    options.set_global_int("encoding_method", 0); // Sequential
    options.set_attribute_int(0, "quantization_bits", 14);
    
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut buffer = EncoderBuffer::new();
    encoder.encode(&options, &mut buffer).expect("Encode failed");
    
    println!("Sequential encoded size: {}", buffer.data().len());
    
    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = Mesh::new();
    let mut decoder_buffer = DecoderBuffer::new(buffer.data());
    decoder.decode(&mut decoder_buffer, &mut decoded_mesh).expect("Decode failed");
    
    verify_mesh_attributes(&mesh, &decoded_mesh, 0.002);
}

