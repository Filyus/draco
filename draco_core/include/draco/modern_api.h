// Modern Draco API Header
// Provides clean, Rust-style interface while preserving original functionality

#ifndef DRACO_MODERN_API_H_
#define DRACO_MODERN_API_H_

#include "draco/core/status.h"
#include "draco/core/status_or.h"
#include "draco/point_cloud/point_cloud.h"
#include "draco/mesh/mesh.h"
#include "draco/compression/encode.h"
#include "draco/compression/decode.h"
#include "draco/attributes/geometry_attribute.h"
#include "draco/attributes/point_attribute.h"

#include <memory>
#include <vector>

namespace draco {

// Modern PointCloud wrapper class
class ModernPointCloud {
public:
    ModernPointCloud() : point_cloud_(std::make_unique<PointCloud>()) {}

    // Get number of points
    int32_t num_points() const { return point_cloud_->num_points(); }

    // Get number of attributes
    int32_t num_attributes() const { return point_cloud_->num_attributes(); }

    // Create a PointAttribute from GeometryAttribute and add it
    int AddAttribute(const GeometryAttribute& attribute) {
        auto point_attr = std::make_unique<PointAttribute>();
        point_attr->Init(attribute);
        return point_cloud_->AddAttribute(std::move(point_attr));
    }

    // Get attribute
    const PointAttribute* GetAttribute(int32_t index) const {
        return point_cloud_->GetAttributeByUniqueId(index);
    }

    // Access underlying point cloud for advanced usage
    PointCloud* get() const { return point_cloud_.get(); }

private:
    std::unique_ptr<PointCloud> point_cloud_;

    friend class ModernDecoder;
};

// Modern Mesh wrapper class
class ModernMesh {
public:
    ModernMesh() : mesh_(std::make_unique<Mesh>()) {}

    // Get number of faces
    int32_t num_faces() const { return mesh_->num_faces(); }

    // Get number of points
    int32_t num_points() const { return mesh_->num_points(); }

    // Get number of attributes
    int32_t num_attributes() const { return mesh_->num_attributes(); }

    // Add attribute
    int AddAttribute(const GeometryAttribute& attribute) {
        auto point_attr = std::make_unique<PointAttribute>();
        point_attr->Init(attribute);
        return mesh_->AddAttribute(std::move(point_attr));
    }

    // Get face as modern array
    std::vector<uint32_t> GetFace(int32_t face_id) const {
        const auto& face = mesh_->face(FaceIndex(face_id));
        return {face[0].value(), face[1].value(), face[2].value()};
    }

    // Access underlying mesh for advanced usage
    Mesh* get() const { return mesh_.get(); }

private:
    std::unique_ptr<Mesh> mesh_;

    friend class ModernDecoder;
};

// Modern encoder class
class ModernEncoder {
public:
    // Encode point cloud
    template<typename T>
    StatusOr<std::vector<T>> EncodePointCloud(const ModernPointCloud& point_cloud,
                                             int32_t compression_level = 7) {
        EncoderBuffer buffer;
        Encoder encoder;

        // Set encoding options using original API
        encoder.SetSpeedOptions(compression_level, compression_level);
        encoder.SetAttributeQuantization(GeometryAttribute::POSITION, 14);

        // Encode
        DRACO_RETURN_IF_ERROR(encoder.EncodePointCloudToBuffer(point_cloud.get(), &buffer));

        // Convert to vector
        const T* data = reinterpret_cast<const T*>(buffer.data());
        return std::vector<T>(data, data + buffer.size() / sizeof(T));
    }

    // Encode mesh
    template<typename T>
    StatusOr<std::vector<T>> EncodeMesh(const ModernMesh& mesh,
                                       int32_t compression_level = 7) {
        EncoderBuffer buffer;
        Encoder encoder;

        // Set encoding options using original API
        encoder.SetSpeedOptions(compression_level, compression_level);
        encoder.SetAttributeQuantization(GeometryAttribute::POSITION, 14);

        // Encode
        DRACO_RETURN_IF_ERROR(encoder.EncodeMeshToBuffer(mesh.get(), &buffer));

        // Convert to vector
        const T* data = reinterpret_cast<const T*>(buffer.data());
        return std::vector<T>(data, data + buffer.size() / sizeof(T));
    }
};

// Modern decoder class
class ModernDecoder {
public:
    // Decode point cloud
    StatusOr<ModernPointCloud> DecodePointCloud(const void* data, size_t size) {
        DecoderBuffer buffer;
        buffer.Init(static_cast<const char*>(data), size);

        Decoder decoder;
        auto status_or = decoder.DecodePointCloudFromBuffer(&buffer);

        if (!status_or.ok()) {
            return status_or.status();
        }

        ModernPointCloud modern_pc;
        modern_pc.point_cloud_ = std::move(status_or.value());
        return modern_pc;
    }

    // Decode mesh
    StatusOr<ModernMesh> DecodeMesh(const void* data, size_t size) {
        DecoderBuffer buffer;
        buffer.Init(static_cast<const char*>(data), size);

        Decoder decoder;
        auto status_or = decoder.DecodeMeshFromBuffer(&buffer);

        if (!status_or.ok()) {
            return status_or.status();
        }

        ModernMesh modern_mesh;
        modern_mesh.mesh_ = std::move(status_or.value());
        return modern_mesh;
    }
};

}  // namespace draco

#endif  // DRACO_MODERN_API_H_