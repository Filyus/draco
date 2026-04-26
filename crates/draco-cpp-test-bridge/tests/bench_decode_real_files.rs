/// Decode performance comparison using C++-encoded .drc files.
///
/// This is the fairest possible benchmark: both decoders receive exactly the
/// same bytes produced by the C++ reference encoder, so there is no question
/// about encoder bias.
mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::mesh::Mesh;
use draco_core::mesh_decoder::MeshDecoder;
use draco_cpp_test_bridge;

fn create_uv_sphere_data(lat_segments: usize, lon_segments: usize) -> (Vec<f32>, Vec<u32>) {
    assert!(lat_segments >= 2);
    assert!(lon_segments >= 3);

    let mut positions = Vec::with_capacity((1 + (lat_segments - 1) * lon_segments + 1) * 3);
    positions.extend_from_slice(&[0.0, 1.0, 0.0]);

    for lat in 1..lat_segments {
        let theta = std::f32::consts::PI * lat as f32 / lat_segments as f32;
        let y = theta.cos();
        let radius = theta.sin();
        for lon in 0..lon_segments {
            let phi = 2.0 * std::f32::consts::PI * lon as f32 / lon_segments as f32;
            positions.push(radius * phi.cos());
            positions.push(y);
            positions.push(radius * phi.sin());
        }
    }

    let bottom = (positions.len() / 3) as u32;
    positions.extend_from_slice(&[0.0, -1.0, 0.0]);

    let ring = |lat_ring: usize, lon: usize| -> u32 {
        1 + ((lat_ring - 1) * lon_segments + lon % lon_segments) as u32
    };

    let mut faces =
        Vec::with_capacity((lon_segments * 2 + (lat_segments - 2) * lon_segments * 2) * 3);

    for lon in 0..lon_segments {
        faces.extend_from_slice(&[0, ring(1, lon + 1), ring(1, lon)]);
    }

    for lat in 1..lat_segments - 1 {
        for lon in 0..lon_segments {
            let a = ring(lat, lon);
            let b = ring(lat, lon + 1);
            let c = ring(lat + 1, lon);
            let d = ring(lat + 1, lon + 1);
            faces.extend_from_slice(&[a, b, c]);
            faces.extend_from_slice(&[b, d, c]);
        }
    }

    for lon in 0..lon_segments {
        faces.extend_from_slice(&[
            ring(lat_segments - 1, lon),
            ring(lat_segments - 1, lon + 1),
            bottom,
        ]);
    }

    (positions, faces)
}

fn create_subdivided_cube_data(subdivisions: usize) -> (Vec<f32>, Vec<u32>) {
    assert!(subdivisions >= 1);

    let mut positions = Vec::with_capacity(6 * (subdivisions + 1) * (subdivisions + 1) * 3);
    let mut faces = Vec::with_capacity(6 * subdivisions * subdivisions * 2 * 3);

    let mut add_face = |axis: usize, sign: f32| {
        let base = (positions.len() / 3) as u32;
        for v in 0..=subdivisions {
            for u in 0..=subdivisions {
                let a = -1.0 + 2.0 * u as f32 / subdivisions as f32;
                let b = -1.0 + 2.0 * v as f32 / subdivisions as f32;
                let p = match axis {
                    0 => [sign, b, if sign > 0.0 { -a } else { a }],
                    1 => [a, sign, if sign > 0.0 { b } else { -b }],
                    _ => [if sign > 0.0 { a } else { -a }, b, sign],
                };
                positions.extend_from_slice(&p);
            }
        }

        let row = subdivisions + 1;
        for v in 0..subdivisions {
            for u in 0..subdivisions {
                let p0 = base + (v * row + u) as u32;
                let p1 = base + (v * row + u + 1) as u32;
                let p2 = base + ((v + 1) * row + u) as u32;
                let p3 = base + ((v + 1) * row + u + 1) as u32;
                faces.extend_from_slice(&[p0, p1, p2]);
                faces.extend_from_slice(&[p1, p3, p2]);
            }
        }
    };

    add_face(0, 1.0);
    add_face(0, -1.0);
    add_face(1, 1.0);
    add_face(1, -1.0);
    add_face(2, 1.0);
    add_face(2, -1.0);

    (positions, faces)
}

