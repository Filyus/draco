use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use draco_core::compression_config::EncodedGeometryType;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::encoder_options::EncoderOptions;
use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
use draco_core::geometry_indices::{FaceIndex, PointIndex};
use draco_core::mesh::Mesh;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::point_cloud::PointCloud;
use draco_core::point_cloud_decoder::PointCloudDecoder;
use draco_core::draco_types::DataType;
use draco_core::geometry_indices::INVALID_ATTRIBUTE_VALUE_INDEX;
use draco_io::obj_reader;
use draco_io::ply_reader;

fn repo_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_header(bytes: &[u8]) -> (u8, u8, EncodedGeometryType, u8) {
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

fn decode_drc(bytes: &[u8]) {
    let (_major, _minor, geometry_type, _method) = parse_header(bytes);
    match geometry_type {
        EncodedGeometryType::TriangularMesh => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            let status = decoder.decode(&mut buffer, &mut mesh);
            assert!(status.is_ok(), "mesh decode failed: {:?}", status.err());
            assert!(mesh.num_points() > 0);
        }
        EncodedGeometryType::PointCloud => {
            let mut buffer = DecoderBuffer::new(bytes);
            let mut pc = PointCloud::new();
            let mut decoder = PointCloudDecoder::new();
            let status = decoder.decode(&mut buffer, &mut pc);
            assert!(status.is_ok(), "point cloud decode failed: {:?}", status.err());
            assert!(pc.num_points() > 0);
        }
        _ => unreachable!(),
    }
}

fn parse_obj_positions(path: &Path) -> Vec<[f32; 3]> {
    obj_reader::read_obj_positions(path).expect("failed to read OBJ positions")
}

fn extract_mesh_positions(mesh: &Mesh) -> Vec<[f32; 3]> {
    let pos_id = mesh.named_attribute_id(GeometryAttributeType::Position);
    assert!(pos_id >= 0, "mesh has no POSITION attribute");
    let att = mesh.attribute(pos_id);
    assert_eq!(att.data_type(), DataType::Float32, "expected float32 positions");
    assert_eq!(att.num_components(), 3, "expected 3-component positions");

    let stride = att.byte_stride() as usize;
    assert!(stride >= 12, "unexpected stride={stride}");
    let buf = att.buffer().data();

    let mut out = Vec::with_capacity(mesh.num_points());
    for p in 0..mesh.num_points() {
        let entry = att.mapped_index(PointIndex(p as u32));
        assert!(
            entry != INVALID_ATTRIBUTE_VALUE_INDEX,
            "invalid mapped index for point {p}"
        );
        let base = (entry.0 as usize) * stride;
        assert!(base + 12 <= buf.len(), "position read OOB");
        let x = f32::from_le_bytes(buf[base..base + 4].try_into().unwrap());
        let y = f32::from_le_bytes(buf[base + 4..base + 8].try_into().unwrap());
        let z = f32::from_le_bytes(buf[base + 8..base + 12].try_into().unwrap());
        out.push([x, y, z]);
    }
    out
}

