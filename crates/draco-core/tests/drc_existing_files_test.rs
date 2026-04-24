use std::fs;
use std::path::{Path, PathBuf};

use draco_core::compression_config::EncodedGeometryType;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::encoder_options::EncoderOptions;
use draco_core::mesh::Mesh;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::point_cloud::PointCloud;
use draco_core::point_cloud_decoder::PointCloudDecoder;
use draco_core::status::DracoError;

fn repo_testdata_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/draco-core
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata")
}

fn collect_drc_files_recursive(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("drc"))
            {
                out.push(path);
            }
        }
    }

    out
}

fn read_file_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn parse_header(bytes: &[u8]) -> (u8, u8, EncodedGeometryType, u8) {
    // Draco header (common):
    // 0..5: "DRACO", 5: major, 6: minor, 7: geometry_type, 8: encoding method
    assert!(bytes.len() >= 9, "file too small for drc header");
    assert_eq!(&bytes[0..5], b"DRACO", "invalid magic");
    let major = bytes[5];
    let minor = bytes[6];
    let geometry_type = match bytes[7] {
        0 => EncodedGeometryType::PointCloud,
        1 => EncodedGeometryType::TriangularMesh,
        other => panic!("unexpected geometry type in header: {other}"),
    };
    let method = bytes[8];
    (major, minor, geometry_type, method)
}

fn supports_mesh_bitstream(major: u8, _minor: u8) -> bool {
    // Rust MeshDecoder supports the modern v2.2+ layout and the v2.0/v2.1
    // legacy mesh layout used by Draco 1.0.0/1.1.0 test fixtures.
    major >= 2
}

