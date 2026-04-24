// Performance comparison test using direct C++ FFI bindings
// This gives accurate comparison without process startup overhead

mod common;

use draco_core::draco_types::DataType;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
use draco_core::geometry_indices::{FaceIndex, PointIndex};
use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::EncoderOptions;
use draco_cpp_ffi;
use std::time::Instant;

fn create_grid_mesh_data(grid_size: usize) -> (Vec<f32>, Vec<u32>) {
    let num_points = grid_size * grid_size;
    let num_faces = (grid_size - 1) * (grid_size - 1) * 2;

    // Create positions
    let mut positions = Vec::with_capacity(num_points * 3);
    for y in 0..grid_size {
        for x in 0..grid_size {
            let px = x as f32;
            let py = y as f32;
            let pz = (x as f32 * 0.2).sin() * (y as f32 * 0.2).cos() * 2.0;
            positions.push(px);
            positions.push(py);
            positions.push(pz);
        }
    }

    // Create faces
    let mut faces = Vec::with_capacity(num_faces * 3);
    for y in 0..grid_size - 1 {
        for x in 0..grid_size - 1 {
            let p0 = (y * grid_size + x) as u32;
            let p1 = (y * grid_size + x + 1) as u32;
            let p2 = ((y + 1) * grid_size + x) as u32;
            let p3 = ((y + 1) * grid_size + x + 1) as u32;

            faces.push(p0);
            faces.push(p1);
            faces.push(p2);

            faces.push(p1);
            faces.push(p3);
            faces.push(p2);
        }
    }

    (positions, faces)
}

fn create_mesh_from_data(positions: &[f32], faces: &[u32]) -> Mesh {
    let num_points = positions.len() / 3;
    let num_faces = faces.len() / 3;

    let mut mesh = Mesh::new();
    mesh.set_num_points(num_points);
    mesh.set_num_faces(num_faces);

    let mut pos_attr = PointAttribute::new();
    pos_attr.init(
        GeometryAttributeType::Position,
        3,
        DataType::Float32,
        false,
        num_points,
    );

    for i in 0..num_points {
        let offset = i * 3 * 4;
        pos_attr
            .buffer_mut()
            .update(&positions[i * 3].to_le_bytes(), Some(offset));
        pos_attr
            .buffer_mut()
            .update(&positions[i * 3 + 1].to_le_bytes(), Some(offset + 4));
        pos_attr
            .buffer_mut()
            .update(&positions[i * 3 + 2].to_le_bytes(), Some(offset + 8));
    }
    mesh.add_attribute(pos_attr);

    for i in 0..num_faces {
        mesh.set_face(
            FaceIndex(i as u32),
            [
                PointIndex(faces[i * 3]),
                PointIndex(faces[i * 3 + 1]),
                PointIndex(faces[i * 3 + 2]),
            ],
        );
    }

    mesh
}

fn benchmark_rust_encoding(mesh: &Mesh, speed: i32, iterations: u32) -> (f64, usize) {
    let mut total_time = 0.0;
    let mut output_size = 0;

    for _ in 0..iterations {
        let mut options = EncoderOptions::new();
        options.set_global_int("encoding_speed", speed);
        options.set_global_int("decoding_speed", speed);
        options.set_attribute_int(0, "quantization_bits", 10);

        let start = Instant::now();
        let mut encoder = MeshEncoder::new();
        encoder.set_mesh(mesh.clone());
        let mut encoder_buffer = EncoderBuffer::new();
        let _ = encoder.encode(&options, &mut encoder_buffer);
        let elapsed = start.elapsed();

        total_time += elapsed.as_secs_f64();
        output_size = encoder_buffer.data().len();
    }

    (total_time / iterations as f64, output_size)
}

