fn main() {
    let data = std::fs::read(r"D:\Projects\Draco\testdata\cube_att.drc").unwrap();
    println!("File size: {}", data.len());
    match draco_cpp_ffi::profile_cpp_decode(&data, 1) {
        Some(r) => println!(
            "C++ decode OK: {} pts, {} faces, {} us",
            r.num_points, r.num_faces, r.decode_time_us
        ),
        None => println!("C++ decode FAILED"),
    }
}