fn supports_point_cloud_bitstream(major: u8, minor: u8, method: u8) -> bool {
    // Current PointCloudDecoder supports:
    // - v2.2+ sequential (method=0)
    // - v2.3 KD-tree (method=1)
    // - our v1.3 sequential format (method=0)
    (major == 2 && minor >= 2 && method == 0)
        || (major == 2 && minor == 3 && method == 1)
        || (major == 1 && minor == 3 && method == 0)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GeometryKind {
    Mesh,
    PointCloud,
}

impl From<EncodedGeometryType> for GeometryKind {
    fn from(value: EncodedGeometryType) -> Self {
        match value {
            EncodedGeometryType::TriangularMesh => Self::Mesh,
            EncodedGeometryType::PointCloud => Self::PointCloud,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SkipReason {
    UnsupportedBitstream,
    UnsupportedTraversal,
}

#[derive(Debug, Eq, PartialEq)]
struct SkippedFixture {
    path: String,
    major: u8,
    minor: u8,
    geometry: GeometryKind,
    method: u8,
    reason: SkipReason,
}

fn skipped(
    path: &str,
    major: u8,
    minor: u8,
    geometry: GeometryKind,
    method: u8,
    reason: SkipReason,
) -> SkippedFixture {
    SkippedFixture {
        path: path.to_string(),
        major,
        minor,
        geometry,
        method,
        reason,
    }
}

fn relative_testdata_path(path: &Path) -> String {
    path.strip_prefix(repo_testdata_dir())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn skipped_fixture_for_current_decoder(path: &Path, bytes: &[u8]) -> Option<SkippedFixture> {
    let (major, minor, geometry_type, method) = parse_header(bytes);
    let path = relative_testdata_path(path);
    let geometry = GeometryKind::from(geometry_type);

    match geometry_type {
        EncodedGeometryType::TriangularMesh => {
            if !supports_mesh_bitstream(major, minor) {
                return Some(skipped(
                    &path,
                    major,
                    minor,
                    geometry,
                    method,
                    SkipReason::UnsupportedBitstream,
                ));
            }

            let mut buffer = DecoderBuffer::new(bytes);
            let mut mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            if let Err(DracoError::DracoError(msg)) = decoder.decode(&mut buffer, &mut mesh) {
                if msg.starts_with("Unsupported Edgebreaker traversal decoder type") {
                    return Some(skipped(
                        &path,
                        major,
                        minor,
                        geometry,
                        method,
                        SkipReason::UnsupportedTraversal,
                    ));
                }
            }
        }
        EncodedGeometryType::PointCloud => {
            if !supports_point_cloud_bitstream(major, minor, method) {
                return Some(skipped(
                    &path,
                    major,
                    minor,
                    geometry,
                    method,
                    SkipReason::UnsupportedBitstream,
                ));
            }
        }
        _ => unreachable!(),
    }

    None
}

fn decode_drc(bytes: &[u8]) -> (EncodedGeometryType, Option<Mesh>, Option<PointCloud>) {
    let (_major, _minor, geometry_type, _method) = parse_header(bytes);

    match geometry_type {
        EncodedGeometryType::TriangularMesh => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            let status = decoder.decode(&mut buffer, &mut mesh);
            assert!(status.is_ok(), "mesh decode failed: {:?}", status.err());
            (geometry_type, Some(mesh), None)
        }
        EncodedGeometryType::PointCloud => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut pc = PointCloud::new();
            let mut decoder = PointCloudDecoder::new();
            let status = decoder.decode(&mut buffer, &mut pc);
            assert!(
                status.is_ok(),
                "point cloud decode failed: {:?}",
                status.err()
            );
            (geometry_type, None, Some(pc))
        }
        _ => unreachable!(),
    }
}

#[test]
fn decode_legacy_mesh_v20_v21_from_testdata() {
    let fixtures = [
        "test_nm.obj.edgebreaker.1.0.0.drc",
        "test_nm.obj.edgebreaker.1.1.0.drc",
        "test_nm.obj.sequential.1.0.0.drc",
        "test_nm.obj.sequential.1.1.0.drc",
    ];

    for fixture in fixtures {
        let path = repo_testdata_dir().join(fixture);
        let bytes = read_file_bytes(&path);
        let (major, minor, geometry_type, _method) = parse_header(&bytes);

        assert_eq!(
            geometry_type,
            EncodedGeometryType::TriangularMesh,
            "{fixture} should be a mesh fixture"
        );
        assert!(
            major == 2 && (minor == 0 || minor == 1),
            "{fixture} should cover mesh bitstream v2.0 or v2.1, got v{major}.{minor}"
        );

        let mut buffer = DecoderBuffer::new(&bytes);
        let mut mesh = Mesh::new();
        let mut decoder = MeshDecoder::new();
        let status = decoder.decode(&mut buffer, &mut mesh);

        assert!(
            status.is_ok(),
            "legacy mesh decode failed for {fixture} (v{major}.{minor}): {:?}",
            status.err()
        );
        assert!(mesh.num_points() > 0, "{fixture} decoded with 0 points");
        assert!(mesh.num_faces() > 0, "{fixture} decoded with 0 faces");
        assert!(
            mesh.num_attributes() > 0,
            "{fixture} decoded with 0 attributes"
        );
    }
}

#[test]
fn decode_point_cloud_sequential_v22_v23_from_testdata() {
    let fixtures = ["pc_color.drc", "point_cloud_no_qp.drc"];

    for fixture in fixtures {
        let path = repo_testdata_dir().join(fixture);
        let bytes = read_file_bytes(&path);
        let (major, minor, geometry_type, method) = parse_header(&bytes);

        assert_eq!(
            geometry_type,
            EncodedGeometryType::PointCloud,
            "{fixture} should be a point-cloud fixture"
        );
        assert_eq!(
            method, 0,
            "{fixture} should cover sequential point-cloud method"
        );
        assert!(
            major == 2 && (minor == 2 || minor == 3),
            "{fixture} should cover point-cloud bitstream v2.2 or v2.3, got v{major}.{minor}"
        );

        let mut buffer = DecoderBuffer::new(&bytes);
        let mut pc = PointCloud::new();
        let mut decoder = PointCloudDecoder::new();
        let status = decoder.decode(&mut buffer, &mut pc);

        assert!(
            status.is_ok(),
            "point-cloud sequential decode failed for {fixture} (v{major}.{minor}): {:?}",
            status.err()
        );
        assert!(pc.num_points() > 0, "{fixture} decoded with 0 points");
        assert!(
            pc.num_attributes() > 0,
            "{fixture} decoded with 0 attributes"
        );
    }
}

#[test]
fn inventory_skipped_testdata_drc_fixtures() {
    let dir = repo_testdata_dir();
    let mut drc_files = collect_drc_files_recursive(&dir);
    drc_files.sort();
    assert!(!drc_files.is_empty(), "no .drc files found in testdata");

    let actual: Vec<_> = drc_files
        .iter()
        .filter_map(|path| {
            let bytes = read_file_bytes(path);
            skipped_fixture_for_current_decoder(path, &bytes)
        })
        .collect();

    let expected = vec![
        skipped(
            "cube_att.drc",
            1,
            1,
            GeometryKind::Mesh,
            1,
            SkipReason::UnsupportedBitstream,
        ),
        skipped(
            "cube_pc.drc",
            1,
            1,
            GeometryKind::PointCloud,
            0,
            SkipReason::UnsupportedBitstream,
        ),
        skipped(
            "test_nm.obj.edgebreaker.0.10.0.drc",
            1,
            2,
            GeometryKind::Mesh,
            1,
            SkipReason::UnsupportedBitstream,
        ),
        skipped(
            "test_nm.obj.edgebreaker.0.9.1.drc",
            1,
            1,
            GeometryKind::Mesh,
            1,
            SkipReason::UnsupportedBitstream,
        ),
        skipped(
            "test_nm.obj.sequential.0.10.0.drc",
            1,
            2,
            GeometryKind::Mesh,
            0,
            SkipReason::UnsupportedBitstream,
        ),
        skipped(
            "test_nm.obj.sequential.0.9.1.drc",
            1,
            1,
            GeometryKind::Mesh,
            0,
            SkipReason::UnsupportedBitstream,
        ),
        skipped(
            "test_nm_quant.0.9.0.drc",
            1,
            2,
            GeometryKind::Mesh,
            1,
            SkipReason::UnsupportedBitstream,
        ),
    ];

    assert_eq!(actual, expected);
}

#[test]
fn decode_all_testdata_top_level_drc_files() {
    let dir = repo_testdata_dir();
    let mut drc_files = collect_drc_files_recursive(&dir);

    drc_files.sort();
    assert!(!drc_files.is_empty(), "no .drc files found in testdata");

    let mut decoded_any = false;
    for path in drc_files {
        let bytes = read_file_bytes(&path);
        let (major, minor, geometry_type, method) = parse_header(&bytes);

        // Only decode files for bitstream variants we currently support.
        // This still exercises real shipped .drc assets without forcing us
        // to immediately implement all legacy layouts.
        match geometry_type {
            EncodedGeometryType::TriangularMesh => {
                if !supports_mesh_bitstream(major, minor) {
                    continue;
                }
                let mut buffer = DecoderBuffer::new(&bytes);
                let mut mesh = Mesh::new();
                let mut decoder = MeshDecoder::new();
                let status = decoder.decode(&mut buffer, &mut mesh);

                if let Err(DracoError::DracoError(ref msg)) = status {
                    if msg.starts_with("Unsupported Edgebreaker traversal decoder type") {
                        println!(
                            "Skipping {} due to unsupported traversal: {}",
                            path.display(),
                            msg
                        );
                        continue;
                    }
                }

                assert!(
                    status.is_ok(),
                    "mesh decode failed for {} (v{}.{}): {:?}",
                    path.display(),
                    major,
                    minor,
                    status.err()
                );
                decoded_any = true;
                assert!(
                    mesh.num_points() > 0,
                    "{} decoded with 0 points",
                    path.display()
                );
            }
            EncodedGeometryType::PointCloud => {
                if !supports_point_cloud_bitstream(major, minor, method) {
                    continue;
                }
                let mut buffer = DecoderBuffer::new(&bytes);
                let mut pc = PointCloud::new();
                let mut decoder = PointCloudDecoder::new();
                let status = decoder.decode(&mut buffer, &mut pc);
                assert!(
                    status.is_ok(),
                    "point cloud decode failed for {} (v{}.{} method={}): {:?}",
                    path.display(),
                    major,
                    minor,
                    method,
                    status.err()
                );
                decoded_any = true;
                assert!(
                    pc.num_points() > 0,
                    "{} decoded with 0 points",
                    path.display()
                );
            }
            _ => unreachable!(),
        }
    }

    assert!(
        decoded_any,
        "no supported .drc files were decoded; update supports_*() or add compatible fixtures"
    );
}

#[test]
fn roundtrip_encode_decode_mesh_from_testdata() {
    // Pick a v2.2 mesh that the current MeshDecoder supports.
    let path = repo_testdata_dir().join("test_nm.obj.edgebreaker.cl4.2.2.drc");
    let bytes = read_file_bytes(&path);
    let (geometry_type, mesh, _) = decode_drc(&bytes);
    assert_eq!(geometry_type, EncodedGeometryType::TriangularMesh);

    let original = mesh.expect("mesh missing");
    assert!(original.num_points() > 0);

    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(original.clone());

    // Use sequential encoding and quantization for reliable roundtrip
    let mut options = EncoderOptions::new();
    options.set_global_int("encoding_method", 0); // Sequential encoding
    for i in 0..original.num_attributes() {
        options.set_attribute_int(i, "quantization_bits", 14);
    }
    // Keep defaults; this is primarily an integration sanity check.
    let mut enc = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc);
    assert!(status.is_ok(), "re-encode failed: {:?}", status.err());

    let mut buffer = DecoderBuffer::new(enc.data());
    let mut decoded = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut decoded);
    assert!(status.is_ok(), "re-decode failed: {:?}", status.err());

    assert_eq!(decoded.num_faces(), original.num_faces());
    assert_eq!(decoded.num_points(), original.num_points());
    assert_eq!(decoded.num_attributes(), original.num_attributes());
}

#[test]
fn decode_point_cloud_kdtree_from_testdata() {
    let path = repo_testdata_dir().join("pc_kd_color.drc");
    let bytes = read_file_bytes(&path);
    let (geometry_type, _, pc) = decode_drc(&bytes);
    assert_eq!(geometry_type, EncodedGeometryType::PointCloud);

    let original = pc.expect("point cloud missing");
    assert!(original.num_points() > 0);

    // Minimal invariants.
    assert!(original.num_attributes() >= 1);
}
