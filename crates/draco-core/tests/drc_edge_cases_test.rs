use std::path::PathBuf;

use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::encoder_options::EncoderOptions;
use draco_core::mesh::Mesh;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::point_cloud::PointCloud;
use draco_core::point_cloud_decoder::PointCloudDecoder;
use draco_core::point_cloud_encoder::PointCloudEncoder;

fn repo_testdata_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata")
}

#[test]
fn decode_rejects_invalid_magic() {
    let mut bytes = vec![0u8; 32];
    bytes[0..5].copy_from_slice(b"XXXXX");

    let mut buffer = DecoderBuffer::new(&bytes);
    let mut mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut mesh);

    assert!(status.is_err());
}

#[test]
fn decode_rejects_invalid_geometry_type_in_header() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"DRACO");
    bytes.push(2); // major
    bytes.push(2); // minor
    bytes.push(99); // invalid geometry type
    bytes.push(0); // method
    bytes.extend_from_slice(&0u16.to_le_bytes()); // flags

    let mut buffer = DecoderBuffer::new(&bytes);
    let mut mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut mesh);

    assert!(status.is_err());
}

#[test]
fn decode_rejects_truncated_file() {
    let path = repo_testdata_dir().join("cube_att.drc");
    let bytes = std::fs::read(&path).expect("failed to read cube_att.drc");
    assert!(bytes.len() > 16, "unexpectedly small cube_att.drc");

    // Truncate the tail; should fail gracefully (no panic).
    let truncated = &bytes[0..bytes.len() - 7];

    // Use header byte to select decoder (this file is a mesh).
    let mut buffer = DecoderBuffer::new(truncated);
    let mut mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut mesh);

    assert!(status.is_err());
}

#[test]
fn encode_decode_empty_mesh() {
    let mesh = Mesh::new();

    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);

    let options = EncoderOptions::new();
    let mut enc = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc);
    assert!(status.is_ok(), "empty mesh encode failed: {:?}", status.err());

    let mut buffer = DecoderBuffer::new(enc.data());
    let mut decoded = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut decoded);
    assert!(status.is_ok(), "empty mesh decode failed: {:?}", status.err());

    assert_eq!(decoded.num_faces(), 0);
    assert_eq!(decoded.num_points(), 0);
    assert_eq!(decoded.num_attributes(), 0);
}

#[test]
fn encode_decode_empty_point_cloud() {
    let pc = PointCloud::new();

    let mut encoder = PointCloudEncoder::new();
    encoder.set_point_cloud(pc);

    let options = EncoderOptions::new();
    let mut enc = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc);
    assert!(status.is_ok(), "empty point cloud encode failed: {:?}", status.err());

    let mut buffer = DecoderBuffer::new(enc.data());
    let mut decoded = PointCloud::new();
    let mut decoder = PointCloudDecoder::new();
    let status = decoder.decode(&mut buffer, &mut decoded);
    assert!(status.is_ok(), "empty point cloud decode failed: {:?}", status.err());

    assert_eq!(decoded.num_points(), 0);
    assert_eq!(decoded.num_attributes(), 0);
}
