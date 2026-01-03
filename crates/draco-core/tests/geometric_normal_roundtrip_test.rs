
use draco_core::mesh::Mesh;
use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
use draco_core::draco_types::DataType;
use draco_core::geometry_indices::{PointIndex, VertexIndex, CornerIndex};
use draco_core::mesh_prediction_scheme_data::MeshPredictionSchemeData;
use draco_core::prediction_scheme_geometric_normal::{
    PredictionSchemeGeometricNormalEncoder,
    PredictionSchemeGeometricNormalDecoder,
    PredictionSchemeGeometricNormalEncodingTransform,
    PredictionSchemeGeometricNormalDecodingTransform,
};
use draco_core::prediction_scheme::{PredictionSchemeEncoder, PredictionSchemeDecoder, PredictionScheme};
use draco_core::normal_compression_utils::OctahedronToolBox;
use draco_core::corner_table::CornerTable;
use draco_core::decoder_buffer::DecoderBuffer;

#[test]
fn test_geometric_normal_roundtrip() {
    // 1. Create Mesh
    let mut mesh = Mesh::new();
    
    // Position Attribute
    let mut pos_att = PointAttribute::new();
    pos_att.init(GeometryAttributeType::Position, 3, DataType::Int32, false, 4);
    {
        let buffer = pos_att.buffer_mut();
        // 0: (0,0,0)
        buffer.write(0, &0i32.to_le_bytes());
        buffer.write(4, &0i32.to_le_bytes());
        buffer.write(8, &0i32.to_le_bytes());
        // 1: (10,0,0)
        buffer.write(12, &10i32.to_le_bytes());
        buffer.write(16, &0i32.to_le_bytes());
        buffer.write(20, &0i32.to_le_bytes());
        // 2: (0,10,0)
        buffer.write(24, &0i32.to_le_bytes());
        buffer.write(28, &10i32.to_le_bytes());
        buffer.write(32, &0i32.to_le_bytes());
        // 3: (10,10,0)
        buffer.write(36, &10i32.to_le_bytes());
        buffer.write(40, &10i32.to_le_bytes());
        buffer.write(44, &0i32.to_le_bytes());
    }
    pos_att.set_identity_mapping();
    let pos_att_id = mesh.add_attribute(pos_att);
    
    // Faces
    mesh.add_face([PointIndex(0), PointIndex(1), PointIndex(2)]);
    mesh.add_face([PointIndex(2), PointIndex(1), PointIndex(3)]);
    
    // 2. Create CornerTable
    let mut corner_table = CornerTable::new(2);
    let faces = vec![
        [VertexIndex(0), VertexIndex(1), VertexIndex(2)],
        [VertexIndex(2), VertexIndex(1), VertexIndex(3)],
    ];
    corner_table.init(&faces);
    
    // Manually set opposites
    // Face 0: (0,1,2). Edge 1-2 (opp c0).
    // Face 1: (2,1,3). Edge 2-1 (opp c5).
    corner_table.set_opposite(CornerIndex(0), CornerIndex(5));
    corner_table.set_opposite(CornerIndex(5), CornerIndex(0));
    
    corner_table.compute_vertex_corners(4);
    
    // 3. Prepare Data for Prediction Scheme
    let vertex_to_data_map = vec![0, 1, 2, 3];
    let data_to_corner_map = vec![0, 1, 2, 5]; // c0, c1, c2, c5
    
    let mut mesh_data = MeshPredictionSchemeData::new();
    mesh_data.set(&corner_table, &data_to_corner_map, &vertex_to_data_map);
    
    // 4. Prepare Normal Data (to be encoded)
    // Normals are (0, 0, 1) for all vertices.
    // In Octahedral coordinates, (0, 0, 1) maps to center of octahedron?
    // (0, 0, 1) -> s=0.5, t=0.5 in [0,1] space.
    // Quantized to 10 bits (1023).
    // Center is around 512, 512.
    
    let quantization_bits = 10;
    let _max_value = (1 << quantization_bits) - 1;
    let mut tool_box = OctahedronToolBox::new();
    tool_box.set_quantization_bits(quantization_bits);
    
    let mut normal_3d = [0, 0, 1]; // Integer vector
    tool_box.canonicalize_integer_vector(&mut normal_3d);
    
    let (s, t) = tool_box.integer_vector_to_quantized_octahedral_coords(&normal_3d);
    // s, t are i32.
    
    let mut in_data = vec![0i32; 8]; // 4 points * 2 components
    for i in 0..4 {
        in_data[i*2] = s;
        in_data[i*2+1] = t;
    }
    
    // 5. Encode
    let mut transform = PredictionSchemeGeometricNormalEncodingTransform::new();
    transform.set_quantization_bits(quantization_bits);
    
    let mut encoder = PredictionSchemeGeometricNormalEncoder::new(transform);
    encoder.init(&mesh_data);
    
    // Set parent attribute (Position)
    let pos_att_ref = mesh.attribute(pos_att_id);
    encoder.set_parent_attribute(pos_att_ref);
    
    let mut out_corr = vec![0i32; 8];
    let entry_to_point_id_map = vec![0, 1, 2, 3]; // Identity
    
    let res = encoder.compute_correction_values(
        &in_data,
        &mut out_corr,
        4,
        2,
        Some(&entry_to_point_id_map)
    );
    assert!(res, "Compute correction values failed");
    
    let mut encoded_data = Vec::new();
    let res = encoder.encode_prediction_data(&mut encoded_data);
    assert!(res, "Encode prediction data failed");
    
    // 6. Decode
    let dec_transform = PredictionSchemeGeometricNormalDecodingTransform::new();
    // Quantization bits are read from buffer in decoder
    
    let mut decoder = PredictionSchemeGeometricNormalDecoder::new(dec_transform);
    decoder.init(&mesh_data);
    decoder.set_parent_attribute(pos_att_ref);
    
    let mut decoder_buffer = DecoderBuffer::new(&encoded_data);
    // Set version to 2.2+ so decoder doesn't expect a mode byte
    // (the encoder writes v2.2+ format without mode byte)
    decoder_buffer.set_version(2, 2);
    let res = decoder.decode_prediction_data(&mut decoder_buffer);
    assert!(res, "Decode prediction data failed");
    
    let mut out_original = vec![0i32; 8];
    let res = decoder.compute_original_values(
        &out_corr,
        &mut out_original,
        4,
        2,
        Some(&entry_to_point_id_map)
    );
    assert!(res, "Compute original values failed");
    
    // 7. Verify
    for i in 0..8 {
        assert_eq!(in_data[i], out_original[i], "Mismatch at index {}", i);
    }
}
