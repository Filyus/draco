// Debug test to compare C++ vs Rust parallelogram prediction
// This creates the exact same mesh as the Rust test_grid_encoding_parallelogram test
// and outputs detailed debug information.

#include <iostream>
#include <fstream>
#include <memory>
#include <vector>

#include "draco/draco_features.h"
#include "draco/mesh/mesh.h"
#include "draco/mesh/triangle_soup_mesh_builder.h"
#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/core/encoder_buffer.h"
#include "draco/core/decoder_buffer.h"

// Test: 5x5 grid = 25 vertices (matching Rust test_quantization_levels with small grid)
std::unique_ptr<draco::Mesh> CreateGridMesh(int width, int height) {
    draco::TriangleSoupMeshBuilder mb;
    
    // Calculate number of faces: (width-1) * (height-1) * 2
    int num_faces = (width - 1) * (height - 1) * 2;
    mb.Start(num_faces);
    
    // Add position attribute
    const int pos_att_id = mb.AddAttribute(draco::GeometryAttribute::POSITION, 
                                            3, draco::DT_FLOAT32);
    
    // Add faces
    int face_idx = 0;
    for (int y = 0; y < height - 1; ++y) {
        for (int x = 0; x < width - 1; ++x) {
            // Calculate vertex positions as floats
            float p0[3] = {static_cast<float>(x), static_cast<float>(y), 0.0f};
            float p1[3] = {static_cast<float>(x + 1), static_cast<float>(y), 0.0f};
            float p2[3] = {static_cast<float>(x), static_cast<float>(y + 1), 0.0f};
            float p3[3] = {static_cast<float>(x + 1), static_cast<float>(y + 1), 0.0f};
            
            // Triangle 1: p0, p1, p2
            mb.SetAttributeValuesForFace(pos_att_id, draco::FaceIndex(face_idx),
                p0, p1, p2);
            face_idx++;
            
            // Triangle 2: p1, p3, p2
            mb.SetAttributeValuesForFace(pos_att_id, draco::FaceIndex(face_idx),
                p1, p3, p2);
            face_idx++;
        }
    }
    
    return mb.Finalize();
}

int main() {
    std::cout << "=== C++ Parallelogram Debug Test ===" << std::endl;
    
    // Create 5x5 grid mesh (same as Rust test_quantization_levels)
    int width = 5;
    int height = 5;
    auto mesh = CreateGridMesh(width, height);
    
    if (!mesh) {
        std::cerr << "Failed to create mesh!" << std::endl;
        return 1;
    }
    
    std::cout << "Created mesh: " << mesh->num_points() << " points, " 
              << mesh->num_faces() << " faces" << std::endl;
    
    // Print original positions
    const draco::PointAttribute* pos_att = mesh->GetNamedAttribute(
        draco::GeometryAttribute::POSITION);
    if (pos_att) {
        std::cout << "\nOriginal positions:" << std::endl;
        for (draco::PointIndex pi(0); pi < mesh->num_points(); ++pi) {
            float pos[3];
            pos_att->GetValue(pos_att->mapped_index(pi), pos);
            std::cout << "  Point " << pi.value() << ": (" << pos[0] << ", " 
                      << pos[1] << ", " << pos[2] << ")" << std::endl;
        }
    }
    
    // Print faces
    std::cout << "\nFaces:" << std::endl;
    for (draco::FaceIndex fi(0); fi < mesh->num_faces(); ++fi) {
        auto face = mesh->face(fi);
        std::cout << "  Face " << fi.value() << ": [" << face[0].value() 
                  << ", " << face[1].value() << ", " << face[2].value() << "]" << std::endl;
    }
    
    // Encode with parallelogram prediction
    draco::Encoder encoder;
    encoder.SetEncodingMethod(draco::MESH_EDGEBREAKER_ENCODING);
    encoder.SetSpeedOptions(5, 5);  // Speed 5 should use parallelogram
    encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, 14);
    
    draco::EncoderBuffer encoder_buffer;
    auto status = encoder.EncodeMeshToBuffer(*mesh, &encoder_buffer);
    
    if (!status.ok()) {
        std::cerr << "Encoding failed: " << status.error_msg_string() << std::endl;
        return 1;
    }
    
    std::cout << "\nEncoded size: " << encoder_buffer.size() << " bytes" << std::endl;
    
    // Write encoded data for comparison
    {
        std::ofstream out("cpp_encoded_grid.drc", std::ios::binary);
        out.write(encoder_buffer.data(), encoder_buffer.size());
    }
    std::cout << "Wrote encoded file: cpp_encoded_grid.drc" << std::endl;
    
    // Decode
    draco::DecoderBuffer decoder_buffer;
    decoder_buffer.Init(encoder_buffer.data(), encoder_buffer.size());
    
    draco::Decoder decoder;
    auto decode_status = decoder.DecodeMeshFromBuffer(&decoder_buffer);
    
    if (!decode_status.ok()) {
        std::cerr << "Decoding failed: " << decode_status.status().error_msg_string() << std::endl;
        return 1;
    }
    
    auto decoded_mesh = std::move(decode_status).value();
    
    std::cout << "\nDecoded mesh: " << decoded_mesh->num_points() << " points, "
              << decoded_mesh->num_faces() << " faces" << std::endl;
    
    // Print decoded positions
    const draco::PointAttribute* dec_pos_att = decoded_mesh->GetNamedAttribute(
        draco::GeometryAttribute::POSITION);
    if (dec_pos_att) {
        std::cout << "\nDecoded positions:" << std::endl;
        for (draco::PointIndex pi(0); pi < decoded_mesh->num_points(); ++pi) {
            float pos[3];
            dec_pos_att->GetValue(dec_pos_att->mapped_index(pi), pos);
            std::cout << "  Point " << pi.value() << ": (" << pos[0] << ", " 
                      << pos[1] << ", " << pos[2] << ")" << std::endl;
        }
    }
    
    // Print decoded faces
    std::cout << "\nDecoded faces:" << std::endl;
    for (draco::FaceIndex fi(0); fi < std::min(decoded_mesh->num_faces(), (size_t)10); ++fi) {
        auto face = decoded_mesh->face(fi);
        std::cout << "  Face " << fi.value() << ": [" << face[0].value() 
                  << ", " << face[1].value() << ", " << face[2].value() << "]" << std::endl;
    }
    
    // Verify that all original grid positions are present in decoded mesh
    std::cout << "\n=== Verification ===" << std::endl;
    int missing = 0;
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            float target[3] = {static_cast<float>(x), static_cast<float>(y), 0.0f};
            
            bool found = false;
            for (draco::PointIndex pi(0); pi < decoded_mesh->num_points(); ++pi) {
                float pos[3];
                dec_pos_att->GetValue(dec_pos_att->mapped_index(pi), pos);
                
                // Allow small quantization error
                float dx = pos[0] - target[0];
                float dy = pos[1] - target[1];
                float dz = pos[2] - target[2];
                if (std::abs(dx) < 0.1f && std::abs(dy) < 0.1f && std::abs(dz) < 0.1f) {
                    found = true;
                    break;
                }
            }
            
            if (!found) {
                std::cout << "MISSING: Point (" << x << ", " << y << ", 0) not found in decoded mesh" << std::endl;
                missing++;
            }
        }
    }
    
    if (missing == 0) {
        std::cout << "SUCCESS: All " << (width * height) << " grid positions found in decoded mesh" << std::endl;
    } else {
        std::cout << "FAILED: " << missing << " grid positions missing" << std::endl;
    }
    
    return missing > 0 ? 1 : 0;
}
