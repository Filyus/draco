use std::panic::{self, AssertUnwindSafe};
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

#[derive(Clone, Copy)]
enum DecoderKind {
    Mesh,
    PointCloud,
}

fn draco_header(major: u8, minor: u8, geometry: u8, method: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"DRACO");
    bytes.push(major);
    bytes.push(minor);
    bytes.push(geometry);
    bytes.push(method);
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

fn decode_malformed_without_panic(kind: DecoderKind, bytes: &[u8]) -> Result<(), String> {
    let status = panic::catch_unwind(AssertUnwindSafe(|| match kind {
        DecoderKind::Mesh => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            decoder.decode(&mut buffer, &mut mesh)
        }
        DecoderKind::PointCloud => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut pc = PointCloud::new();
            let mut decoder = PointCloudDecoder::new();
            decoder.decode(&mut buffer, &mut pc)
        }
    }))
    .map_err(|_| "decoder panicked".to_string())?;

    status.map_err(|e| format!("{e:?}"))
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
fn malformed_drc_inputs_fail_without_panic() {
    let mut truncated_mesh_payload = draco_header(2, 0, 1, 0);
    truncated_mesh_payload.extend_from_slice(&8u32.to_le_bytes());

    let mut truncated_point_cloud_payload = draco_header(2, 0, 0, 0);
    truncated_point_cloud_payload.extend_from_slice(&4u32.to_le_bytes());

    let mut corrupt_point_cloud_varint = draco_header(2, 2, 0, 0);
    corrupt_point_cloud_varint.extend_from_slice(&1u32.to_le_bytes());
    corrupt_point_cloud_varint.push(1); // one attributes decoder
    corrupt_point_cloud_varint.extend_from_slice(&[0x80; 10]);

    let mut truncated_point_cloud_attribute_metadata = draco_header(2, 2, 0, 0);
    truncated_point_cloud_attribute_metadata.extend_from_slice(&1u32.to_le_bytes());
    truncated_point_cloud_attribute_metadata.push(1); // one attributes decoder
    truncated_point_cloud_attribute_metadata.push(1); // one attribute in decoder

    let cases = [
        ("empty mesh stream", DecoderKind::Mesh, Vec::new()),
        ("short mesh header", DecoderKind::Mesh, b"DRAC".to_vec()),
        ("invalid mesh magic", DecoderKind::Mesh, vec![0u8; 16]),
        (
            "invalid mesh geometry type",
            DecoderKind::Mesh,
            draco_header(2, 2, 99, 0),
        ),
        (
            "truncated mesh payload",
            DecoderKind::Mesh,
            truncated_mesh_payload,
        ),
        (
            "empty point-cloud stream",
            DecoderKind::PointCloud,
            Vec::new(),
        ),
        (
            "short point-cloud header",
            DecoderKind::PointCloud,
            b"DRAC".to_vec(),
        ),
        (
            "truncated point-cloud payload",
            DecoderKind::PointCloud,
            truncated_point_cloud_payload,
        ),
        (
            "corrupt point-cloud varint",
            DecoderKind::PointCloud,
            corrupt_point_cloud_varint,
        ),
        (
            "truncated point-cloud attribute metadata",
            DecoderKind::PointCloud,
            truncated_point_cloud_attribute_metadata,
        ),
    ];

    for (name, kind, bytes) in cases {
        assert!(
            decode_malformed_without_panic(kind, &bytes).is_err(),
            "{name} unexpectedly decoded successfully"
        );
    }
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
// #[ignore = "Empty mesh encoding/decoding is an edge case - decoder expects at least one attribute"]
fn encode_decode_empty_mesh() {
    let mesh = Mesh::new();

    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);

    let options = EncoderOptions::new();
    let mut enc = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc);
    assert!(
        status.is_ok(),
        "empty mesh encode failed: {:?}",
        status.err()
    );

    let mut buffer = DecoderBuffer::new(enc.data());
    let mut decoded = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut decoded);
    assert!(
        status.is_ok(),
        "empty mesh decode failed: {:?}",
        status.err()
    );

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
    assert!(
        status.is_ok(),
        "empty point cloud encode failed: {:?}",
        status.err()
    );

    let mut buffer = DecoderBuffer::new(enc.data());
    let mut decoded = PointCloud::new();
    let mut decoder = PointCloudDecoder::new();
    let status = decoder.decode(&mut buffer, &mut decoded);
    assert!(
        status.is_ok(),
        "empty point cloud decode failed: {:?}",
        status.err()
    );

    assert_eq!(decoded.num_points(), 0);
    assert_eq!(decoded.num_attributes(), 0);
}
