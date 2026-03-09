//! Test PLY bunny file parsing and encoding.

use std::fs;

use std::path::Path;

/// Parse the bun_zipper.ply file to verify the structure  
#[test]
fn test_ply_bunny_structure() {
    let ply_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/bun_zipper.ply");
    println!("Reading: {:?}", ply_path);
    
    let content = fs::read_to_string(&ply_path).expect("Failed to read PLY file");
    let mut lines = content.lines();
    
    // Parse header
    let first_line = lines.next().unwrap();
    assert_eq!(first_line, "ply", "Expected PLY header");
    
    let mut vertex_count = 0;
    let mut face_count = 0;
    let mut vertex_properties: Vec<String> = Vec::new();
    let mut in_vertex_element = false;
    
    for line in &mut lines {
        let line = line.trim();
        if line == "end_header" {
            break;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        match parts[0] {
            "element" => {
                if parts.len() >= 3 {
                    if parts[1] == "vertex" {
                        vertex_count = parts[2].parse().unwrap_or(0);
                        in_vertex_element = true;
                    } else {
                        in_vertex_element = false;
                        if parts[1] == "face" {
                            face_count = parts[2].parse().unwrap_or(0);
                        }
                    }
                }
            }
            "property" => {
                if in_vertex_element && parts.len() >= 3 {
                    vertex_properties.push(parts[2].to_string());
                }
            }
            _ => {}
        }
    }
    
    println!("\n=== PLY Header Info ===");
    println!("Vertex count: {}", vertex_count);
    println!("Face count: {}", face_count);
    println!("Vertex properties: {:?}", vertex_properties);
    
    // Count actual data lines (vertices + faces)
    let mut vertex_data_count = 0;
    let mut face_data_count = 0;
    let mut total_indices = 0;
    
    // Read vertex data
    for _ in 0..vertex_count {
        if lines.next().is_some() {
            vertex_data_count += 1;
        }
    }
    
    // Read face data  
    for _ in 0..face_count {
        if let Some(line) = lines.next() {
            face_data_count += 1;
            // Face format: n i0 i1 i2 ...
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let n: usize = parts[0].parse().unwrap_or(0);
                total_indices += n;
            }
        }
    }
    
    println!("\n=== Data Counts ===");
    println!("Vertex data lines: {}", vertex_data_count);
    println!("Face data lines: {}", face_data_count);
    println!("Total face indices: {}", total_indices);
    
    // For triangles: face_count * 3 = total_indices
    let triangulated_count = (total_indices as f64 - 2.0 * face_count as f64) as usize;
    println!("Estimated triangulated faces: {} (for 69451 tris should be ~69451 if all tris)", triangulated_count);
    
    // Verify expectations
    assert_eq!(vertex_count, 35947, "Expected 35947 vertices in header");
    assert_eq!(face_count, 69451, "Expected 69451 faces in header");
    
    // Properties should NOT include nx, ny, nz (normals)
    assert!(!vertex_properties.contains(&"nx".to_string()), "File should not have normals");
    assert!(!vertex_properties.contains(&"ny".to_string()), "File should not have normals");
    assert!(!vertex_properties.contains(&"nz".to_string()), "File should not have normals");
    
    // Expected properties: x, y, z, confidence, intensity
    assert!(vertex_properties.contains(&"x".to_string()), "Should have x");
    assert!(vertex_properties.contains(&"y".to_string()), "Should have y");
    assert!(vertex_properties.contains(&"z".to_string()), "Should have z");
    
    println!("\n=== Test passed! ===");
}


