use std::time::Instant;
use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::geometry_indices::{PointIndex, FaceIndex};
use draco_core::EncoderOptions;
use draco_core::geometry_attribute::{PointAttribute, GeometryAttributeType};
use draco_core::draco_types::DataType;

mod common;

fn create_test_mesh(grid_size: usize) -> Mesh {
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
            faces.push(p0); faces.push(p1); faces.push(p2);
            faces.push(p1); faces.push(p3); faces.push(p2);
        }
    }
    let mut mesh = Mesh::new();
    mesh.set_num_points(num_points);
    mesh.set_num_faces(num_faces);
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(GeometryAttributeType::Position, 3, DataType::Float32, false, num_points);
    for i in 0..num_points {
        let offset = i * 3 * 4;
        pos_attr.buffer_mut().update(&positions[i * 3].to_le_bytes(), Some(offset));
        pos_attr.buffer_mut().update(&positions[i * 3 + 1].to_le_bytes(), Some(offset + 4));
        pos_attr.buffer_mut().update(&positions[i * 3 + 2].to_le_bytes(), Some(offset + 8));
    }
    mesh.add_attribute(pos_attr);
    for i in 0..num_faces {
        mesh.set_face(FaceIndex(i as u32), [PointIndex(faces[i * 3]), PointIndex(faces[i * 3 + 1]), PointIndex(faces[i * 3 + 2])]);
    }
    mesh
}

// Use the FFI function re-exported by the crate (safer and ensures linking through the crate)
// Note: function is declared in the crate via `pub use ffi::*;` and can be called as `draco_cpp_ffi::draco_benchmark_decode_mesh`.

#[test]
fn bench_decode_comparison() {
    common::disable_noisy_debug_env();
    if common::skip_if_ffi_unavailable() {
        return;
    }

    let mesh = create_test_mesh(100);
    let iterations = 100;
    
    println!("\nComparing Rust vs C++ decode performance");
    println!("Mesh: 10,000 vertices, 19,602 faces");
    println!("Iterations: {}\n", iterations);
    
    for speed in [0, 1, 5, 10] {
        let mut options = EncoderOptions::new();
        options.set_global_int("encoding_speed", speed);
        options.set_global_int("decoding_speed", speed);
        options.set_attribute_int(0, "quantization_bits", 10);
        
        let mut encoder = MeshEncoder::new();
        encoder.set_mesh(mesh.clone());
        let mut encoder_buffer = EncoderBuffer::new();
        encoder.encode(&options, &mut encoder_buffer).unwrap();
        let encoded_data = encoder_buffer.data().to_vec();
        
        // Warmup Rust decoder
        for _ in 0..5 {
            let mut decoder_buffer = DecoderBuffer::new(&encoded_data);
            let mut out_mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            decoder.decode(&mut decoder_buffer, &mut out_mesh).unwrap();
        }
        
        // Benchmark Rust decoder
        let start = Instant::now();
        for _ in 0..iterations {
            let mut decoder_buffer = DecoderBuffer::new(&encoded_data);
            let mut out_mesh = Mesh::new();
            let mut decoder = MeshDecoder::new();
            decoder.decode(&mut decoder_buffer, &mut out_mesh).unwrap();
        }
        let rust_elapsed = start.elapsed();
        let rust_avg_us = rust_elapsed.as_micros() / iterations as u128;
        
        // Benchmark C++ decoder (it runs iterations internally and returns the average)
        let (cpp_avg_us_raw, _num_points, _num_faces) = draco_cpp_ffi::benchmark_cpp_decode(&encoded_data, iterations as u32)
            .expect("C++ benchmark decode failed");
        // benchmark_cpp_decode already returns total_time / iterations (the average), so no further division needed
        let cpp_avg_us = cpp_avg_us_raw as u128;
        
        let ratio = rust_avg_us as f64 / cpp_avg_us as f64;
        let pct_diff = ((ratio - 1.0) * 100.0).abs();
        let comparison = if ratio > 1.05 {
            format!("Rust {:.1}% slower", pct_diff)
        } else if ratio < 0.95 {
            format!("Rust {:.1}% faster", pct_diff)
        } else {
            "Similar performance".to_string()
        };
        
        println!("Speed {}: {} bytes", speed, encoded_data.len());
        println!("  Rust:  {:>6} μs avg", rust_avg_us);
        println!("  C++:   {:>6} μs avg", cpp_avg_us);
        println!("  Ratio: {:.2}x ({})", ratio, comparison);
        println!();
    }
}
