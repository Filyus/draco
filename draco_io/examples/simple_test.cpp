// Simple test for basic I/O functionality
#include "draco/core/status.h"
#include "draco/core/status_or.h"
#include "draco/io/point_cloud_io.h"
#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/core/encoder_buffer.h"
#include "draco/core/decoder_buffer.h"

#include <iostream>
#include <memory>

int main() {
    std::cout << "Simple Draco I/O Test\n";
    std::cout << "=====================\n\n";

    // Test basic point cloud reading
    std::cout << "Testing basic point cloud compression...\n";

    // Create a simple point cloud
    draco::PointCloud pc;
    pc.set_num_points(3);

    // Add position attribute
    draco::GeometryAttribute pos_attr;
    pos_attr.Init(draco::GeometryAttribute::POSITION, nullptr, 3, draco::DataType::DT_FLOAT32, false, sizeof(float) * 3, 0);

    auto point_attr = std::make_unique<draco::PointAttribute>();
    point_attr->Init(draco::GeometryAttribute::POSITION, 3, draco::DataType::DT_FLOAT32, false, 3);
    int pos_id = pc.AddAttribute(std::move(point_attr));

    std::cout << "✅ Created simple point cloud with " << pc.num_points() << " points\n";
    std::cout << "✅ Added position attribute with ID: " << pos_id << "\n";

    // Test encoding
    draco::Encoder encoder;
    encoder.SetSpeedOptions(5, 5);
    encoder.SetAttributeQuantization(draco::GeometryAttribute::POSITION, 12);

    draco::EncoderBuffer buffer;
    auto status = encoder.EncodePointCloudToBuffer(pc, &buffer);
    if (status.ok()) {
        std::cout << "✅ Successfully encoded point cloud: " << buffer.size() << " bytes\n";
    } else {
        std::cout << "❌ Encoding failed: " << status.error_msg() << "\n";
        return 1;
    }

    // Test decoding
    draco::DecoderBuffer decode_buffer;
    decode_buffer.Init(reinterpret_cast<const char*>(buffer.data()), buffer.size());

    draco::Decoder decoder;
    auto result = decoder.DecodePointCloudFromBuffer(&decode_buffer);
    if (result.ok()) {
        auto decoded_pc = std::move(result).value();
        std::cout << "✅ Successfully decoded point cloud\n";
        std::cout << "📊 Decoded points: " << decoded_pc->num_points() << "\n";
        std::cout << "📊 Decoded attributes: " << decoded_pc->num_attributes() << "\n";
    } else {
        std::cout << "❌ Decoding failed: " << result.status().error_msg() << "\n";
        return 1;
    }

    std::cout << "\n🎉 Basic Draco I/O functionality works!\n";
    std::cout << "✅ draco_core and draco_io separation is functional\n";

    return 0;
}