fn testdata_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Decode `data` with the Rust decoder `iters` times.
/// Returns `(avg_us, num_points, num_faces)` or `None` on failure.
fn bench_rust_decode(data: &[u8], iters: u32) -> Option<(f64, usize, usize)> {
    let mut total = Duration::ZERO;
    let mut pts = 0;
    let mut faces = 0;
    for _ in 0..iters {
        let mut buf = DecoderBuffer::new(data);
        let mut mesh = Mesh::new();
        let mut dec = MeshDecoder::new();
        let start = Instant::now();
        dec.decode(&mut buf, &mut mesh).ok()?;
        total += start.elapsed();
        pts = mesh.num_points();
        faces = mesh.num_faces();
    }
    Some((
        total.as_secs_f64() * 1_000_000.0 / f64::from(iters),
        pts,
        faces,
    ))
}

/// Decode `data` with the C++ decoder `iters` times.
/// Returns `(avg_us, num_points, num_faces)` or `None` when the C++ test bridge is unavailable.
fn bench_cpp_decode(data: &[u8], iters: u32) -> Option<(f64, usize, usize)> {
    let r = draco_cpp_test_bridge::profile_cpp_decode(data, iters)?;
    Some((
        r.decode_time_us as f64,
        r.num_points as usize,
        r.num_faces as usize,
    ))
}

struct FileCase {
    label: &'static str,
    path: Vec<&'static str>, // relative path components from testdata/
    iters: u32,
}

