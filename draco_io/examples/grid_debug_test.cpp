// Debug test for grid mesh encoding with parallelogram prediction
// This matches the Rust test_grid_encoding_parallelogram test

#include <iostream>
#include <memory>
#include <vector>
#include <cmath>
#include <cstdio>

#include "draco/mesh/mesh.h"
#include "draco/mesh/triangle_soup_mesh_builder.h"
#include "draco/io/mesh_io.h"
#include "draco/io/file_reader_factory.h"
#include "draco/io/stdio_file_reader.h"
#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/core/encoder_buffer.h"
#include "draco/core/decoder_buffer.h"
#include "draco/mesh/corner_table.h"
#include "draco/mesh/mesh_misc_functions.h"
#include <algorithm>

using namespace draco;

int main() {
    std::cout << "=== C++ Debug Grid Test ===" << std::endl;
    std::cout.flush();

    // Ensure StdioFileReader is registered
    static bool reader_registered = FileReaderFactory::RegisterReader(StdioFileReader::Open);
    std::cout << "Reader registered: " << (reader_registered ? "yes" : "no") << std::endl;

    // Create 5x5 grid mesh in-memory (same as Rust test)
    auto CreateGridMesh = [](int width, int height) -> std::unique_ptr<Mesh> {
        TriangleSoupMeshBuilder mb;
        int num_faces = (width - 1) * (height - 1) * 2;
        mb.Start(num_faces);
        // Add a position attribute so SetAttributeValuesForFace has a valid att_id.
        const int pos_att_id = mb.AddAttribute(GeometryAttribute::POSITION, 3, DT_FLOAT32);
        int face_idx = 0;
        for (int y = 0; y < height - 1; ++y) {
            for (int x = 0; x < width - 1; ++x) {
                float p0[3] = {static_cast<float>(x), static_cast<float>(y), 0.0f};
                float p1[3] = {static_cast<float>(x + 1), static_cast<float>(y), 0.0f};
                float p2[3] = {static_cast<float>(x), static_cast<float>(y + 1), 0.0f};
                float p3[3] = {static_cast<float>(x + 1), static_cast<float>(y + 1), 0.0f};
                // Triangle 1: p0, p1, p2
                mb.SetAttributeValuesForFace(pos_att_id, FaceIndex(face_idx), p0, p1, p2);
                face_idx++;
                // Triangle 2: p1, p3, p2
                mb.SetAttributeValuesForFace(pos_att_id, FaceIndex(face_idx), p1, p3, p2);
                face_idx++;
            }
        }
        return mb.Finalize();
    };

    std::unique_ptr<Mesh> mesh = CreateGridMesh(5, 5);
    if (!mesh) {
        std::cerr << "Failed to create grid mesh!" << std::endl;
        return 1;
    }
    std::cout << "Created grid mesh with " << mesh->num_faces() << " faces, " << mesh->num_points() << " points" << std::endl;

    // DEBUG: Create CornerTable directly and print vertices/opposites for parity checks
    std::unique_ptr<draco::CornerTable> ct = draco::CreateCornerTableFromPositionAttribute(mesh.get());
    if (ct) {
      std::cout << "C++ corner_table vertices (first 36): [";
      for (size_t i = 0; i < std::min(static_cast<size_t>(ct->num_corners()), size_t(36)); ++i) {
        if (i > 0) std::cout << ", ";
        std::cout << ct->Vertex(draco::CornerIndex(static_cast<uint32_t>(i))).value();
      }
      std::cout << "]" << std::endl;

      std::cout << "C++ opposites (first 36) = [";
      for (size_t i = 0; i < std::min(static_cast<size_t>(ct->num_corners()), size_t(36)); ++i) {
        if (i > 0) std::cout << ", ";
        std::cout << ct->Opposite(draco::CornerIndex(static_cast<uint32_t>(i))).value();
      }
      std::cout << "]" << std::endl;
    }

    // Print first few vertices
    std::cout << "Original mesh vertices (first 10):" << std::endl;
    const PointAttribute *pos_att = mesh->GetNamedAttribute(GeometryAttribute::POSITION);
    for (int i = 0; i < 10 && i < static_cast<int>(mesh->num_points()); i++) {
        float pos[3];
        pos_att->GetMappedValue(PointIndex(i), pos);
        std::cout << "  Point " << i << ": (" << pos[0] << ", " << pos[1] << ", " << pos[2] << ")" << std::endl;
    }

    // Encode with Edgebreaker and Parallelogram prediction
    EncoderBuffer encoder_buffer;
    Encoder encoder;
    encoder.SetEncodingMethod(MESH_EDGEBREAKER_ENCODING);
    encoder.SetAttributePredictionScheme(GeometryAttribute::POSITION, 
                                          MESH_PREDICTION_PARALLELOGRAM);
    encoder.SetSpeedOptions(0, 0);  // Best compression

    std::cout << "\n=== ENCODING ===" << std::endl;
    std::cout.flush();
    Status status = encoder.EncodeMeshToBuffer(*mesh, &encoder_buffer);
    std::cout.flush();
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
    const PointAttribute *decoded_pos = decoded_mesh->GetNamedAttribute(GeometryAttribute::POSITION);
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
        if (std::abs(pos[0]) < 0.001f && std::abs(pos[1]) < 0.001f && std::abs(pos[2]) < 0.001f) {
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