/// Parse PLY file like the WASM module does
fn parse_ply_like_wasm(content: &str) -> (Vec<f32>, Vec<u32>, Vec<f32>) {
    let mut lines = content.lines().peekable();

    // Parse header
    let first_line = lines.next().unwrap_or("").trim();
    assert_eq!(first_line, "ply");

    let mut vertex_count = 0usize;
    let mut face_count = 0usize;
    let mut current_element = String::new();
    let mut vertex_properties: Vec<String> = Vec::new();

    for l in lines.by_ref() {
        let line = l.trim();

        if line == "end_header" {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "element" => {
                if parts.len() >= 3 {
                    current_element = parts[1].to_string();
                    let count: usize = parts[2].parse().unwrap_or(0);
                    if current_element == "vertex" {
                        vertex_count = count;
                    } else if current_element == "face" {
                        face_count = count;
                    }
                }
            }
            "property" => {
                if parts.len() >= 3 && current_element == "vertex" && parts[1] != "list" {
                    vertex_properties.push(parts[2].to_string());
                }
            }
            _ => {}
        }
    }

    // Find property indices
    let x_idx = vertex_properties.iter().position(|p| p == "x");
    let y_idx = vertex_properties.iter().position(|p| p == "y");
    let z_idx = vertex_properties.iter().position(|p| p == "z");
    let nx_idx = vertex_properties.iter().position(|p| p == "nx");
    let ny_idx = vertex_properties.iter().position(|p| p == "ny");
    let nz_idx = vertex_properties.iter().position(|p| p == "nz");

    let has_normals = nx_idx.is_some() && ny_idx.is_some() && nz_idx.is_some();

    let mut positions: Vec<f32> = Vec::with_capacity(vertex_count * 3);
    let mut normals: Vec<f32> = Vec::new();

    if has_normals {
        normals.reserve(vertex_count * 3);
    }

    // Parse vertices
    for _ in 0..vertex_count {
        let line = match lines.next() {
            Some(l) => l.trim(),
            None => break,
        };

        let values: Vec<f32> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if let (Some(xi), Some(yi), Some(zi)) = (x_idx, y_idx, z_idx) {
            positions.push(*values.get(xi).unwrap_or(&0.0));
            positions.push(*values.get(yi).unwrap_or(&0.0));
            positions.push(*values.get(zi).unwrap_or(&0.0));
        }

        if has_normals {
            if let (Some(nxi), Some(nyi), Some(nzi)) = (nx_idx, ny_idx, nz_idx) {
                normals.push(*values.get(nxi).unwrap_or(&0.0));
                normals.push(*values.get(nyi).unwrap_or(&0.0));
                normals.push(*values.get(nzi).unwrap_or(&0.0));
            }
        }
    }

    // Parse faces
    let mut indices: Vec<u32> = Vec::with_capacity(face_count * 3);

    for _ in 0..face_count {
        let line = match lines.next() {
            Some(l) => l.trim(),
            None => break,
        };

        let values: Vec<u32> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if values.is_empty() {
            continue;
        }

        let count = values[0] as usize;
        if values.len() < count + 1 {
            continue;
        }

        // Triangulate (fan triangulation for polygons)
        for j in 1..count - 1 {
            indices.push(values[1]);
            indices.push(values[j + 1]);
            indices.push(values[j + 2]);
        }
    }

    (positions, indices, normals)
}


/// Test parsing PLY like the WASM module does and encode with Draco
#[test]
fn test_ply_bunny_encode_draco() {
    use draco_core::encoder_buffer::EncoderBuffer;
    use draco_core::encoder_options::EncoderOptions;
    use draco_core::mesh::Mesh as DracoMesh;
    use draco_core::mesh_encoder::MeshEncoder;
    use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
    use draco_core::draco_types::DataType;
    use draco_core::geometry_indices::PointIndex;
    
    let ply_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/bun_zipper.ply");
    let content = fs::read_to_string(&ply_path).expect("Failed to read PLY file");
    
    let (positions, indices, normals) = parse_ply_like_wasm(&content);
    
    println!("\n=== Parsed PLY Data ===");
    println!("Positions length: {}", positions.len());
    println!("Indices length: {}", indices.len());
    println!("Normals length: {}", normals.len());
    
    let vertex_count = positions.len() / 3;
    let face_count = indices.len() / 3;
    
    println!("Vertex count: {}", vertex_count);
    println!("Face count: {}", face_count);
    
    // Verify counts match expectations
    assert_eq!(vertex_count, 35947, "Expected 35947 vertices");
    assert_eq!(positions.len(), 35947 * 3, "Expected 35947 * 3 = 107841 position floats");
    assert_eq!(face_count, 69451, "Expected 69451 faces");
    assert_eq!(indices.len(), 69451 * 3, "Expected 69451 * 3 = 208353 indices");
    assert_eq!(normals.len(), 0, "Expected 0 normals (bunny has no normals)");
    
    // Verify index range
    let max_index = indices.iter().max().copied().unwrap_or(0);
    let min_index = indices.iter().min().copied().unwrap_or(0);
    println!("Index range: {} to {}", min_index, max_index);
    assert!(max_index < vertex_count as u32, "Max index should be < vertex count");
    
    // Now create Draco mesh and encode
    println!("\n=== Creating Draco Mesh ===");
    let mut draco_mesh = DracoMesh::new();
    draco_mesh.set_num_points(vertex_count);

    // Add position attribute
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(
        GeometryAttributeType::Position,
        3,
        DataType::Float32,
        false,
        vertex_count,
    );
    let pos_buffer = pos_attr.buffer_mut();
    for (i, chunk) in positions.chunks(3).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        pos_buffer.write(i * 12, &bytes);
    }
    draco_mesh.add_attribute(pos_attr);

    // Add faces
    for i in 0..face_count {
        let i0 = PointIndex(indices[i * 3]);
        let i1 = PointIndex(indices[i * 3 + 1]);
        let i2 = PointIndex(indices[i * 3 + 2]);
        draco_mesh.add_face([i0, i1, i2]);
    }
    
    println!("Draco mesh created:");
    println!("  num_points: {}", draco_mesh.num_points());
    println!("  num_faces: {}", draco_mesh.num_faces());
    
    // Encode
    println!("\n=== Encoding with Draco ===");
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(draco_mesh);
    let mut encoder_buffer = EncoderBuffer::new();
    let mut enc_options = EncoderOptions::default();
    enc_options.set_attribute_int(0, "quantization_bits", 0);
    
    encoder.encode(&enc_options, &mut encoder_buffer)
        .expect("Draco encoding should succeed");
    
    let encoded_data = encoder_buffer.data();
    println!("Encoded size: {} bytes", encoded_data.len());
    
    assert!(!encoded_data.is_empty(), "Encoded data should not be empty");
    
    // Now decode and verify
    println!("\n=== Decoding Draco ===");
    use draco_core::decoder_buffer::DecoderBuffer;
    use draco_core::mesh_decoder::MeshDecoder;
    
    let mut decoder_buffer = DecoderBuffer::new(encoded_data);
    
    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = DracoMesh::new();
    decoder.decode(&mut decoder_buffer, &mut decoded_mesh)
        .expect("Decoding should succeed");
    
    println!("Decoded mesh:");
    println!("  num_points: {}", decoded_mesh.num_points());
    println!("  num_faces: {}", decoded_mesh.num_faces());
    
    // Verify decoded counts match original
    // Verify decoded counts (Draco may deduplicate vertices, so <= is expected)
    assert!(decoded_mesh.num_points() <= vertex_count, "Decoded vertex count mismatch: {} > {}", decoded_mesh.num_points(), vertex_count);
    // Sanity check
    assert!(decoded_mesh.num_points() > 34000, "Decoded vertex count too low: {}", decoded_mesh.num_points());
    assert_eq!(decoded_mesh.num_faces(), face_count, "Decoded face count mismatch");
    
    println!("\n=== Test passed! ===");
}