fn extract_point_cloud_positions(pc: &PointCloud) -> Vec<[f32; 3]> {
    let pos_id = pc.named_attribute_id(GeometryAttributeType::Position);
    if pos_id < 0 {
        let mut desc = String::new();
        desc.push_str(&format!("point cloud has no POSITION attribute; num_attributes={}\n", pc.num_attributes()));
        for i in 0..pc.num_attributes() {
            let att = pc.attribute(i);
            desc.push_str(&format!(
                "  att[{i}]: type={:?} comps={} dtype={:?} unique_id={} stride={}\n",
                att.attribute_type(),
                att.num_components(),
                att.data_type(),
                att.unique_id(),
                att.byte_stride(),
            ));
        }
        panic!("{desc}");
    }
    let att = pc.attribute(pos_id);
    assert_eq!(att.data_type(), DataType::Float32, "expected float32 positions");
    assert_eq!(att.num_components(), 3, "expected 3-component positions");

    let stride = att.byte_stride() as usize;
    assert!(stride >= 12, "unexpected stride={stride}");
    let buf = att.buffer().data();

    let mut out = Vec::with_capacity(pc.num_points());
    for p in 0..pc.num_points() {
        let entry = att.mapped_index(PointIndex(p as u32));
        assert!(
            entry != INVALID_ATTRIBUTE_VALUE_INDEX,
            "invalid mapped index for point {p}"
        );
        let base = (entry.0 as usize) * stride;
        assert!(base + 12 <= buf.len(), "position read OOB");
        let x = f32::from_le_bytes(buf[base..base + 4].try_into().unwrap());
        let y = f32::from_le_bytes(buf[base + 4..base + 8].try_into().unwrap());
        let z = f32::from_le_bytes(buf[base + 8..base + 12].try_into().unwrap());
        out.push([x, y, z]);
    }
    out
}

fn assert_positions_close(a: &[[f32; 3]], b: &[[f32; 3]], tol: f32) {
    assert_eq!(a.len(), b.len(), "vertex count mismatch");

    let mut a_sorted: Vec<[f32; 3]> = a.to_vec();
    let mut b_sorted: Vec<[f32; 3]> = b.to_vec();
    let cmp = |lhs: &[f32; 3], rhs: &[f32; 3]| {
        lhs[0]
            .total_cmp(&rhs[0])
            .then_with(|| lhs[1].total_cmp(&rhs[1]))
            .then_with(|| lhs[2].total_cmp(&rhs[2]))
    };
    a_sorted.sort_by(cmp);
    b_sorted.sort_by(cmp);

    for (i, (pa, pb)) in a_sorted.iter().zip(b_sorted.iter()).enumerate() {
        let dx = (pa[0] - pb[0]).abs();
        let dy = (pa[1] - pb[1]).abs();
        let dz = (pa[2] - pb[2]).abs();
        assert!(
            dx <= tol && dy <= tol && dz <= tol,
            "vertex[{i}] mismatch: rust={pa:?} cpp={pb:?} (abs diff={dx},{dy},{dz}, tol={tol})"
        );
    }
}

fn cpp_tools() -> Option<(PathBuf, PathBuf)> {
    // Allow override.
    if let Ok(dir) = std::env::var("DRACO_CPP_BUILD_DIR") {
        let build_dir = PathBuf::from(dir);
        let enc = build_dir.join("draco_encoder.exe");
        let dec = build_dir.join("draco_decoder.exe");
        if enc.is_file() && dec.is_file() {
            return Some((enc, dec));
        }
    }

    let repo = repo_root_dir();
    let candidates = [
        repo.join("build/Debug"),
        repo.join("build/Release"),
        repo.join("build-original/Debug"),
        repo.join("build-original/Release"),
        repo.join("build/x64/Debug"),
        repo.join("build/x64/Release"),
    ];

    for c in candidates {
        let enc = c.join("draco_encoder.exe");
        let dec = c.join("draco_decoder.exe");
        if enc.is_file() && dec.is_file() {
            return Some((enc, dec));
        }
    }

    None
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut base = std::env::temp_dir();
    let pid = std::process::id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    base.push(format!("draco_rust_cpp_{prefix}_{pid}_{now}"));
    fs::create_dir_all(&base).expect("failed to create temp dir");
    base
}

fn write_simple_triangle_obj(path: &Path) {
    // Simple triangle in XY plane.
    // OBJ is 1-indexed for faces.
    let obj = "v 0 0 0\n\
               v 1 0 0\n\
               v 0 1 0\n\
               f 1 2 3\n";
    fs::write(path, obj).expect("failed to write obj");
}

fn write_point_cloud_ply(path: &Path, points: &[[f32; 3]]) {
    ply_reader::write_ply_positions(path, points).expect("failed to write PLY");
}

