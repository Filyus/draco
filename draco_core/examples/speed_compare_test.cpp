// Speed comparison test for Draco encoding
// Compares encoding at different speeds to identify prediction scheme behavior

#include "draco/core/status.h"
#include "draco/mesh/mesh.h"
#include "draco/mesh/triangle_soup_mesh_builder.h"
#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/compression/expert_encode.h"

#include <iostream>
#include <iomanip>
#include <fstream>
#include <vector>
#include <cmath>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

using namespace draco;

// Create a simple test mesh (a cube with some complexity)
std::unique_ptr<Mesh> CreateTestMesh() {
    TriangleSoupMeshBuilder builder;
    builder.Start(12); // 12 triangles for a cube
    
    const int pos_att_id = builder.AddAttribute(GeometryAttribute::POSITION, 3, DT_FLOAT32);
    
    // Cube vertices
    float vertices[8][3] = {
        {0.0f, 0.0f, 0.0f},
        {1.0f, 0.0f, 0.0f},
        {1.0f, 1.0f, 0.0f},
        {0.0f, 1.0f, 0.0f},
        {0.0f, 0.0f, 1.0f},
        {1.0f, 0.0f, 1.0f},
        {1.0f, 1.0f, 1.0f},
        {0.0f, 1.0f, 1.0f}
    };
    
    // Cube faces (12 triangles, 2 per face)
    int faces[12][3] = {
        {0, 1, 2}, {0, 2, 3}, // front
        {4, 6, 5}, {4, 7, 6}, // back
        {0, 4, 5}, {0, 5, 1}, // bottom
        {2, 6, 7}, {2, 7, 3}, // top
        {0, 3, 7}, {0, 7, 4}, // left
        {1, 5, 6}, {1, 6, 2}  // right
    };
    
    for (int i = 0; i < 12; ++i) {
        builder.SetAttributeValuesForFace(pos_att_id, FaceIndex(i),
            vertices[faces[i][0]],
            vertices[faces[i][1]],
            vertices[faces[i][2]]);
    }
    
    return builder.Finalize();
}

// Create a more complex mesh (sphere approximation)
std::unique_ptr<Mesh> CreateSphereMesh(int segments = 16, int rings = 8) {
    std::vector<std::array<float, 3>> vertices;
    std::vector<std::array<int, 3>> faces;
    
    // Create vertices
    for (int r = 0; r <= rings; ++r) {
        float phi = M_PI * r / rings;
        for (int s = 0; s < segments; ++s) {
            float theta = 2.0f * M_PI * s / segments;
            float x = sin(phi) * cos(theta);
            float y = cos(phi);
            float z = sin(phi) * sin(theta);
            vertices.push_back({x, y, z});
        }
    }
    
    // Create faces
    for (int r = 0; r < rings; ++r) {
        for (int s = 0; s < segments; ++s) {
            int next_s = (s + 1) % segments;
            int v0 = r * segments + s;
            int v1 = r * segments + next_s;
            int v2 = (r + 1) * segments + next_s;
            int v3 = (r + 1) * segments + s;
            
            faces.push_back({v0, v1, v2});
            faces.push_back({v0, v2, v3});
        }
    }
    
    TriangleSoupMeshBuilder builder;
    builder.Start(faces.size());
    
    const int pos_att_id = builder.AddAttribute(GeometryAttribute::POSITION, 3, DT_FLOAT32);
    
    for (size_t i = 0; i < faces.size(); ++i) {
        builder.SetAttributeValuesForFace(pos_att_id, FaceIndex(i),
            vertices[faces[i][0]].data(),
            vertices[faces[i][1]].data(),
            vertices[faces[i][2]].data());
    }
    
    return builder.Finalize();
}

void TestEncodingSpeed(Mesh* mesh, int speed, const std::string& name) {
    Encoder encoder;
    encoder.SetSpeedOptions(speed, speed);
    encoder.SetAttributeQuantization(GeometryAttribute::POSITION, 14);
    encoder.SetEncodingMethod(MESH_EDGEBREAKER_ENCODING);
    
    EncoderBuffer buffer;
    Status status = encoder.EncodeMeshToBuffer(*mesh, &buffer);
    
    if (!status.ok()) {
        std::cout << "Speed " << std::setw(2) << speed << ": ENCODE FAILED - " << status.error_msg() << std::endl;
        return;
    }
    
    // Decode the mesh back
    DecoderBuffer dec_buffer;
    dec_buffer.Init(buffer.data(), buffer.size());
    
    Decoder decoder;
    auto decoded_mesh = decoder.DecodeMeshFromBuffer(&dec_buffer);
    
    if (!decoded_mesh.ok()) {
        std::cout << "Speed " << std::setw(2) << speed << ": DECODE FAILED - " << decoded_mesh.status().error_msg() << std::endl;
        return;
    }
    
    // Save encoded data to file for comparison
    std::string filename = "cpp_encoded_" + name + "_speed_" + std::to_string(speed) + ".drc";
    std::ofstream outfile(filename, std::ios::binary);
    outfile.write(buffer.data(), buffer.size());
    outfile.close();
    
    // Print result
    auto& dec_mesh = *decoded_mesh.value();
    std::cout << "Speed " << std::setw(2) << speed 
              << ": " << std::setw(6) << buffer.size() << " bytes"
              << ", decoded: " << dec_mesh.num_faces() << " faces, " 
              << dec_mesh.num_points() << " points" << std::endl;
    
    // Print first few vertex positions for verification
    if (dec_mesh.num_points() > 0) {
        const PointAttribute* pos_att = dec_mesh.GetNamedAttribute(GeometryAttribute::POSITION);
        if (pos_att) {
            std::cout << "  First 3 vertices: ";
            for (int i = 0; i < std::min(3, (int)dec_mesh.num_points()); ++i) {
                std::array<float, 3> pos;
                pos_att->GetValue(AttributeValueIndex(i), pos.data());
                std::cout << "[" << pos[0] << "," << pos[1] << "," << pos[2] << "] ";
            }
            std::cout << std::endl;
        }
    }
}

int main() {
    std::cout << "=== C++ Draco Speed Comparison Test ===" << std::endl;
    std::cout << std::endl;
    
    // Test with cube
    std::cout << "--- Cube Mesh (12 faces) ---" << std::endl;
    auto cube = CreateTestMesh();
    std::cout << "Original: " << cube->num_faces() << " faces, " << cube->num_points() << " points" << std::endl;
    
    for (int speed = 0; speed <= 10; ++speed) {
        TestEncodingSpeed(cube.get(), speed, "cube");
    }
    
    std::cout << std::endl;
    
    // Test with sphere
    std::cout << "--- Sphere Mesh (256 faces) ---" << std::endl;
    auto sphere = CreateSphereMesh(16, 8);
    std::cout << "Original: " << sphere->num_faces() << " faces, " << sphere->num_points() << " points" << std::endl;
    
    for (int speed = 0; speed <= 10; ++speed) {
        TestEncodingSpeed(sphere.get(), speed, "sphere");
    }
    
    std::cout << std::endl;
    std::cout << "Files saved as cpp_encoded_*.drc for comparison with Rust implementation" << std::endl;
    
    return 0;
}
