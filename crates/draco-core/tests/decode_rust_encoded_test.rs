use draco_core::{DecoderBuffer, Mesh, MeshDecoder};
use std::fs;

#[test]
fn test_decode_rust_encoded() {
    // Read the Rust-encoded file
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/lamp_mesh.drc");
    let data = fs::read(path).expect("Failed to read file");

    println!("File size: {} bytes", data.len());
    println!("First 30 bytes: {:?}", &data[..30.min(data.len())]);

    let mut buffer = DecoderBuffer::new(&data);
    let mut decoder = MeshDecoder::new();
    let mut mesh = Mesh::new();

    match decoder.decode(&mut buffer, &mut mesh) {
        Ok(()) => {
            println!("Decoded successfully!");
            println!("  Num faces: {}", mesh.num_faces());
            println!("  Num points: {}", mesh.num_points());
            println!("  Num attributes: {}", mesh.num_attributes());

            for i in 0..mesh.num_attributes() {
                let att = mesh.attribute(i);
                println!(
                    "  Attribute {}: type={:?}, components={}",
                    i,
                    att.attribute_type(),
                    att.num_components()
                );
            }
        }
        Err(e) => {
            println!("Decode FAILED: {:?}", e);
        }
    }
}

#[test]
fn test_decode_cpp_encoded() {
    // Read the C++ standard edgebreaker encoded file
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/lamp_cpp_std.drc");
    let data = fs::read(path).expect("Failed to read file");

    println!("File size: {} bytes", data.len());
    println!("First 30 bytes: {:?}", &data[..30.min(data.len())]);

    let mut buffer = DecoderBuffer::new(&data);
    let mut decoder = MeshDecoder::new();
    let mut mesh = Mesh::new();

    match decoder.decode(&mut buffer, &mut mesh) {
        Ok(()) => {
            println!("Decoded successfully!");
            println!("  Num faces: {}", mesh.num_faces());
            println!("  Num points: {}", mesh.num_points());
            println!("  Num attributes: {}", mesh.num_attributes());

            for i in 0..mesh.num_attributes() {
                let att = mesh.attribute(i);
                println!(
                    "  Attribute {}: type={:?}, components={}",
                    i,
                    att.attribute_type(),
                    att.num_components()
                );
            }
        }
        Err(e) => {
            println!("Decode FAILED: {:?}", e);
        }
    }
}
