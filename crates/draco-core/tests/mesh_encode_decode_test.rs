use draco_core::mesh::Mesh;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::draco_types::DataType;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_options::EncoderOptions;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::geometry_indices::{PointIndex, FaceIndex};

#[test]
fn test_mesh_encode_decode() {
    let mut mesh = Mesh::new();
    let mut pos_att = PointAttribute::new();
    
    let num_points = 4;
    pos_att.init(GeometryAttributeType::Position, 3, DataType::Float32, false, num_points);
    
    // A simple quad (2 triangles)
    // 0 --- 1
    // | \   |
    // |  \  |
    // 3 --- 2
    let positions: [f32; 12] = [
        0.0, 0.0, 0.0, // 0
        1.0, 0.0, 0.0, // 1
        1.0, 1.0, 0.0, // 2
        0.0, 1.0, 0.0, // 3
    ];
    
    let buffer = pos_att.buffer_mut();
    for i in 0..num_points {
        let bytes = [
            positions[i*3].to_le_bytes(),
            positions[i*3+1].to_le_bytes(),
            positions[i*3+2].to_le_bytes(),
        ].concat();
        buffer.write(i * 12, &bytes);
    }
    
    mesh.add_attribute(pos_att);
    
    // Faces
    mesh.set_num_faces(2);
    mesh.set_face(FaceIndex(0), [PointIndex(0), PointIndex(1), PointIndex(2)]);
    mesh.set_face(FaceIndex(1), [PointIndex(0), PointIndex(2), PointIndex(3)]);
    
    // Encode
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);
    
    let mut options = EncoderOptions::new();
    options.set_attribute_int(0, "quantization_bits", 10);
    
    let mut enc_buffer = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc_buffer);
    assert!(status.is_ok(), "Encoding failed: {:?}", status.err());
    
    // Decode
    let mut dec_buffer = DecoderBuffer::new(enc_buffer.data());
    let mut decoded_mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut dec_buffer, &mut decoded_mesh);
    assert!(status.is_ok(), "Decoding failed: {:?}", status.err());
    
    assert_eq!(decoded_mesh.num_faces(), 2);
    assert_eq!(decoded_mesh.num_points(), 4);
    
    // Check faces
    let f0 = decoded_mesh.face(FaceIndex(0));
    assert_eq!(f0[0], PointIndex(0));
    assert_eq!(f0[1], PointIndex(1));
    assert_eq!(f0[2], PointIndex(2));
    
    let f1 = decoded_mesh.face(FaceIndex(1));
    assert_eq!(f1[0], PointIndex(0));
    assert_eq!(f1[1], PointIndex(2));
    assert_eq!(f1[2], PointIndex(3));
    
    // Check attributes
    let decoded_att = decoded_mesh.attribute(0);
    assert_eq!(decoded_att.attribute_type(), GeometryAttributeType::Position);
    
    // Check values (approximate)
    let decoded_buffer = decoded_att.buffer();
    for i in 0..num_points {
        let mut bytes = [0u8; 12];
        decoded_buffer.read(i * 12, &mut bytes);
        let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let y = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let z = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        
        let ex = positions[i*3];
        let ey = positions[i*3+1];
        let ez = positions[i*3+2];
        
        assert!((x - ex).abs() < 0.01);
        assert!((y - ey).abs() < 0.01);
        assert!((z - ez).abs() < 0.01);
    }
}
