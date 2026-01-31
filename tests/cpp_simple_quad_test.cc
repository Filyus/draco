// Simple 2-face quad test to debug encoder/decoder traversal mismatch
#include <iostream>
#include <memory>
#include "draco/mesh/mesh.h"
#include "draco/mesh/triangle_soup_mesh_builder.h"
#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/core/encoder_buffer.h"
#include "draco/core/decoder_buffer.h"

std::unique_ptr<draco::Mesh> CreateSimpleQuad() {
    draco::TriangleSoupMeshBuilder mb;
    mb.Start(2);  // 2 triangles
    
    const int pos_att_id = mb.AddAttribute(draco::GeometryAttribute::POSITION, 
                                            3, draco::DT_FLOAT32);
    
    // Simple quad: 
    // v2(0,1,0) --- v3(1,1,0)
    //    |     \      |
    //    |      \     |
    // v0(0,0,0) --- v1(1,0,0)
    //
    // Face 0: v0, v1, v2 (corners 0,1,2)
    // Face 1: v1, v3, v2 (corners 3,4,5)
    
    float v0[3] = {0.0f, 0.0f, 0.0f};
    float v1[3] = {1.0f, 0.0f, 0.0f};
    float v2[3] = {0.0f, 1.0f, 0.0f};
    float v3[3] = {1.0f, 1.0f, 0.0f};
    
    // Face 0: v0, v1, v2
    mb.SetAttributeValuesForFace(pos_att_id, draco::FaceIndex(0), v0, v1, v2);
    
    // Face 1: v1, v3, v2 
    mb.SetAttributeValuesForFace(pos_att_id, draco::FaceIndex(1), v1, v3, v2);
    
    return mb.Finalize();
}

int main() {
    std::cout << "=== C++ Simple Quad Test ===" << std::endl;
    
    auto mesh = CreateSimpleQuad();
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
    encoder.SetSpeedOptions(5, 5);
    encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, 11);
    
    draco::EncoderBuffer encoder_buffer;
    auto status = encoder.EncodeMeshToBuffer(*mesh, &encoder_buffer);
    
    if (!status.ok()) {
        std::cerr << "Encoding failed: " << status.error_msg() << std::endl;
        return 1;
    }
    
    std::cout << "\nEncoded size: " << encoder_buffer.size() << " bytes" << std::endl;
    
    // Decode
    draco::Decoder decoder;
    draco::DecoderBuffer decoder_buffer;
    decoder_buffer.Init(encoder_buffer.data(), encoder_buffer.size());
    
    auto dec_status = decoder.DecodeMeshFromBuffer(&decoder_buffer);
    if (!dec_status.ok()) {
        std::cerr << "Decoding failed: " << dec_status.status().error_msg() << std::endl;
        return 1;
    }
    
    auto dec_mesh = std::move(dec_status).value();
    
    std::cout << "\nDecoded mesh: " << dec_mesh->num_points() << " points, "
              << dec_mesh->num_faces() << " faces" << std::endl;
    
    // Print decoded positions
    const draco::PointAttribute* dec_pos_att = dec_mesh->GetNamedAttribute(
        draco::GeometryAttribute::POSITION);
    if (dec_pos_att) {
        std::cout << "\nDecoded positions:" << std::endl;
        for (draco::PointIndex pi(0); pi < dec_mesh->num_points(); ++pi) {
            float pos[3];
            dec_pos_att->GetValue(dec_pos_att->mapped_index(pi), pos);
            std::cout << "  Point " << pi.value() << ": (" << pos[0] << ", " 
                      << pos[1] << ", " << pos[2] << ")" << std::endl;
        }
    }
    
    // Verify
    std::cout << "\n=== Verification ===" << std::endl;
    bool all_match = true;
    float tolerance = 0.001f;
    
    float expected[4][3] = {
        {0.0f, 0.0f, 0.0f},
        {1.0f, 0.0f, 0.0f},
        {0.0f, 1.0f, 0.0f},
        {1.0f, 1.0f, 0.0f}
    };
    
    for (int i = 0; i < 4; i++) {
        bool found = false;
        for (draco::PointIndex pi(0); pi < dec_mesh->num_points(); ++pi) {
            float pos[3];
            dec_pos_att->GetValue(dec_pos_att->mapped_index(pi), pos);
            if (std::abs(pos[0] - expected[i][0]) < tolerance &&
                std::abs(pos[1] - expected[i][1]) < tolerance &&
                std::abs(pos[2] - expected[i][2]) < tolerance) {
                found = true;
                break;
            }
        }
        if (!found) {
            std::cout << "MISSING: Expected position (" << expected[i][0] << ", "
                      << expected[i][1] << ", " << expected[i][2] << ")" << std::endl;
            all_match = false;
        }
    }
    
    if (all_match) {
        std::cout << "SUCCESS: All positions found in decoded mesh" << std::endl;
    }
    
    return all_match ? 0 : 1;
}
