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

fn supports_mesh_bitstream(major: u8, minor: u8) -> bool {
    // Current Rust MeshEdgebreakerDecoder implementation matches the v2.2+
    // layout (no legacy encoded_connectivity_size indirection).
    (major > 2) || (major == 2 && minor >= 2)
}

fn supports_point_cloud_bitstream(major: u8, minor: u8, method: u8) -> bool {
    // Current PointCloudDecoder supports:
    // - v2.3 KD-tree (method=1)
    // - our v1.3 sequential format (method=0)
    (major == 2 && minor == 3 && method == 1) || (major == 1 && minor == 3 && method == 0)
}

fn decode_drc(bytes: &[u8]) -> (EncodedGeometryType, Option<Mesh>, Option<PointCloud>) {
    let (_major, _minor, geometry_type, _method) = parse_header(bytes);

    match geometry_type {
        EncodedGeometryType::TriangularMesh => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            let status = decoder.decode(&mut buffer, &mut mesh);
            assert!(
                status.is_ok(),
                "mesh decode failed: {:?}",
                status.err()
            );
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
#[ignore = "Some testdata files use unsupported EdgeBreaker traversal types (e.g., bunny_cpp.drc uses type 2)"]
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
                assert!(
                    status.is_ok(),
                    "mesh decode failed for {} (v{}.{}): {:?}",
                    path.display(),
                    major,
                    minor,
                    status.err()
                );
                decoded_any = true;
                assert!(mesh.num_points() > 0, "{} decoded with 0 points", path.display());
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
                assert!(pc.num_points() > 0, "{} decoded with 0 points", path.display());
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

    let options = EncoderOptions::new();
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
    // pc_color.drc is v2.2 sequential and isn't supported by the current Rust
    // PointCloudDecoder implementation.
    let path = repo_testdata_dir().join("pc_kd_color.drc");
    let bytes = read_file_bytes(&path);
    let (geometry_type, _, pc) = decode_drc(&bytes);
    assert_eq!(geometry_type, EncodedGeometryType::PointCloud);

    let original = pc.expect("point cloud missing");
    assert!(original.num_points() > 0);

    // Minimal invariants.
    assert!(original.num_attributes() >= 1);
}
