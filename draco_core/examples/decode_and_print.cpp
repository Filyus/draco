// Decode a .drc file and print vertex positions for comparison
// Usage: decode_and_print.exe <input.drc>

#include <fstream>
#include <iostream>
#include <vector>
#include <cstdint>
#include <iomanip>

#include "draco/compression/decode.h"
#include "draco/core/decoder_buffer.h"
#include "draco/mesh/mesh.h"
#include "draco/point_cloud/point_cloud.h"

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <input.drc>" << std::endl;
        return 1;
    }

    const char* input_file = argv[1];

    // Read the file
    std::ifstream file(input_file, std::ios::binary | std::ios::ate);
    if (!file.is_open()) {
        std::cerr << "Failed to open file: " << input_file << std::endl;
        return 1;
    }

    std::streamsize size = file.tellg();
    file.seekg(0, std::ios::beg);

    std::vector<char> buffer(size);
    if (!file.read(buffer.data(), size)) {
        std::cerr << "Failed to read file" << std::endl;
        return 1;
    }
    file.close();

    std::cout << "File size: " << size << " bytes" << std::endl;

    // Decode
    draco::DecoderBuffer decoder_buffer;
    decoder_buffer.Init(buffer.data(), buffer.size());

    draco::Decoder decoder;
    auto status_or_geometry = decoder.DecodeMeshFromBuffer(&decoder_buffer);

    if (!status_or_geometry.ok()) {
        std::cerr << "Decode failed: " << status_or_geometry.status().error_msg_string() << std::endl;
        return 1;
    }

    std::unique_ptr<draco::Mesh> mesh = std::move(status_or_geometry).value();

    std::cout << "Decoded mesh:" << std::endl;
    std::cout << "  Faces: " << mesh->num_faces() << std::endl;
    std::cout << "  Points: " << mesh->num_points() << std::endl;
    std::cout << "  Attributes: " << mesh->num_attributes() << std::endl;

    // Find position attribute
    const draco::PointAttribute* pos_attr = mesh->GetNamedAttribute(draco::GeometryAttribute::POSITION);
    if (pos_attr == nullptr) {
        std::cerr << "No position attribute found" << std::endl;
        return 1;
    }

    std::cout << "  Position attribute:" << std::endl;
    std::cout << "    Data type: " << static_cast<int>(pos_attr->data_type()) << std::endl;
    std::cout << "    Num components: " << pos_attr->num_components() << std::endl;
    std::cout << "    Unique entries: " << pos_attr->size() << std::endl;

    // Print first 20 vertex positions
    std::cout << std::fixed << std::setprecision(6);
    std::cout << "\nFirst 20 vertex positions:" << std::endl;
    
    int num_to_print = std::min(20, static_cast<int>(mesh->num_points()));
    for (int i = 0; i < num_to_print; ++i) {
        draco::PointIndex pi(i);
        draco::AttributeValueIndex avi = pos_attr->mapped_index(pi);
        
        float values[3];
        pos_attr->GetValue(avi, values);
        
        std::cout << "  v" << i << ": [" << values[0] << ", " << values[1] << ", " << values[2] << "]" << std::endl;
    }

    // Print last 5 vertices if mesh is larger
    if (mesh->num_points() > 25) {
        std::cout << "\nLast 5 vertex positions:" << std::endl;
        for (int i = mesh->num_points() - 5; i < static_cast<int>(mesh->num_points()); ++i) {
            draco::PointIndex pi(i);
            draco::AttributeValueIndex avi = pos_attr->mapped_index(pi);
            
            float values[3];
            pos_attr->GetValue(avi, values);
            
            std::cout << "  v" << i << ": [" << values[0] << ", " << values[1] << ", " << values[2] << "]" << std::endl;
        }
    }

    // Compute bounding box
    float min_pos[3] = {1e30f, 1e30f, 1e30f};
    float max_pos[3] = {-1e30f, -1e30f, -1e30f};
    
    for (draco::PointIndex i(0); i < mesh->num_points(); ++i) {
        draco::AttributeValueIndex avi = pos_attr->mapped_index(i);
        float values[3];
        pos_attr->GetValue(avi, values);
        
        for (int j = 0; j < 3; ++j) {
            min_pos[j] = std::min(min_pos[j], values[j]);
            max_pos[j] = std::max(max_pos[j], values[j]);
        }
    }
    
    std::cout << "\nBounding box:" << std::endl;
    std::cout << "  Min: [" << min_pos[0] << ", " << min_pos[1] << ", " << min_pos[2] << "]" << std::endl;
    std::cout << "  Max: [" << max_pos[0] << ", " << max_pos[1] << ", " << max_pos[2] << "]" << std::endl;

    return 0;
}