fn cpp_encode_point_cloud_ply(
    cpp_encoder: &Path,
    ply_in_path: &Path,
    drc_out_path: &Path,
    compression_level: u8,
    qp: u8,
) {
    let out = Command::new(cpp_encoder)
        .args([
            "-i",
            ply_in_path.to_string_lossy().as_ref(),
            "-o",
            drc_out_path.to_string_lossy().as_ref(),
            "-point_cloud",
            "-cl",
            &compression_level.to_string(),
            "-qp",
            &qp.to_string(),
        ])
        .output()
        .expect("failed to run draco_encoder");
    assert!(
        out.status.success(),
        "draco_encoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn rust_decode_point_cloud_positions_from_drc(drc_path: &Path) -> Vec<[f32; 3]> {
    let bytes = fs::read(drc_path).expect("failed to read drc");
    let (_major, _minor, geometry_type, _method) = parse_header(&bytes);
    assert_eq!(geometry_type, EncodedGeometryType::PointCloud);

    let mut buffer = DecoderBuffer::new(&bytes);
    let mut pc = PointCloud::new();
    let mut decoder = PointCloudDecoder::new();
    let status = decoder.decode(&mut buffer, &mut pc);
    assert!(
        status.is_ok(),
        "Rust point cloud decode failed: {:?}",
        status.err()
    );

    extract_point_cloud_positions(&pc)
}

fn make_line_points_step_quarter(num_points: usize) -> Vec<[f32; 3]> {
    assert!(num_points >= 2);
    // Deterministic line from -1.0 to 1.0 with step = 0.25, all exactly
    // representable in binary f32.
    // For example, 9 points gives: -1.0, -0.75, ..., 1.0.
    let span = (num_points - 1) as f32;
    let step = 2.0 / span;
    // Ensure the chosen num_points yields a dyadic step.
    // (This is just to catch accidental use of non-dyadic counts.)
    assert!(
        (step * 4.0).fract() == 0.0,
        "num_points={num_points} does not yield a 0.25-multiple step"
    );
    (0..num_points)
        .map(|i| {
            let t = i as f32;
            let x = -1.0 + t * step;
            [x, 0.0, 0.0]
        })
        .collect()
}

fn make_grid_points_3x3x3() -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(27);
    for x in [-1.0_f32, 0.0, 1.0] {
        for y in [-1.0_f32, 0.0, 1.0] {
            for z in [-1.0_f32, 0.0, 1.0] {
                out.push([x, y, z]);
            }
        }
    }
    out
}

fn make_rust_triangle_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    mesh.set_num_points(3);
    mesh.set_num_faces(1);
    mesh.set_face(
        FaceIndex(0),
        [PointIndex(0), PointIndex(1), PointIndex(2)],
    );

    // Position attribute: 3 components float32.
    let mut pos_att = PointAttribute::new();
    pos_att.init(GeometryAttributeType::Position, 3, DataType::Float32, false, 3);

    let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let dst = pos_att.buffer_mut().data_mut();
    assert_eq!(dst.len(), positions.len() * std::mem::size_of::<f32>());
    for (i, v) in positions.iter().enumerate() {
        let bytes = v.to_le_bytes();
        let off = i * 4;
        dst[off..off + 4].copy_from_slice(&bytes);
    }

    mesh.add_attribute(pos_att);

    mesh
}

