// C API wrapper for Draco C++ library for FFI benchmarking
// This provides a simple C interface to the C++ Draco encoder/decoder

#include <cstring>
#include <cstdint>
#include <chrono>

#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/mesh/mesh.h"
#include "draco/mesh/triangle_soup_mesh_builder.h"
#include "draco/point_cloud/point_cloud.h"
#include "draco/core/encoder_buffer.h"
#include "draco/core/decoder_buffer.h"

extern "C" {

// Opaque handle types
typedef void* DracoMeshHandle;
typedef void* DracoEncoderBufferHandle;

// Create a new mesh
DracoMeshHandle draco_create_mesh() {
    return new draco::Mesh();
}

// Free a mesh
void draco_free_mesh(DracoMeshHandle handle) {
    delete static_cast<draco::Mesh*>(handle);
}

// Set mesh face count
void draco_mesh_set_num_faces(DracoMeshHandle handle, uint32_t num_faces) {
    auto* mesh = static_cast<draco::Mesh*>(handle);
    mesh->SetNumFaces(num_faces);
}

// Add a face to the mesh
void draco_mesh_set_face(DracoMeshHandle handle, uint32_t face_idx, uint32_t v0, uint32_t v1, uint32_t v2) {
    auto* mesh = static_cast<draco::Mesh*>(handle);
    draco::Mesh::Face face;
    face[0] = draco::PointIndex(v0);
    face[1] = draco::PointIndex(v1);
    face[2] = draco::PointIndex(v2);
    mesh->SetFace(draco::FaceIndex(face_idx), face);
}

// Set number of points and add position attribute
int draco_mesh_add_position_attribute(DracoMeshHandle handle, uint32_t num_points, const float* positions) {
    auto* mesh = static_cast<draco::Mesh*>(handle);

    // Create a GeometryAttribute with explicit stride/offset to match single-shot construction
    draco::GeometryAttribute ga;
    ga.Init(draco::GeometryAttribute::POSITION, nullptr, 3, draco::DT_FLOAT32, false, sizeof(float) * 3, 0);

    int pos_att_id = mesh->AddAttribute(ga, true, num_points);
    if (pos_att_id < 0) return -1;
    draco::PointAttribute* pos_att = mesh->attribute(pos_att_id);

    for (uint32_t i = 0; i < num_points; ++i) {
        pos_att->SetAttributeValue(draco::AttributeValueIndex(i), &positions[i * 3]);
    }

    mesh->set_num_points(num_points);
    return pos_att_id;
}

// Create encoder buffer
DracoEncoderBufferHandle draco_create_encoder_buffer() {
    return new draco::EncoderBuffer();
}

// Free encoder buffer
void draco_free_encoder_buffer(DracoEncoderBufferHandle handle) {
    delete static_cast<draco::EncoderBuffer*>(handle);
}

// Get encoded data pointer and size
const uint8_t* draco_encoder_buffer_data(DracoEncoderBufferHandle handle) {
    auto* buffer = static_cast<draco::EncoderBuffer*>(handle);
    return reinterpret_cast<const uint8_t*>(buffer->data());
}

size_t draco_encoder_buffer_size(DracoEncoderBufferHandle handle) {
    auto* buffer = static_cast<draco::EncoderBuffer*>(handle);
    return buffer->size();
}

// Encode mesh with given speed and quantization settings
// Returns encoding time in microseconds, or -1 on error
int64_t draco_encode_mesh(
    DracoMeshHandle mesh_handle,
    DracoEncoderBufferHandle buffer_handle,
    int encoding_speed,
    int decoding_speed,
    int quantization_bits
) {
    auto* mesh = static_cast<draco::Mesh*>(mesh_handle);
    auto* buffer = static_cast<draco::EncoderBuffer*>(buffer_handle);
    
    draco::Encoder encoder;
    encoder.SetSpeedOptions(encoding_speed, decoding_speed);
    encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, quantization_bits);
    // Don't set encoding method - let C++ use default (sequential at speed 10, edgebreaker otherwise)
    
    auto start = std::chrono::high_resolution_clock::now();
    draco::Status status = encoder.EncodeMeshToBuffer(*mesh, buffer);
    auto end = std::chrono::high_resolution_clock::now();
    
    if (!status.ok()) {
        return -1;
    }
    
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
    return duration.count();
}

