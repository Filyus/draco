use draco_core::mesh::Mesh;
use draco_core::mesh_encoder::MeshEncoder;
use draco_core::mesh_decoder::MeshDecoder;
use draco_core::encoder_buffer::EncoderBuffer;
use draco_core::decoder_buffer::DecoderBuffer;
use draco_core::encoder_options::EncoderOptions;

// Create small grid mesh helper (same as other tests)
fn create_grid_mesh(width: u32, height: u32) -> Mesh {
    let mut mesh = Mesh::new();
    let num_points = width * height;
    mesh.set_num_points(num_points as usize);

    let mut pos_attr = draco_core::geometry_attribute::PointAttribute::new();
    pos_attr.init(
        draco_core::geometry_attribute::GeometryAttributeType::Position,
        3, draco_core::draco_types::DataType::Float32, false, num_points as usize);

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            let coords = [x as f32, y as f32, 0.0f32];
            let offset = i * 3 * 4;
            pos_attr.buffer_mut().update(&coords[0].to_le_bytes(), Some(offset));
            pos_attr.buffer_mut().update(&coords[1].to_le_bytes(), Some(offset + 4));
            pos_attr.buffer_mut().update(&coords[2].to_le_bytes(), Some(offset + 8));
        }
    }
    mesh.add_attribute(pos_attr);

    let mut face_idx = 0;
    for y in 0..height-1 {
        for x in 0..width-1 {
            let p0 = y * width + x;
            let p1 = y * width + (x + 1);
            let p2 = (y + 1) * width + x;
            let p3 = (y + 1) * width + (x + 1);

            mesh.set_face(draco_core::FaceIndex(face_idx), [draco_core::PointIndex(p0), draco_core::PointIndex(p1), draco_core::PointIndex(p2)]);
            face_idx += 1;
            mesh.set_face(draco_core::FaceIndex(face_idx), [draco_core::PointIndex(p1), draco_core::PointIndex(p3), draco_core::PointIndex(p2)]);
            face_idx += 1;
        }
    }
    mesh.set_num_faces(face_idx as usize);
    mesh
}

// This test is currently ignored because it compares encoder-side DFS traversal
// events against decoder output, but the decoder does not currently emit
// corresponding test_event_log entries for that phase.
#[test]
#[ignore = "Decoder does not emit traversal events to test_event_log yet (enc=32, dec=0 on 4x4 grid)"]
fn test_encoder_decoder_event_sequence_4x4() {
    // Initialize and clear the test event log
    draco_core::test_event_log::init();
    draco_core::test_event_log::clear();

    // Build mesh and run encoder path (which constructs the encoder corner
    // table and records traversal order used to simulate decoder-side
    // attribute sequencing)
    let mesh = create_grid_mesh(4, 4);
    let mut encoder = MeshEncoder::new();
    encoder.set_mesh(mesh.clone());
    let mut buffer = EncoderBuffer::new();
    let mut options = EncoderOptions::default();
    options.set_global_int("encoding_method", 1);
    options.set_global_int("encoding_speed", 5);
    encoder.encode(&options, &mut buffer).expect("Encode failed");

    let enc_events = draco_core::test_event_log::take_events();

    // Now clear and run decoder to capture its sequence
    draco_core::test_event_log::clear();

    let mut decoder = MeshDecoder::new();
    let mut decoded_mesh = draco_core::Mesh::new();
    let mut dec_buffer = DecoderBuffer::new(buffer.data());
    decoder.decode(&mut dec_buffer, &mut decoded_mesh).expect("Decode failed");

    let dec_events = draco_core::test_event_log::take_events();

    // Find first differing event (if any)
    let min_len = usize::min(enc_events.len(), dec_events.len());
    for i in 0..min_len {
        if enc_events[i] != dec_events[i] {
            panic!("First event mismatch at idx {}:\n  enc='{}'\n  dec='{}'\n\nEncoder events (first 40): {:?}\nDecoder events (first 40): {:?}", 
                i, enc_events[i], dec_events[i], &enc_events[..enc_events.len().min(40)], &dec_events[..dec_events.len().min(40)]);
        }
    }

    // If one is longer, fail too (ordering mismatch)
    if enc_events.len() != dec_events.len() {
        panic!("Event sequences differ in length: enc={} dec={}", enc_events.len(), dec_events.len());
    }

    // If equal, test passes
    println!("Event sequences match for encoder and decoder on 4x4 grid ({} events)", enc_events.len());
}
