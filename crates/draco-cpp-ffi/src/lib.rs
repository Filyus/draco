//! FFI bindings to the C++ Draco library for performance comparison
//! 
//! This crate provides direct FFI access to the original C++ Draco encoder
//! for accurate performance benchmarking against the Rust implementation.

#[cfg(not(draco_ffi_disabled))]
mod ffi {
    use std::os::raw::c_int;
    
    /// Profiling result structure from C++
    #[repr(C)]
    pub struct DracoProfileResult {
        pub mesh_setup_us: i64,
        pub encoder_setup_us: i64,
        pub encode_time_us: i64,
        pub total_time_us: i64,
        pub output_size: usize,
    }
    
    /// Decode profiling result structure from C++
    #[repr(C)]
    pub struct DracoDecodeProfileResult {
        pub decode_time_us: i64,
        pub num_points: u32,
        pub num_faces: u32,
    }
    
    extern "C" {
        /// Benchmark encoding: runs encoding multiple times and returns average time in microseconds
        pub fn draco_benchmark_encode_mesh(
            num_points: u32,
            positions: *const f32,
            num_faces: u32,
            faces: *const u32,
            encoding_speed: c_int,
            decoding_speed: c_int,
            quantization_bits: c_int,
            iterations: u32,
            output_size: *mut usize,
        ) -> i64;
        
        /// Single-shot encoding that returns encoded bytes
        pub fn draco_encode_mesh_single(
            num_points: u32,
            positions: *const f32,
            num_faces: u32,
            faces: *const u32,
            encoding_speed: c_int,
            decoding_speed: c_int,
            quantization_bits: c_int,
            output_buffer: *mut u8,
            output_buffer_size: usize,
        ) -> usize;

        /// Create/free mesh handles
        pub fn draco_create_mesh() -> *mut ::std::ffi::c_void;
        pub fn draco_free_mesh(handle: *mut ::std::ffi::c_void);

        /// Mesh setup helpers
        pub fn draco_mesh_set_num_faces(handle: *mut ::std::ffi::c_void, num_faces: u32);
        pub fn draco_mesh_set_face(handle: *mut ::std::ffi::c_void, face_idx: u32, v0: u32, v1: u32, v2: u32);
        pub fn draco_mesh_add_position_attribute(handle: *mut ::std::ffi::c_void, num_points: u32, positions: *const f32) -> c_int;

        /// Encoder buffer helpers
        pub fn draco_create_encoder_buffer() -> *mut ::std::ffi::c_void;
        pub fn draco_free_encoder_buffer(handle: *mut ::std::ffi::c_void);
        pub fn draco_encoder_buffer_data(handle: *mut ::std::ffi::c_void) -> *const u8;
        pub fn draco_encoder_buffer_size(handle: *mut ::std::ffi::c_void) -> usize;

        /// Encode using handles (mesh -> encoder buffer)
        pub fn draco_encode_mesh(
            mesh_handle: *mut ::std::ffi::c_void,
            buffer_handle: *mut ::std::ffi::c_void,
            encoding_speed: c_int,
            decoding_speed: c_int,
            quantization_bits: c_int,
        ) -> i64;

        /// Get version info for verification
        pub fn draco_get_version(
            major: *mut c_int,
            minor: *mut c_int,
            revision: *mut c_int,
        );
        
        /// Detailed profiling of encoding stages
        pub fn draco_profile_encode(
            num_points: u32,
            positions: *const f32,
            num_faces: u32,
            faces: *const u32,
            encoding_speed: c_int,
            decoding_speed: c_int,
            quantization_bits: c_int,
            iterations: u32,
            result: *mut DracoProfileResult,
        ) -> c_int;
        
        /// Benchmark decoding
        pub fn draco_benchmark_decode_mesh(
            encoded_data: *const u8,
            encoded_size: usize,
            iterations: u32,
            out_num_points: *mut u32,
            out_num_faces: *mut u32,
        ) -> i64;
        
        /// Profile decoding with detailed timing
        pub fn draco_profile_decode(
            encoded_data: *const u8,
            encoded_size: usize,
            iterations: u32,
            result: *mut DracoDecodeProfileResult,
        ) -> c_int;
    }
}