// Benchmark encoding: runs encoding multiple times and returns average time in microseconds
// Uses direct mesh construction to match Rust's mesh structure exactly
int64_t draco_benchmark_encode_mesh(
    uint32_t num_points,
    const float* positions,
    uint32_t num_faces,
    const uint32_t* faces,  // Each face is 3 consecutive indices
    int encoding_speed,
    int decoding_speed,
    int quantization_bits,
    uint32_t iterations,
    size_t* output_size  // Output: encoded size in bytes
) {
    int64_t total_time = 0;
    *output_size = 0;
    
    for (uint32_t iter = 0; iter < iterations; ++iter) {
        // Create mesh directly (matching Rust's approach)
        draco::Mesh mesh;
        mesh.set_num_points(num_points);
        mesh.SetNumFaces(num_faces);
        
        // Create position attribute with explicit identity mapping
        draco::GeometryAttribute ga;
        ga.Init(draco::GeometryAttribute::POSITION, nullptr, 3, draco::DT_FLOAT32, 
                false, sizeof(float) * 3, 0);
        
        // AddAttribute with identity_mapping = true creates proper identity mapped attribute
        int pos_att_id = mesh.AddAttribute(ga, true, num_points);
        draco::PointAttribute* pos_att = mesh.attribute(pos_att_id);
        
        // Set attribute values
        for (uint32_t i = 0; i < num_points; ++i) {
            pos_att->SetAttributeValue(draco::AttributeValueIndex(i), &positions[i * 3]);
        }
        
        // Set faces
        for (uint32_t i = 0; i < num_faces; ++i) {
            draco::Mesh::Face face;
            face[0] = draco::PointIndex(faces[i * 3]);
            face[1] = draco::PointIndex(faces[i * 3 + 1]);
            face[2] = draco::PointIndex(faces[i * 3 + 2]);
            mesh.SetFace(draco::FaceIndex(i), face);
        }
        
        // Setup encoder
        draco::Encoder encoder;
        encoder.SetSpeedOptions(encoding_speed, decoding_speed);
        encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, quantization_bits);
        
        draco::EncoderBuffer buffer;
        
        // Time just the encoding
        auto start = std::chrono::high_resolution_clock::now();
        draco::Status status = encoder.EncodeMeshToBuffer(mesh, &buffer);
        auto end = std::chrono::high_resolution_clock::now();
        
        if (!status.ok()) {
            return -1;
        }
        
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
        total_time += duration.count();
        *output_size = buffer.size();
    }
    
    return total_time / iterations;
}

// Get version info for verification
void draco_get_version(int* major, int* minor, int* revision) {
    // Draco version from CMakeLists.txt
    *major = 1;
    *minor = 5;
    *revision = 7;
}

// Profiling result structure
struct DracoProfileResult {
    int64_t mesh_setup_us;      // Time to create mesh and set attributes
    int64_t encoder_setup_us;   // Time to create and configure encoder
    int64_t encode_time_us;     // Time for actual encoding
    int64_t total_time_us;      // Total time including mesh setup
    size_t output_size;
};

// Detailed profiling of encoding stages
// Returns 0 on success, -1 on error
int draco_profile_encode(
    uint32_t num_points,
    const float* positions,
    uint32_t num_faces,
    const uint32_t* faces,
    int encoding_speed,
    int decoding_speed,
    int quantization_bits,
    uint32_t iterations,
    DracoProfileResult* result
) {
    int64_t total_mesh_setup = 0;
    int64_t total_encoder_setup = 0;
    int64_t total_encode = 0;
    int64_t total_all = 0;
    
    for (uint32_t iter = 0; iter < iterations; ++iter) {
        auto all_start = std::chrono::high_resolution_clock::now();
        
        // === MESH SETUP ===
        auto mesh_start = std::chrono::high_resolution_clock::now();
        
        draco::Mesh mesh;
        mesh.set_num_points(num_points);
        mesh.SetNumFaces(num_faces);
        
        draco::GeometryAttribute ga;
        ga.Init(draco::GeometryAttribute::POSITION, nullptr, 3, draco::DT_FLOAT32, 
                false, sizeof(float) * 3, 0);
        
        int pos_att_id = mesh.AddAttribute(ga, true, num_points);
        draco::PointAttribute* pos_att = mesh.attribute(pos_att_id);
        
        for (uint32_t i = 0; i < num_points; ++i) {
            pos_att->SetAttributeValue(draco::AttributeValueIndex(i), &positions[i * 3]);
        }
        
        for (uint32_t i = 0; i < num_faces; ++i) {
            draco::Mesh::Face face;
            face[0] = draco::PointIndex(faces[i * 3]);
            face[1] = draco::PointIndex(faces[i * 3 + 1]);
            face[2] = draco::PointIndex(faces[i * 3 + 2]);
            mesh.SetFace(draco::FaceIndex(i), face);
        }
        
        auto mesh_end = std::chrono::high_resolution_clock::now();
        
        // === ENCODER SETUP ===
        auto encoder_start = std::chrono::high_resolution_clock::now();
        
        draco::Encoder encoder;
        encoder.SetSpeedOptions(encoding_speed, decoding_speed);
        encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, quantization_bits);
        draco::EncoderBuffer buffer;
        
        auto encoder_end = std::chrono::high_resolution_clock::now();
        
        // === ENCODING ===
        auto encode_start = std::chrono::high_resolution_clock::now();
        draco::Status status = encoder.EncodeMeshToBuffer(mesh, &buffer);
        auto encode_end = std::chrono::high_resolution_clock::now();
        
        if (!status.ok()) {
            return -1;
        }
        
        auto all_end = std::chrono::high_resolution_clock::now();
        
        total_mesh_setup += std::chrono::duration_cast<std::chrono::microseconds>(mesh_end - mesh_start).count();
        total_encoder_setup += std::chrono::duration_cast<std::chrono::microseconds>(encoder_end - encoder_start).count();
        total_encode += std::chrono::duration_cast<std::chrono::microseconds>(encode_end - encode_start).count();
        total_all += std::chrono::duration_cast<std::chrono::microseconds>(all_end - all_start).count();
        
        result->output_size = buffer.size();
    }
    
    result->mesh_setup_us = total_mesh_setup / iterations;
    result->encoder_setup_us = total_encoder_setup / iterations;
    result->encode_time_us = total_encode / iterations;
    result->total_time_us = total_all / iterations;
    
    return 0;
}