/// Test that bunny encoded with Rust can be decoded by C++ decoder
#[test]
fn test_bunny_cpp_interop() {
    use std::process::Command;
    use draco_core::encoder_buffer::EncoderBuffer;
    use draco_core::encoder_options::EncoderOptions;
    use draco_core::mesh::Mesh as DracoMesh;
    use draco_core::mesh_encoder::MeshEncoder;
    use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
    use draco_core::draco_types::DataType;
    use draco_core::geometry_indices::PointIndex;

    let ply_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/bun_zipper.ply");
    println!("Reading PLY: {:?}", ply_path);
    
    // Parse PLY manually since the reader doesn't handle faces yet
    let content = fs::read_to_string(&ply_path).expect("Failed to read PLY file");
    let (positions, indices, _normals) = parse_ply_like_wasm(&content);
    
    let vertex_count = positions.len() / 3;
    let face_count = indices.len() / 3;
    
    println!("Loaded mesh: {} points, {} faces", vertex_count, face_count);
    assert_eq!(vertex_count, 35947, "Expected 35947 vertices");
    assert_eq!(face_count, 69451, "Expected 69451 faces");
    
    // Create Draco mesh
    let mut mesh = DracoMesh::new();
    mesh.set_num_points(vertex_count);

    // Add position attribute
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(
        GeometryAttributeType::Position,
        3,
        DataType::Float32,
        false,
        vertex_count,
    );
    let pos_buffer = pos_attr.buffer_mut();
    for (i, chunk) in positions.chunks(3).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        pos_buffer.write(i * 12, &bytes);
    }
    mesh.add_attribute(pos_attr);

    // Add faces
    for i in 0..face_count {
        let i0 = PointIndex(indices[i * 3]);
        let i1 = PointIndex(indices[i * 3 + 1]);
        let i2 = PointIndex(indices[i * 3 + 2]);
        mesh.add_face([i0, i1, i2]);
    }
    
    // Encode with Draco
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh);
    let mut encoder_buffer = EncoderBuffer::new();
    let enc_options = EncoderOptions::default();
    
    encoder.encode(&enc_options, &mut encoder_buffer)
        .expect("Draco encoding should succeed");
    
    let encoded_data = encoder_buffer.data();
    println!("Rust encoded size: {} bytes", encoded_data.len());
    
    // Save to temp file
    let output_path = std::env::temp_dir().join("bunny_rust_encoded.drc");
    fs::write(&output_path, encoded_data).expect("Failed to write file");
    println!("Saved to: {:?}", output_path);
    
    // Try to decode with C++ decoder (if available via env var or default paths)
    let cpp_decoder_path = std::env::var("DRACO_CPP_DECODER")
        .ok()
        .or_else(|| {
            let candidates = [
                "../../build-original/src/draco/Release/draco_decoder.exe",
                "../../build/src/draco/Release/draco_decoder.exe",
                "../../build/src/draco/Debug/draco_decoder.exe",
            ];
            candidates.iter()
                .find(|p| Path::new(p).exists())
                .map(|s| s.to_string())
        });
    
    if let Some(decoder_path) = cpp_decoder_path {
        if Path::new(&decoder_path).exists() {
            let output = Command::new(&decoder_path)
                .args(["-i", output_path.to_string_lossy().as_ref()])
                .output()
                .expect("Failed to run C++ decoder");
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("C++ decoder stdout:\n{}", stdout);
            println!("C++ decoder stderr:\n{}", stderr);
            
            // Check for success
            if stdout.contains("Failed") || stderr.contains("Failed") {
                panic!("C++ decoder failed to decode Rust-encoded bunny!");
            } else {
                println!("SUCCESS: C++ decoder can decode Rust-encoded bunny!");
            }
        } else {
            println!("C++ decoder not found at {:?}, skipping interop test", decoder_path);
        }
    } else {
        println!("C++ decoder not found, skipping interop test");
    }
}
