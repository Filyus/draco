use draco_core::point_cloud::PointCloud;
use draco_core::point_cloud_encoder::PointCloudEncoder;
use draco_core::point_cloud_decoder::PointCloudDecoder;
use draco_core::encoder_options::EncoderOptions;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
use draco_core::draco_types::DataType;

#[test]
fn test_normal_encoding_decoding() {
    let mut pc = PointCloud::new();
    pc.set_num_points(4);

    // Add Normal attribute
    let mut att = PointAttribute::new();
    att.init(GeometryAttributeType::Normal, 3, DataType::Float32, false, 4);
    
    // Set some normal values (unit vectors)
    // (1, 0, 0), (0, 1, 0), (0, 0, 1), (0.577, 0.577, 0.577)
    let normals: Vec<f32> = vec![
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
        0.57735, 0.57735, 0.57735
    ];
    
    // Write data to buffer
    // PointAttribute buffer expects bytes.
    let mut byte_data = Vec::with_capacity(normals.len() * 4);
    for val in &normals {
        byte_data.extend_from_slice(&val.to_le_bytes());
    }
    att.buffer_mut().write(0, &byte_data);
    
    let att_id = pc.add_attribute(att);

    let mut options = EncoderOptions::default();
    options.set_attribute_int(att_id, "quantization_bits", 10); // 10 bits for better precision

    let mut encoder = PointCloudEncoder::new();
    encoder.set_point_cloud(pc);
    
    let mut out_buffer = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut out_buffer);
    assert!(status.is_ok(), "Encoding failed: {:?}", status.err());

    let mut decoder = PointCloudDecoder::new();
    let mut in_buffer = DecoderBuffer::new(out_buffer.data());
    let mut out_pc = PointCloud::new();
    let status = decoder.decode(&mut in_buffer, &mut out_pc);
    assert!(status.is_ok(), "Decoding failed: {:?}", status.err());

    assert_eq!(out_pc.num_points(), 4);
    assert_eq!(out_pc.num_attributes(), 1);
    
    let out_att = out_pc.attribute(0);
    assert_eq!(out_att.attribute_type(), GeometryAttributeType::Normal);
    
    // Check values
    let buffer = out_att.buffer();
    let data = buffer.data();
    
    for i in 0..4 {
        let offset = i * 3 * 4;
        let x = f32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
        let y = f32::from_le_bytes(data[offset+4..offset+8].try_into().unwrap());
        let z = f32::from_le_bytes(data[offset+8..offset+12].try_into().unwrap());
        
        let expected_x = normals[i*3];
        let expected_y = normals[i*3+1];
        let expected_z = normals[i*3+2];
        
        // Error tolerance for 10 bits quantization
        let tolerance = 0.01;
        
        assert!((x - expected_x).abs() < tolerance, "Point {}: x mismatch: got {}, expected {}", i, x, expected_x);
        assert!((y - expected_y).abs() < tolerance, "Point {}: y mismatch: got {}, expected {}", i, y, expected_y);
        assert!((z - expected_z).abs() < tolerance, "Point {}: z mismatch: got {}, expected {}", i, z, expected_z);
    }
}