#[cfg(not(draco_ffi_disabled))]
pub use ffi::*;

/// Detailed profiling result from C++ encoder
#[derive(Debug, Clone)]
pub struct CppProfileResult {
    pub mesh_setup_us: i64,
    pub encoder_setup_us: i64,
    pub encode_time_us: i64,
    pub total_time_us: i64,
    pub output_size: usize,
}

/// Check if FFI is available
pub fn is_available() -> bool {
    #[cfg(draco_ffi_disabled)]
    return false;
    
    #[cfg(not(draco_ffi_disabled))]
    return true;
}

/// Get Draco C++ library version
#[cfg(not(draco_ffi_disabled))]
pub fn get_version() -> (i32, i32, i32) {
    let mut major = 0;
    let mut minor = 0;
    let mut revision = 0;
    unsafe {
        draco_get_version(&mut major, &mut minor, &mut revision);
    }
    (major, minor, revision)
}

#[cfg(draco_ffi_disabled)]
pub fn get_version() -> (i32, i32, i32) {
    (0, 0, 0)
}

/// Benchmark result from C++ encoder
pub struct CppBenchmarkResult {
    pub avg_time_us: i64,
    pub output_size: usize,
}

/// Benchmark the C++ encoder with given mesh data
/// 
/// # Arguments
/// * `positions` - Flat array of f32 positions (num_points * 3 values)
/// * `faces` - Flat array of face indices (num_faces * 3 values)
/// * `encoding_speed` - Encoding speed (0 = best compression, 10 = fastest)
/// * `decoding_speed` - Decoding speed (0 = best compression, 10 = fastest)
/// * `quantization_bits` - Quantization bits for position attribute
/// * `iterations` - Number of iterations to average
/// 
/// # Returns
/// * `Some(CppBenchmarkResult)` if FFI is available and encoding succeeded
/// * `None` if FFI is disabled or encoding failed
#[cfg(not(draco_ffi_disabled))]
pub fn benchmark_cpp_encode(
    positions: &[f32],
    faces: &[u32],
    encoding_speed: i32,
    decoding_speed: i32,
    quantization_bits: i32,
    iterations: u32,
) -> Option<CppBenchmarkResult> {
    let num_points = (positions.len() / 3) as u32;
    let num_faces = (faces.len() / 3) as u32;
    
    let mut output_size: usize = 0;
    
    let avg_time_us = unsafe {
        draco_benchmark_encode_mesh(
            num_points,
            positions.as_ptr(),
            num_faces,
            faces.as_ptr(),
            encoding_speed,
            decoding_speed,
            quantization_bits,
            iterations,
            &mut output_size,
        )
    };
    
    if avg_time_us < 0 {
        return None;
    }
    
    Some(CppBenchmarkResult {
        avg_time_us,
        output_size,
    })
}

#[cfg(draco_ffi_disabled)]
pub fn benchmark_cpp_encode(
    _positions: &[f32],
    _faces: &[u32],
    _encoding_speed: i32,
    _decoding_speed: i32,
    _quantization_bits: i32,
    _iterations: u32,
) -> Option<CppBenchmarkResult> {
    None
}

/// Encode a mesh using C++ Draco and return the encoded bytes
#[cfg(not(draco_ffi_disabled))]
pub fn encode_cpp_mesh(
    positions: &[f32],
    faces: &[u32],
    encoding_speed: i32,
    decoding_speed: i32,
    quantization_bits: i32,
) -> Option<Vec<u8>> {
    let num_points = (positions.len() / 3) as u32;
    let num_faces = (faces.len() / 3) as u32;
    
    // Allocate a large enough buffer (estimate: 10 bytes per vertex + overhead)
    let buffer_size = (num_points as usize * 10 + 1024).max(65536);
    let mut buffer = vec![0u8; buffer_size];
    
    let encoded_size = unsafe {
        ffi::draco_encode_mesh_single(
            num_points,
            positions.as_ptr(),
            num_faces,
            faces.as_ptr(),
            encoding_speed,
            decoding_speed,
            quantization_bits,
            buffer.as_mut_ptr(),
            buffer_size,
        )
    };
    
    if encoded_size == 0 {
        return None;
    }
    
    buffer.truncate(encoded_size);
    Some(buffer)
}

