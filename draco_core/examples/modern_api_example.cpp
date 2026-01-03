// Example demonstrating modern Draco API usage

#include "draco/modern_api.h"
#include "draco/attributes/geometry_attribute.h"
#include "draco/core/vector_d.h"

#include <iostream>
#include <vector>

using namespace draco;

int main() {
    std::cout << "Draco Modern API Example\n";
    std::cout << "========================\n\n";

    // Create a simple point cloud using modern API
    ModernPointCloud pc;

    // Add position attribute
    GeometryAttribute position_attr;
    position_attr.Init(GeometryAttribute::POSITION, nullptr, 3, DataType::DT_FLOAT32, false, sizeof(float) * 3, 0);
    int pos_id = pc.AddAttribute(position_attr);
    std::cout << "Added position attribute with ID: " << pos_id << "\n";

    // Create some test points
    std::vector<float> points = {
        0.0f, 0.0f, 0.0f,
        1.0f, 0.0f, 0.0f,
        0.0f, 1.0f, 0.0f,
        1.0f, 1.0f, 0.0f
    };

    // Access the underlying point cloud to add the actual data
    auto* raw_pc = pc.get();
    PointAttribute* pos_attr_ptr = raw_pc->attribute(pos_id);

    // Add points
    raw_pc->set_num_points(4);
    for (int i = 0; i < 4; ++i) {
        pos_attr_ptr->SetAttributeValue(pos_attr_ptr->mapped_index(PointIndex(i)),
                                       &points[i * 3]);
    }

    std::cout << "Created point cloud with " << pc.num_points() << " points\n";
    std::cout << "Number of attributes: " << pc.num_attributes() << "\n\n";

    // Encode the point cloud
    ModernEncoder encoder;
    auto encode_result = encoder.EncodePointCloud<uint8_t>(pc);

    if (!encode_result.ok()) {
        std::cerr << "Encoding failed: " << encode_result.status().error_msg() << "\n";
        return 1;
    }

    std::vector<uint8_t> compressed_data = encode_result.value();
    std::cout << "Successfully encoded point cloud to " << compressed_data.size() << " bytes\n\n";

    // Decode the point cloud
    ModernDecoder decoder;
    auto decode_result = decoder.DecodePointCloud(compressed_data.data(), compressed_data.size());

    if (!decode_result.ok()) {
        std::cerr << "Decoding failed: " << decode_result.status().error_msg() << "\n";
        return 1;
    }

    ModernPointCloud decoded_pc = decode_result.value();
    std::cout << "Successfully decoded point cloud\n";
    std::cout << "Decoded point cloud has " << decoded_pc.num_points() << " points\n";
    std::cout << "Number of attributes: " << decoded_pc.num_attributes() << "\n\n";

    // Verify the data is the same
    if (decoded_pc.num_points() == pc.num_points()) {
        std::cout << "✓ Encoding/Decoding successful - point counts match!\n";
    } else {
        std::cout << "✗ Point counts don't match!\n";
        return 1;
    }

    std::cout << "\nModern API demonstration completed successfully!\n";
    return 0;
}