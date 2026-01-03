/// Debug test to understand why Ngon_12 Draco buffer fails to decode
use std::fs;

#[test]
fn debug_decode_ngon12() {
    let draco_data = fs::read("input/ngon12.drc").expect("ngon12.drc not found");
    println!("Draco buffer size: {} bytes", draco_data.len());
    println!("First 50 bytes: {:02X?}", &draco_data[..50.min(draco_data.len())]);

    // Parse header manually
    assert_eq!(&draco_data[0..5], b"DRACO", "Invalid magic");
    let version_major = draco_data[5];
    let version_minor = draco_data[6];
    let geometry_type = draco_data[7];
    let encoding_method = draco_data[8];
    println!("Version: {}.{}", version_major, version_minor);
    println!("Geometry type: {} (1=mesh)", geometry_type);
    println!("Encoding method: {} (1=edgebreaker)", encoding_method);

    // Flags (version >= 1.3)
    let flags = u16::from_le_bytes([draco_data[9], draco_data[10]]);
    println!("Flags: 0x{:04X}", flags);

    // Dump bytes 190-225 for TexCoord analysis
    println!("\nBytes 190-{}: {:02X?}", draco_data.len(), &draco_data[190..]);
    
    // Print in formatted groups
    println!("\nFormatted bytes starting at position 194 (where TexCoord decode_values starts):");
    for i in (194..draco_data.len()).step_by(4) {
        let end = (i + 4).min(draco_data.len());
        let bytes = &draco_data[i..end];
        if bytes.len() == 4 {
            let float_val = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let int_val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            println!("  [{}..{}] {:02X?} float={:.6} u32={}", i, end, bytes, float_val, int_val);
        } else {
            println!("  [{}..{}] {:02X?}", i, end, bytes);
        }
    }

    // Now decode using the Rust decoder
    let mut decoder_buffer = draco_core::decoder_buffer::DecoderBuffer::new(&draco_data);
    let mut mesh = draco_core::mesh::Mesh::new();
    let mut decoder = draco_core::mesh_decoder::MeshDecoder::new();

    match decoder.decode(&mut decoder_buffer, &mut mesh) {
        Ok(()) => {
            println!("SUCCESS: Decoded mesh with {} faces, {} points", mesh.num_faces(), mesh.num_points());
            println!("Attributes: {}", mesh.num_attributes());
            for i in 0..mesh.num_attributes() {
                let att = mesh.attribute(i as i32);
                println!("  Attribute {}: {:?}, {} components, {} entries",
                    i, att.attribute_type(), att.num_components(), att.size());
            }
        }
        Err(e) => {
            println!("DECODE FAILED: {:?}", e);
            println!("Buffer position at failure: {}", decoder_buffer.position());
        }
    }
}
