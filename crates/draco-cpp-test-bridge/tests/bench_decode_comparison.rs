use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::mesh::Mesh;
use draco_core::mesh_decoder::MeshDecoder;
use std::time::Instant;

mod common;

fn create_test_mesh_data(grid_size: usize) -> (Vec<f32>, Vec<u32>) {
    let num_points = grid_size * grid_size;
    let num_faces = (grid_size - 1) * (grid_size - 1) * 2;

    let mut positions = Vec::with_capacity(num_points * 3);
    for y in 0..grid_size {
        for x in 0..grid_size {
            positions.push(x as f32);
            positions.push(y as f32);
            positions.push((x as f32 * 0.2).sin() * (y as f32 * 0.2).cos() * 2.0);
        }
    }

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

// Use the bridge function re-exported by the crate (safer and ensures linking through the crate)
// Note: function is declared in the crate via `pub use ffi::*;` and can be called as `draco_cpp_test_bridge::draco_benchmark_decode_mesh`.

#[test]
fn bench_decode_comparison() {
    common::disable_noisy_debug_env();
    if common::skip_if_cpp_bridge_unavailable() {
        return;
    }

    println!("\nComparing Rust vs C++ decode performance on C++-encoded grids");
    println!(
        "{:>7} {:>8} {:>7} {:>10} {:>8} {:>10} {:>10} {:>9}",
        "Grid", "Speed", "Iters", "Bytes", "Faces", "Rust µs", "C++ µs", "Speedup"
    );
    println!("{}", "-".repeat(86));

    for (grid_size, iterations) in [(20, 200), (50, 80), (100, 30)] {
        let (positions, faces) = create_test_mesh_data(grid_size);
        let num_faces = (grid_size - 1) * (grid_size - 1) * 2;

        for speed in [0, 1, 5, 10] {
            let encoded_data =
                draco_cpp_test_bridge::encode_cpp_mesh(&positions, &faces, speed, speed, 10)
                    .expect("C++ encode failed");

            for _ in 0..iterations.min(5) {
                let mut decoder_buffer = DecoderBuffer::new(&encoded_data);
                let mut out_mesh = Mesh::new();
                let mut decoder = MeshDecoder::new();
                decoder.decode(&mut decoder_buffer, &mut out_mesh).unwrap();
            }

            let start = Instant::now();
            for _ in 0..iterations {
                let mut decoder_buffer = DecoderBuffer::new(&encoded_data);
                let mut out_mesh = Mesh::new();
                let mut decoder = MeshDecoder::new();
                decoder.decode(&mut decoder_buffer, &mut out_mesh).unwrap();
            }
            let rust_elapsed = start.elapsed();
            let rust_avg_us = rust_elapsed.as_micros() / iterations as u128;

            let (cpp_avg_us_raw, _num_points, _num_faces) =
                draco_cpp_test_bridge::benchmark_cpp_decode(&encoded_data, iterations as u32)
                    .expect("C++ benchmark decode failed");
            let cpp_avg_us = cpp_avg_us_raw as u128;

            let speedup = cpp_avg_us as f64 / rust_avg_us as f64;
            println!(
                "{:>7} {:>8} {:>7} {:>10} {:>8} {:>10} {:>10} {:>8.2}x",
                format!("{grid_size}x{grid_size}"),
                speed,
                iterations,
                encoded_data.len(),
                num_faces,
                rust_avg_us,
                cpp_avg_us,
                speedup
            );
        }
        println!("{}", "-".repeat(86));
    }
}