// Single-shot encoding that returns encoded data for byte comparison
// Uses direct mesh construction to match Rust's mesh structure
// Returns encoded size, or 0 on error. Caller provides output buffer.
size_t draco_encode_mesh_single(
    uint32_t num_points,
    const float* positions,
    uint32_t num_faces,
    const uint32_t* faces,
    int encoding_speed,
    int decoding_speed,
    int quantization_bits,
    uint8_t* output_buffer,
    size_t output_buffer_size
) {
    // Create mesh directly (matching Rust's approach)
    draco::Mesh mesh;
    mesh.set_num_points(num_points);
    mesh.SetNumFaces(num_faces);
    
    // Create position attribute with explicit identity mapping
    draco::GeometryAttribute ga;
    ga.Init(draco::GeometryAttribute::POSITION, nullptr, 3, draco::DT_FLOAT32, 
            false, sizeof(float) * 3, 0);
    
    // AddAttribute with identity_mapping = true creates proper identity mapped attribute
    int pos_att_id = mesh.AddAttribute(ga, true, num_points);
    draco::PointAttribute* pos_att = mesh.attribute(pos_att_id);
    
    // Set attribute values
    for (uint32_t i = 0; i < num_points; ++i) {
        pos_att->SetAttributeValue(draco::AttributeValueIndex(i), &positions[i * 3]);
    }
    
    // Set faces
    for (uint32_t i = 0; i < num_faces; ++i) {
        draco::Mesh::Face face;
        face[0] = draco::PointIndex(faces[i * 3]);
        face[1] = draco::PointIndex(faces[i * 3 + 1]);
        face[2] = draco::PointIndex(faces[i * 3 + 2]);
        mesh.SetFace(draco::FaceIndex(i), face);
    }
    
    // Setup encoder
    draco::Encoder encoder;
    encoder.SetSpeedOptions(encoding_speed, decoding_speed);
    encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, quantization_bits);
    
    draco::EncoderBuffer buffer;
    draco::Status status = encoder.EncodeMeshToBuffer(mesh, &buffer);
    
    if (!status.ok()) {
        return 0;
    }
    
    size_t encoded_size = buffer.size();
    if (encoded_size > output_buffer_size) {
        return 0;  // Buffer too small
    }
    
    std::memcpy(output_buffer, buffer.data(), encoded_size);
    return encoded_size;
}

// Decode profiling result structure
struct DracoDecodeProfileResult {
    int64_t decode_time_us;
    uint32_t num_points;
    uint32_t num_faces;
};

// Benchmark decoding: runs decoding multiple times and returns average time in microseconds
int64_t draco_benchmark_decode_mesh(
    const uint8_t* encoded_data,
    size_t encoded_size,
    uint32_t iterations,
    uint32_t* out_num_points,
    uint32_t* out_num_faces
) {
    int64_t total_time = 0;
    
    for (uint32_t iter = 0; iter < iterations; ++iter) {
        draco::DecoderBuffer buffer;
        buffer.Init(reinterpret_cast<const char*>(encoded_data), encoded_size);
        
        draco::Decoder decoder;
        
        auto start = std::chrono::high_resolution_clock::now();
        auto result = decoder.DecodeMeshFromBuffer(&buffer);
        auto end = std::chrono::high_resolution_clock::now();
        
        if (!result.ok()) {
            return -1;
        }
        
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
        total_time += duration.count();
        
        auto mesh = std::move(result).value();
        *out_num_points = mesh->num_points();
        *out_num_faces = mesh->num_faces();
    }
    
    return total_time / iterations;
}

// Profile decoding with detailed timing
int draco_profile_decode(
    const uint8_t* encoded_data,
    size_t encoded_size,
    uint32_t iterations,
    DracoDecodeProfileResult* result
) {
    int64_t total_decode = 0;
    
    for (uint32_t iter = 0; iter < iterations; ++iter) {
        draco::DecoderBuffer buffer;
        buffer.Init(reinterpret_cast<const char*>(encoded_data), encoded_size);
        
        draco::Decoder decoder;
        
        auto start = std::chrono::high_resolution_clock::now();
        auto decode_result = decoder.DecodeMeshFromBuffer(&buffer);
        auto end = std::chrono::high_resolution_clock::now();
        
        if (!decode_result.ok()) {
            return -1;
        }
        
        total_decode += std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
        
        auto mesh = std::move(decode_result).value();
        result->num_points = mesh->num_points();
        result->num_faces = mesh->num_faces();
    }
    
    result->decode_time_us = total_decode / iterations;
    return 0;
}

} // extern "C"