#[cfg(draco_ffi_disabled)]
pub fn encode_cpp_mesh(
    _positions: &[f32],
    _faces: &[u32],
    _encoding_speed: i32,
    _decoding_speed: i32,
    _quantization_bits: i32,
) -> Option<Vec<u8>> {
    None
}

/// Profile C++ encoding with detailed timing breakdown
#[cfg(not(draco_ffi_disabled))]
pub fn profile_cpp_encode(
    positions: &[f32],
    faces: &[u32],
    encoding_speed: i32,
    decoding_speed: i32,
    quantization_bits: i32,
    iterations: u32,
) -> Option<CppProfileResult> {
    let num_points = (positions.len() / 3) as u32;
    let num_faces = (faces.len() / 3) as u32;
    
    let mut result = ffi::DracoProfileResult {
        mesh_setup_us: 0,
        encoder_setup_us: 0,
        encode_time_us: 0,
        total_time_us: 0,
        output_size: 0,
    };
    
    let status = unsafe {
        ffi::draco_profile_encode(
            num_points,
            positions.as_ptr(),
            num_faces,
            faces.as_ptr(),
            encoding_speed,
            decoding_speed,
            quantization_bits,
            iterations,
            &mut result,
        )
    };
    
    if status != 0 {
        return None;
    }
    
    Some(CppProfileResult {
        mesh_setup_us: result.mesh_setup_us,
        encoder_setup_us: result.encoder_setup_us,
        encode_time_us: result.encode_time_us,
        total_time_us: result.total_time_us,
        output_size: result.output_size,
    })
}

#[cfg(draco_ffi_disabled)]
pub fn profile_cpp_encode(
    _positions: &[f32],
    _faces: &[u32],
    _encoding_speed: i32,
    _decoding_speed: i32,
    _quantization_bits: i32,
    _iterations: u32,
) -> Option<CppProfileResult> {
    None
}

/// Decode profiling result from C++
#[derive(Debug, Clone)]
pub struct CppDecodeProfileResult {
    pub decode_time_us: i64,
    pub num_points: u32,
    pub num_faces: u32,
}

/// Profile C++ decoding
#[cfg(not(draco_ffi_disabled))]
pub fn profile_cpp_decode(
    encoded_data: &[u8],
    iterations: u32,
) -> Option<CppDecodeProfileResult> {
    let mut result = ffi::DracoDecodeProfileResult {
        decode_time_us: 0,
        num_points: 0,
        num_faces: 0,
    };
    
    let status = unsafe {
        ffi::draco_profile_decode(
            encoded_data.as_ptr(),
            encoded_data.len(),
            iterations,
            &mut result,
        )
    };
    
    if status != 0 {
        return None;
    }
    
    Some(CppDecodeProfileResult {
        decode_time_us: result.decode_time_us,
        num_points: result.num_points,
        num_faces: result.num_faces,
    })
}

#[cfg(draco_ffi_disabled)]
pub fn profile_cpp_decode(
    _encoded_data: &[u8],
    _iterations: u32,
) -> Option<CppDecodeProfileResult> {
    None
}

/// Benchmark C++ decoding via the Rust wrapper (returns avg time and output sizes)
#[cfg(not(draco_ffi_disabled))]
pub fn benchmark_cpp_decode(
    encoded_data: &[u8],
    iterations: u32,
) -> Option<(i64, u32, u32)> {
    let mut out_num_points = 0u32;
    let mut out_num_faces = 0u32;
    // draco_benchmark_decode_mesh returns total_time / iterations (the per-iteration average in µs)
    let avg_us = unsafe {
        ffi::draco_benchmark_decode_mesh(
            encoded_data.as_ptr(),
            encoded_data.len(),
            iterations,
            &mut out_num_points as *mut u32,
            &mut out_num_faces as *mut u32,
        )
    };

    if avg_us < 0 {
        None
    } else {
        Some((avg_us, out_num_points, out_num_faces))
    }
}