#[test]
fn cpp_encode_then_rust_decode() {
    let Some((cpp_encoder, _cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("cpp_encode_then_rust_decode");
    let obj_path = tmp.join("tri.obj");
    let drc_path = tmp.join("tri.drc");

    write_simple_triangle_obj(&obj_path);

    let out = Command::new(&cpp_encoder)
        .args(["-i", obj_path.to_string_lossy().as_ref(), "-o", drc_path.to_string_lossy().as_ref(), "-cl", "7", "-qp", "11"])
        .output()
        .expect("failed to run draco_encoder");

    assert!(
        out.status.success(),
        "draco_encoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let bytes = fs::read(&drc_path).expect("failed to read drc");
    decode_drc(&bytes);
}

#[test]
fn cpp_and_rust_decode_positions_match() {
    let Some((cpp_encoder, cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("cpp_and_rust_decode_positions_match");
    let obj_in_path = tmp.join("tri.obj");
    let drc_path = tmp.join("tri.drc");
    let obj_out_path = tmp.join("tri_decoded.obj");

    write_simple_triangle_obj(&obj_in_path);

    // C++ encode.
    let out = Command::new(&cpp_encoder)
        .args([
            "-i",
            obj_in_path.to_string_lossy().as_ref(),
            "-o",
            drc_path.to_string_lossy().as_ref(),
            "-cl",
            "7",
            "-qp",
            "11",
        ])
        .output()
        .expect("failed to run draco_encoder");
    assert!(
        out.status.success(),
        "draco_encoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // C++ decode to OBJ.
    let out = Command::new(&cpp_decoder)
        .args([
            "-i",
            drc_path.to_string_lossy().as_ref(),
            "-o",
            obj_out_path.to_string_lossy().as_ref(),
        ])
        .output()
        .expect("failed to run draco_decoder");
    assert!(
        out.status.success(),
        "draco_decoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let cpp_positions = parse_obj_positions(&obj_out_path);
    assert!(!cpp_positions.is_empty(), "C++ produced no OBJ vertices");

    // Rust decode the same .drc and extract positions.
    let bytes = fs::read(&drc_path).expect("failed to read drc");
    let (_major, _minor, geometry_type, _method) = parse_header(&bytes);
    assert_eq!(geometry_type, EncodedGeometryType::TriangularMesh);

    let mut buffer = DecoderBuffer::new(&bytes);
    let mut mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut mesh);
    assert!(status.is_ok(), "Rust mesh decode failed: {:?}", status.err());

    let rust_positions = extract_mesh_positions(&mesh);
    assert_positions_close(&rust_positions, &cpp_positions, 1e-5);
}

#[test]
fn rust_encode_then_cpp_decode() {
    let Some((_cpp_encoder, cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("rust_encode_then_cpp_decode");
    let drc_path = tmp.join("tri_rust.drc");
    let out_path = tmp.join("tri_out.obj");

    let mesh = make_rust_triangle_mesh();
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);

    let options = EncoderOptions::new();
    let mut enc = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc);
    assert!(status.is_ok(), "Rust MeshEncoder failed: {:?}", status.err());

    fs::write(&drc_path, enc.data()).expect("failed to write drc");

    let out = Command::new(&cpp_decoder)
        .args(["-i", drc_path.to_string_lossy().as_ref(), "-o", out_path.to_string_lossy().as_ref()])
        .output()
        .expect("failed to run draco_decoder");

    assert!(
        out.status.success(),
        "draco_decoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let out_bytes = fs::read(&out_path).expect("failed to read decoder output");
    assert!(!out_bytes.is_empty(), "C++ draco_decoder produced empty output");
}

#[test]
fn cpp_encode_rust_decode_rust_encode_cpp_decode_chain() {
    let Some((cpp_encoder, cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("cpp_rust_cpp_chain");
    let obj_path = tmp.join("tri.obj");
    let drc_cpp_path = tmp.join("tri_cpp.drc");
    let drc_rust_path = tmp.join("tri_rust.drc");
    let out_obj_path = tmp.join("tri_out.obj");

    write_simple_triangle_obj(&obj_path);

    // C++ encode.
    let out = Command::new(&cpp_encoder)
        .args(["-i", obj_path.to_string_lossy().as_ref(), "-o", drc_cpp_path.to_string_lossy().as_ref(), "-cl", "7", "-qp", "11"])
        .output()
        .expect("failed to run draco_encoder");
    assert!(
        out.status.success(),
        "draco_encoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Rust decode -> Mesh.
    let cpp_bytes = fs::read(&drc_cpp_path).expect("failed to read drc");
    let (_major, _minor, geometry_type, _method) = parse_header(&cpp_bytes);
    assert_eq!(geometry_type, EncodedGeometryType::TriangularMesh);

    let mut buffer = DecoderBuffer::new(&cpp_bytes);
    let mut mesh = Mesh::new();
    let mut decoder = MeshDecoder::new();
    let status = decoder.decode(&mut buffer, &mut mesh);
    assert!(status.is_ok(), "Rust MeshDecoder failed: {:?}", status.err());
    assert!(mesh.num_points() > 0);

    // Rust encode back.
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);
    let options = EncoderOptions::new();
    let mut enc = EncoderBuffer::new();
    let status = encoder.encode(&options, &mut enc);
    assert!(status.is_ok(), "Rust MeshEncoder failed: {:?}", status.err());
    fs::write(&drc_rust_path, enc.data()).expect("failed to write rust drc");

    // C++ decode Rust-produced drc.
    let out = Command::new(&cpp_decoder)
        .args(["-i", drc_rust_path.to_string_lossy().as_ref(), "-o", out_obj_path.to_string_lossy().as_ref()])
        .output()
        .expect("failed to run draco_decoder");

    assert!(
        out.status.success(),
        "draco_decoder failed: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let out_bytes = fs::read(&out_obj_path).expect("failed to read decoder output");
    assert!(!out_bytes.is_empty(), "C++ draco_decoder produced empty output");
}

#[test]
fn cpp_encode_point_cloud_then_rust_decode_positions_match_ground_truth() {
    let Some((cpp_encoder, _cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("cpp_encode_point_cloud_ground_truth");
    let ply_in_path = tmp.join("pc.ply");
    let drc_path = tmp.join("pc.drc");

    // Deterministic coordinates with exact binary representations where possible.
    // (Avoids false negatives from decimal parsing/printing quirks.)
    let expected: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [0.125, 1.25, 0.5],
        [-0.5, 0.25, -0.125],
    ];

    write_point_cloud_ply(&ply_in_path, &expected);
    cpp_encode_point_cloud_ply(&cpp_encoder, &ply_in_path, &drc_path, 10, 0);

    let rust_positions = rust_decode_point_cloud_positions_from_drc(&drc_path);
    assert_positions_close(&rust_positions, &expected, 0.0);
}

#[test]
fn cpp_encode_point_cloud_line_positions_match_ground_truth() {
    let Some((cpp_encoder, _cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("cpp_encode_point_cloud_line_ground_truth");
    let ply_in_path = tmp.join("pc_line.ply");
    let drc_path = tmp.join("pc_line.drc");

    let expected = make_line_points_step_quarter(9);
    write_point_cloud_ply(&ply_in_path, &expected);
    cpp_encode_point_cloud_ply(&cpp_encoder, &ply_in_path, &drc_path, 10, 0);

    let rust_positions = rust_decode_point_cloud_positions_from_drc(&drc_path);
    assert_positions_close(&rust_positions, &expected, 0.0);
}

#[test]
fn cpp_encode_point_cloud_grid_3x3x3_positions_match_ground_truth() {
    let Some((cpp_encoder, _cpp_decoder)) = cpp_tools() else {
        eprintln!("Skipping: C++ draco_encoder/draco_decoder not found. Set DRACO_CPP_BUILD_DIR or build the C++ tools under build/Debug.");
        return;
    };

    let tmp = create_temp_dir("cpp_encode_point_cloud_grid_ground_truth");
    let ply_in_path = tmp.join("pc_grid_3x3x3.ply");
    let drc_path = tmp.join("pc_grid_3x3x3.drc");

    let expected = make_grid_points_3x3x3();
    write_point_cloud_ply(&ply_in_path, &expected);
    cpp_encode_point_cloud_ply(&cpp_encoder, &ply_in_path, &drc_path, 10, 0);

    let rust_positions = rust_decode_point_cloud_positions_from_drc(&drc_path);
    assert_positions_close(&rust_positions, &expected, 0.0);
}