#[test]
fn test_ffi_performance_comparison() {
    common::disable_noisy_debug_env();

    if !draco_cpp_ffi::is_available() {
        eprintln!("SKIPPING: C++ FFI not available");
        return;
    }

    let (major, minor, revision) = draco_cpp_ffi::get_version();
    println!("\n=== Rust vs C++ Encoding Performance (Direct FFI) ===");
    println!("C++ Draco version: {}.{}.{}\n", major, minor, revision);

    let grid_sizes = [50, 100];
    let speeds = [0, 1, 5, 10];
    let iterations = 5;

    println!(
        "{:>6} {:>6} {:>12} {:>12} {:>10} {:>14}",
        "Grid", "Speed", "Rust (ms)", "C++ (ms)", "Speedup", "Size"
    );
    println!("{}", "-".repeat(75));

    for &grid_size in &grid_sizes {
        let (positions, faces) = create_grid_mesh_data(grid_size);
        let mesh = create_mesh_from_data(&positions, &faces);

        let num_faces = (grid_size - 1) * (grid_size - 1) * 2;
        let num_points = grid_size * grid_size;

        println!(
            "\nGrid {}x{} ({} points, {} faces):",
            grid_size, grid_size, num_points, num_faces
        );

        for &speed in &speeds {
            // Rust benchmark
            let (rust_time, rust_size) = benchmark_rust_encoding(&mesh, speed, iterations);

            // C++ FFI benchmark
            let cpp_result = draco_cpp_ffi::benchmark_cpp_encode(
                &positions, &faces, speed, speed, 10, // quantization_bits
                iterations,
            );

            let (cpp_time, cpp_size) = match cpp_result {
                Some(r) => (r.avg_time_us as f64 / 1000.0, r.output_size), // us to ms
                None => {
                    eprintln!("  C++ encoding failed for speed {}", speed);
                    continue;
                }
            };

            let rust_ms = rust_time * 1000.0;
            let speedup = if rust_ms > 0.0 {
                cpp_time / rust_ms
            } else {
                0.0
            };

            let size_status = if rust_size == cpp_size {
                "✓ match".to_string()
            } else {
                format!("✗ R:{} C:{}", rust_size, cpp_size)
            };

            println!(
                "{:>6} {:>6} {:>10.2}ms {:>10.2}ms {:>9.2}x   {}",
                grid_size, speed, rust_ms, cpp_time, speedup, size_status
            );
        }
    }

    println!("\nNote: Speedup > 1.0 means Rust is faster, < 1.0 means C++ is faster");
    println!(
        "      Times are averaged over {} iterations (no process startup overhead)\n",
        iterations
    );
}

#[test]
fn test_encoding_correctness() {
    common::disable_noisy_debug_env();

    if !draco_cpp_ffi::is_available() {
        eprintln!("SKIPPING: C++ FFI not available");
        return;
    }

    println!("\n=== Encoding Output Correctness Test ===\n");

    let grid_sizes = [20, 50];
    let speeds = [0, 5, 10];
    let mut all_pass = true;

    for &grid_size in &grid_sizes {
        let (positions, faces) = create_grid_mesh_data(grid_size);
        let mesh = create_mesh_from_data(&positions, &faces);

        for &speed in &speeds {
            // Rust encoding
            let mut options = EncoderOptions::new();
            options.set_global_int("encoding_speed", speed);
            options.set_global_int("decoding_speed", speed);
            options.set_attribute_int(0, "quantization_bits", 10);

            let mut encoder = MeshEncoder::new();
            encoder.set_mesh(mesh.clone());
            let mut encoder_buffer = EncoderBuffer::new();
            let _ = encoder.encode(&options, &mut encoder_buffer);
            let rust_size = encoder_buffer.data().len();

            // C++ encoding
            let cpp_result = draco_cpp_ffi::benchmark_cpp_encode(
                &positions, &faces, speed, speed, 10, 1, // single iteration
            );

            let cpp_size = cpp_result.map(|r| r.output_size).unwrap_or(0);

            let status = if rust_size == cpp_size {
                "PASS"
            } else {
                "FAIL";
                all_pass = false;
                "FAIL"
            };
            println!(
                "Grid {}x{}, speed {}: Rust={} bytes, C++={} bytes [{}]",
                grid_size, grid_size, speed, rust_size, cpp_size, status
            );
        }
    }

    assert!(all_pass, "Some encoding outputs did not match!");
}
