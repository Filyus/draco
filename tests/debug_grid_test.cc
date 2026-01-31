// Debug test for grid mesh encoding with parallelogram prediction
// This matches the Rust test_grid_encoding_parallelogram test

#include <iostream>
#include <memory>
#include <vector>

#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/mesh/mesh.h"
#include "draco/core/draco_types.h"
#include "draco/attributes/geometry_attribute.h"
#include "draco/attributes/point_attribute.h"

using namespace draco;

// Create a 10x10 grid mesh matching the Rust test
std::unique_ptr<Mesh> CreateGridMesh() {
    const int grid_size = 10;
    const int num_vertices = grid_size * grid_size;  // 100 vertices
    const int num_faces = (grid_size - 1) * (grid_size - 1) * 2;  // 162 faces

    auto mesh = std::make_unique<Mesh>();
    mesh->SetNumFaces(num_faces);

    // Create position attribute
    GeometryAttribute pos_att;
    pos_att.Init(GeometryAttribute::POSITION, nullptr, 3, DT_FLOAT32, false,
                 sizeof(float) * 3, 0);
    int pos_att_id = mesh->AddAttribute(pos_att, true, num_vertices);
    PointAttribute *pos_attribute = mesh->attribute(pos_att_id);

    // Set vertex positions
    for (int y = 0; y < grid_size; y++) {
        for (int x = 0; x < grid_size; x++) {
            int vertex_id = y * grid_size + x;
            float pos[3] = {static_cast<float>(x), static_cast<float>(y), 0.0f};
            pos_attribute->SetAttributeValue(AttributeValueIndex(vertex_id), pos);
        }
    }

    // Create faces (two triangles per grid cell)
    int face_idx = 0;
    for (int y = 0; y < grid_size - 1; y++) {
        for (int x = 0; x < grid_size - 1; x++) {
            int v00 = y * grid_size + x;
            int v10 = y * grid_size + x + 1;
            int v01 = (y + 1) * grid_size + x;
            int v11 = (y + 1) * grid_size + x + 1;

            // First triangle: v00, v10, v11
            Mesh::Face face1;
            face1[0] = PointIndex(v00);
            face1[1] = PointIndex(v10);
            face1[2] = PointIndex(v11);
            mesh->SetFace(FaceIndex(face_idx++), face1);

            // Second triangle: v00, v11, v01
            Mesh::Face face2;
            face2[0] = PointIndex(v00);
            face2[1] = PointIndex(v11);
            face2[2] = PointIndex(v01);
            mesh->SetFace(FaceIndex(face_idx++), face2);
        }
    }

    // Use identity mapping
    pos_attribute->SetIdentityMapping();
    mesh->set_num_points(num_vertices);

    return mesh;
}

int main() {
    std::cout << "=== C++ Debug Grid Test ===" << std::endl;

    // Create the grid mesh
    auto mesh = CreateGridMesh();
    std::cout << "Created mesh with " << mesh->num_faces() << " faces, "
              << mesh->num_points() << " points" << std::endl;

    // Print first few vertices
    std::cout << "Original mesh vertices (first 10):" << std::endl;
    PointAttribute *pos_att = mesh->attribute(0);
    for (int i = 0; i < 10; i++) {
        float pos[3];
        pos_att->GetValue(AttributeValueIndex(i), pos);
        std::cout << "  Vertex " << i << ": (" << pos[0] << ", " << pos[1] << ", " << pos[2] << ")" << std::endl;
    }

    // Encode with Edgebreaker and Parallelogram prediction
    EncoderBuffer encoder_buffer;
    Encoder encoder;
    encoder.SetEncodingMethod(MESH_EDGEBREAKER_ENCODING);
    encoder.SetAttributePredictionScheme(GeometryAttribute::POSITION, 
                                          MESH_PREDICTION_PARALLELOGRAM);
    encoder.SetSpeedOptions(0, 0);  // Best compression

    std::cout << "\n=== ENCODING ===" << std::endl;
    Status status = encoder.EncodeMeshToBuffer(*mesh, &encoder_buffer);
    if (!status.ok()) {
        std::cerr << "Encoding failed: " << status.error_msg() << std::endl;
        return 1;
    }
    std::cout << "Encoded size: " << encoder_buffer.size() << " bytes" << std::endl;

    // Decode
    std::cout << "\n=== DECODING ===" << std::endl;
    DecoderBuffer decoder_buffer;
    decoder_buffer.Init(reinterpret_cast<const char*>(encoder_buffer.data()), 
                        encoder_buffer.size());
    
    Decoder decoder;
    auto decoded_geom = decoder.DecodeMeshFromBuffer(&decoder_buffer);
    if (!decoded_geom.ok()) {
        std::cerr << "Decoding failed: " << decoded_geom.status().error_msg() << std::endl;
        return 1;
    }

    Mesh *decoded_mesh = decoded_geom.value().get();
    std::cout << "Decoded mesh with " << decoded_mesh->num_faces() << " faces, "
              << decoded_mesh->num_points() << " points" << std::endl;

    // Print decoded vertices
    std::cout << "\nDecoded mesh vertices (first 20):" << std::endl;
    PointAttribute *decoded_pos = decoded_mesh->attribute(0);
    for (int i = 0; i < 20 && i < static_cast<int>(decoded_mesh->num_points()); i++) {
        float pos[3];
        decoded_pos->GetMappedValue(PointIndex(i), pos);
        std::cout << "  Point " << i << ": (" << pos[0] << ", " << pos[1] << ", " << pos[2] << ")" << std::endl;
    }

    // Verify Point 0 should be (0, 0, 0)
    std::cout << "\n=== VERIFICATION ===" << std::endl;
    bool found_origin = false;
    for (uint32_t i = 0; i < decoded_mesh->num_points(); i++) {
        float pos[3];
        decoded_pos->GetMappedValue(PointIndex(i), pos);
        if (pos[0] == 0.0f && pos[1] == 0.0f && pos[2] == 0.0f) {
            std::cout << "Found (0, 0, 0) at Point " << i << std::endl;
            found_origin = true;
            break;
        }
    }
    if (!found_origin) {
        std::cout << "ERROR: Point (0, 0, 0) not found in decoded mesh!" << std::endl;
    }

    return found_origin ? 0 : 1;
}