#[cfg(draco_ffi_disabled)]
pub fn benchmark_cpp_decode(
    _encoded_data: &[u8],
    _iterations: u32,
) -> Option<(i64, u32, u32)> { None }

// --- Safe RAII wrappers for C++ handles -------------------------------------------------

#[cfg(not(draco_ffi_disabled))]
/// RAII wrapper around a C++ Mesh handle
pub struct CppMesh {
    handle: *mut ::std::ffi::c_void,
}

#[cfg(not(draco_ffi_disabled))]
impl CppMesh {
    pub fn new() -> Option<Self> {
        let h = unsafe { draco_create_mesh() };
        if h.is_null() { None } else { Some(CppMesh { handle: h }) }
    }

    pub fn set_num_faces(&mut self, num_faces: u32) {
        unsafe { draco_mesh_set_num_faces(self.handle, num_faces) }
    }

    pub fn set_face(&mut self, face_idx: u32, v0: u32, v1: u32, v2: u32) {
        unsafe { draco_mesh_set_face(self.handle, face_idx, v0, v1, v2) }
    }

    pub fn add_position_attribute(&mut self, num_points: u32, positions: &[f32]) -> Option<i32> {
        let ret = unsafe { draco_mesh_add_position_attribute(self.handle, num_points, positions.as_ptr()) };
        if ret < 0 { None } else { Some(ret as i32) }
    }
}

#[cfg(not(draco_ffi_disabled))]
impl Drop for CppMesh {
    fn drop(&mut self) { unsafe { draco_free_mesh(self.handle) } }
}

#[cfg(draco_ffi_disabled)]
/// Stub when FFI is disabled
pub struct CppMesh;

#[cfg(draco_ffi_disabled)]
impl CppMesh {
    pub fn new() -> Option<Self> { None }
}

#[cfg(not(draco_ffi_disabled))]
/// RAII wrapper around a C++ EncoderBuffer handle
pub struct CppEncoderBuffer { handle: *mut ::std::ffi::c_void }

#[cfg(not(draco_ffi_disabled))]
impl CppEncoderBuffer {
    pub fn new() -> Option<Self> {
        let h = unsafe { draco_create_encoder_buffer() };
        if h.is_null() { None } else { Some(CppEncoderBuffer { handle: h }) }
    }

    pub fn data(&self) -> &[u8] {
        unsafe {
            let ptr = draco_encoder_buffer_data(self.handle);
            let len = draco_encoder_buffer_size(self.handle);
            std::slice::from_raw_parts(ptr, len)
        }
    }
}

#[cfg(not(draco_ffi_disabled))]
impl Drop for CppEncoderBuffer { fn drop(&mut self) { unsafe { draco_free_encoder_buffer(self.handle) } } }

#[cfg(draco_ffi_disabled)]
pub struct CppEncoderBuffer;

#[cfg(draco_ffi_disabled)]
impl CppEncoderBuffer { pub fn new() -> Option<Self> { None } }

/// Encode using the C++ handle-based API and return encoded bytes
#[cfg(not(draco_ffi_disabled))]
pub fn encode_with_handles(
    mesh: &CppMesh,
    encoding_speed: i32,
    decoding_speed: i32,
    quantization_bits: i32,
) -> Option<Vec<u8>> {
    let buffer = CppEncoderBuffer::new()?;
    let status = unsafe { draco_encode_mesh(mesh.handle, buffer.handle, encoding_speed, decoding_speed, quantization_bits) };
    if status < 0 { return None; }
    Some(buffer.data().to_vec())
}

#[cfg(draco_ffi_disabled)]
pub fn encode_with_handles(
    _mesh: &CppMesh,
    _encoding_speed: i32,
    _decoding_speed: i32,
    _quantization_bits: i32,
) -> Option<Vec<u8>> { None }

// --------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffi_available() {
        if is_available() {
            let (major, minor, revision) = get_version();
            println!("Draco C++ version: {}.{}.{}", major, minor, revision);
        } else {
            println!("Draco FFI is disabled");
        }
    }
}