#[test]
fn bench_decode_real_files() {
    common::disable_noisy_debug_env();

    if !draco_cpp_test_bridge::is_available() {
        eprintln!("SKIPPING: C++ test bridge not available");
        return;
    }

    let base = testdata_dir();

    // Files generated by the C++ encoder (reference_cpp/) — one per speed.
    // Also include some real-world .drc files.
    let reference_files: Vec<FileCase> = (0..=10)
        .map(|s| FileCase {
            label: Box::leak(format!("sphere speed {}", s).into_boxed_str()),
            path: vec![
                "reference_cpp",
                Box::leak(format!("cpp_encoded_sphere_speed_{}.drc", s).into_boxed_str()),
            ],
            iters: 200,
        })
        .chain((0..=10).map(|s| FileCase {
            label: Box::leak(format!("cube   speed {}", s).into_boxed_str()),
            path: vec![
                "reference_cpp",
                Box::leak(format!("cpp_encoded_cube_speed_{}.drc", s).into_boxed_str()),
            ],
            iters: 200,
        }))
        .collect();

    let real_files: &[FileCase] = &[
        FileCase {
            label: "bunny (cpp)",
            path: vec!["bunny_cpp.drc"],
            iters: 50,
        },
        FileCase {
            label: "bunny (cpp standard)",
            path: vec!["bunny_cpp_standard.drc"],
            iters: 50,
        },
        // testdata/cube_att.drc is a bitstream v1.1 mesh below the current
        // Draco 1.0.0+ compatibility floor. C++ expands its split attribute
        // connectivity to 24 logical mesh points, while the Rust decoder
        // currently recovers only the 8 unique position points, so it is not a
        // valid apples-to-apples decode performance case.
        FileCase {
            label: "annulus (edgbrk)",
            path: vec!["annulus_eb.drc"],
            iters: 200,
        },
        FileCase {
            label: "car",
            path: vec!["car.drc"],
            iters: 50,
        },
    ];

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║   DECODE PERFORMANCE — C++ encoded files decoded by C++ vs Rust            ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!("(Both decoders receive identical bytes produced by the C++ reference encoder)");
    println!();

    // ── Reference files (synthetic, all speeds) ──────────────────────────────
    println!("── Synthetic reference files (C++ encoder output, speeds 0–10) ─────────────");
    println!(
        "┌──────────────────────┬────────┬────────┬────────┬───────────┬───────────┬─────────┬─────────┐"
    );
    println!(
        "│ File                 │ Bytes  │ Points │ Faces  │ C++ (µs)  │ Rust (µs) │ Speedup │ Winner  │"
    );
    println!(
        "├──────────────────────┼────────┼────────┼────────┼───────────┼───────────┼─────────┼─────────┤"
    );

    for case in &reference_files {
        let mut p = base.clone();
        for seg in &case.path {
            p = p.join(seg);
        }
        let data = match std::fs::read(&p) {
            Ok(d) => d,
            Err(_) => {
                println!(
                    "│ {:<20} │  n/a   │   n/a  │   n/a  │   MISSING │   MISSING │    -    │    -    │",
                    case.label
                );
                continue;
            }
        };

        let rust = bench_rust_decode(&data, case.iters);
        let cpp = bench_cpp_decode(&data, case.iters);

        match (rust, cpp) {
            (Some((r_us, r_pts, r_faces)), Some((c_us, c_pts, c_faces))) => {
                let speedup = c_us / r_us;
                let winner = if speedup >= 1.0 { "Rust" } else { "C++" };
                let ok = if r_pts == c_pts && r_faces == c_faces {
                    "✓"
                } else {
                    "✗"
                };
                println!(
                    "│ {:<20} │{:>7} │{:>7} │{:>7} │ {:>9.1} │ {:>9.1} │ {:>6.2}x{} │ {:>7} │",
                    case.label,
                    data.len(),
                    r_pts,
                    r_faces,
                    c_us,
                    r_us,
                    speedup,
                    ok,
                    winner
                );
            }
            (None, _) => println!(
                "│ {:<20} │{:>7} │   n/a  │   n/a  │ {:>9.1} │ RUST FAIL │    -    │    -    │",
                case.label,
                data.len(),
                cpp.map(|c| c.0).unwrap_or(0.0)
            ),
            (_, None) => println!(
                "│ {:<20} │{:>7} │   n/a  │   n/a  │  BRIDGE FAIL │ {:>9.1} │    -    │    -    │",
                case.label,
                data.len(),
                rust.map(|r| r.0).unwrap_or(0.0)
            ),
        }
    }

    println!(
        "└──────────────────────┴────────┴────────┴────────┴───────────┴───────────┴─────────┴─────────┘"
    );

    // ── Generated meshes ─────────────────────────────────────────────────────
    println!();
    println!("── Generated mesh cases (C++ encoder output, speeds 0–10) ─────────────────");
    println!(
        "┌──────────────────────┬────────┬────────┬────────┬───────────┬───────────┬─────────┬─────────┐"
    );
    println!(
        "│ Mesh                 │ Bytes  │ Points │ Faces  │ C++ (µs)  │ Rust (µs) │ Speedup │ Winner  │"
    );
    println!(
        "├──────────────────────┼────────┼────────┼────────┼───────────┼───────────┼─────────┼─────────┤"
    );

    let generated_meshes = [
        ("sphere 24x48", create_uv_sphere_data(24, 48), 40),
        ("cube subdiv20", create_subdivided_cube_data(20), 40),
    ];

    for (label, (positions, faces), iters) in generated_meshes {
        for speed in 0..=10 {
            let case_label = format!("{label} s{speed}");
            let Some(data) =
                draco_cpp_test_bridge::encode_cpp_mesh(&positions, &faces, speed, speed, 10)
            else {
                println!(
                    "│ {:<20} │  n/a   │   n/a  │   n/a  │   C++ ENC │   C++ ENC │    -    │    -    │",
                    case_label
                );
                continue;
            };

            let rust = bench_rust_decode(&data, iters);
            let cpp = bench_cpp_decode(&data, iters);

            match (rust, cpp) {
                (Some((r_us, r_pts, r_faces)), Some((c_us, c_pts, c_faces))) => {
                    let speedup = c_us / r_us;
                    let winner = if speedup >= 1.0 { "Rust" } else { "C++" };
                    let ok = if r_pts == c_pts && r_faces == c_faces {
                        "✓"
                    } else {
                        "✗"
                    };
                    println!(
                        "│ {:<20} │{:>7} │{:>7} │{:>7} │ {:>9.1} │ {:>9.1} │ {:>6.2}x{} │ {:>7} │",
                        case_label,
                        data.len(),
                        r_pts,
                        r_faces,
                        c_us,
                        r_us,
                        speedup,
                        ok,
                        winner
                    );
                }
                (None, _) => println!(
                    "│ {:<20} │{:>7} │   n/a  │   n/a  │ {:>9.1} │ RUST FAIL │    -    │    -    │",
                    case_label,
                    data.len(),
                    cpp.map(|c| c.0).unwrap_or(0.0)
                ),
                (_, None) => println!(
                    "│ {:<20} │{:>7} │   n/a  │   n/a  │  BRIDGE FAIL │ {:>9.1} │    -    │    -    │",
                    case_label,
                    data.len(),
                    rust.map(|r| r.0).unwrap_or(0.0)
                ),
            }
        }
    }

    println!(
        "└──────────────────────┴────────┴────────┴────────┴───────────┴───────────┴─────────┴─────────┘"
    );

    // ── Real-world files ──────────────────────────────────────────────────────
    println!();
    println!("── Real-world .drc files ─────────────────────────────────────────────────────");
    println!(
        "┌──────────────────────┬────────┬────────┬────────┬───────────┬───────────┬─────────┬─────────┐"
    );
    println!(
        "│ File                 │ Bytes  │ Points │ Faces  │ C++ (µs)  │ Rust (µs) │ Speedup │ Winner  │"
    );
    println!(
        "├──────────────────────┼────────┼────────┼────────┼───────────┼───────────┼─────────┼─────────┤"
    );

    for case in real_files {
        let mut p = base.clone();
        for seg in &case.path {
            p = p.join(seg);
        }
        let data = match std::fs::read(&p) {
            Ok(d) => d,
            Err(_) => {
                println!(
                    "│ {:<20} │  n/a   │   n/a  │   n/a  │   MISSING │   MISSING │    -    │    -    │",
                    case.label
                );
                continue;
            }
        };

        let rust = bench_rust_decode(&data, case.iters);
        let cpp = bench_cpp_decode(&data, case.iters);

        match (rust, cpp) {
            (Some((r_us, r_pts, r_faces)), Some((c_us, c_pts, c_faces))) => {
                let speedup = c_us / r_us;
                let winner = if speedup >= 1.0 { "Rust" } else { "C++" };
                let ok = if r_pts == c_pts && r_faces == c_faces {
                    "✓"
                } else {
                    "✗"
                };
                println!(
                    "│ {:<20} │{:>7} │{:>7} │{:>7} │ {:>9.1} │ {:>9.1} │ {:>6.2}x{} │ {:>7} │",
                    case.label,
                    data.len(),
                    r_pts,
                    r_faces,
                    c_us,
                    r_us,
                    speedup,
                    ok,
                    winner
                );
            }
            (None, _) => println!(
                "│ {:<20} │{:>7} │   n/a  │   n/a  │ {:>9.1} │ RUST FAIL │    -    │    -    │",
                case.label,
                data.len(),
                cpp.map(|c| c.0).unwrap_or(0.0)
            ),
            (_, None) => println!(
                "│ {:<20} │{:>7} │   n/a  │   n/a  │  BRIDGE FAIL │ {:>9.1} │    -    │    -    │",
                case.label,
                data.len(),
                rust.map(|r| r.0).unwrap_or(0.0)
            ),
        }
    }

    println!(
        "└──────────────────────┴────────┴────────┴────────┴───────────┴───────────┴─────────┴─────────┘"
    );
    println!();
    println!("Notes:");
    println!("  • Speedup > 1.0x means Rust is faster");
    println!("  • ✓ = decoded point/face counts match between Rust and C++");
    println!("  • Iterations: reference files = 200, generated meshes = 40, real-world files = 50");
    println!("  • C++ built with /O2 (Release), Rust with opt-level=3 + thin LTO");
    println!();
}
