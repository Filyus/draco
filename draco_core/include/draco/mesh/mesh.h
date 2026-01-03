// Copyright 2016 The Draco Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
#ifndef DRACO_MESH_MESH_H_
#define DRACO_MESH_MESH_H_

#include <array>
#include <memory>
#include <vector>
#include <unordered_set>

#include "draco/attributes/geometry_indices.h"
#include "draco/core/hash_utils.h"
#include "draco/core/macros.h"
#include "draco/core/status.h"
#include "draco/draco_features.h"
#include "draco/point_cloud/point_cloud.h"
#include "draco/compression/draco_compression_options.h"
#include "draco/material/material_library.h"
#include "draco/mesh/mesh_features.h"
#include "draco/metadata/structural_metadata.h"
#include "draco/texture/texture_library.h"

namespace draco {

// Forward declaration for transcoder functionality
class MeshTranscoder;

// Mesh is a collection of n-dimensional points that are described by a set
// of PointAttributes and can contain connectivity data such as faces, edges,
// or corners.
class Mesh : public PointCloud {
 public:
  typedef std::array<PointIndex, 3> Face;

  // Attribute element type enumeration.
  enum MeshAttributeElementType {
    MESH_VERTEX_ATTRIBUTE,
    MESH_CORNER_ATTRIBUTE,
    MESH_FACE_ATTRIBUTE
  };

  // Allow MeshTranscoder to access protected members
  friend class MeshTranscoder;

  Mesh();

  // Copies all data from the |src| mesh (basic core functionality only).
  void Copy(const Mesh &src);

  void AddFace(const Face &face) { faces_.push_back(face); }

  void SetFace(FaceIndex face_id, const Face &face) {
    if (face_id >= static_cast<uint32_t>(faces_.size())) {
      faces_.resize(face_id.value() + 1, Face());
    }
    faces_[face_id] = face;
  }

  const Face &GetFace(FaceIndex face_id) const {
    DRACO_DCHECK_GE(face_id, 0);
    DRACO_DCHECK_LT(face_id.value(), faces_.size());
    return faces_[face_id];
  }

  FaceIndex num_faces() const { return FaceIndex(faces_.size()); }

  void SetNumFaces(int num_faces) { faces_.resize(num_faces, Face()); }

  // Returns the number of faces of the mesh.
  uint32_t NumFaces() const { return faces_.size(); }

  // Returns the point id for a corner |ci|.
  inline PointIndex CornerToPointId(int ci) const {
    if (ci < 0 || static_cast<uint32_t>(ci) == kInvalidCornerIndex.value()) {
      return kInvalidPointIndex;
    }
    return this->face(FaceIndex(ci / 3))[ci % 3];
  }

  // Returns the point id of a corner |ci|.
  inline PointIndex CornerToPointId(CornerIndex ci) const {
    return this->CornerToPointId(ci.value());
  }

  // Returns the i-th face of the mesh.
  const Face &face(FaceIndex i) const { return faces_[i]; }

  // Attribute data per corner or vertex. This data is used by some encoders
  // that require specific mapping between attribute values and face corners.
  struct AttributeData {
    AttributeData() : element_type(MESH_CORNER_ATTRIBUTE) {}
    MeshAttributeElementType element_type;
  };

  // Returns the element type of the attribute (per-vertex or per-corner).
  MeshAttributeElementType GetAttributeElementType(int att_id) const {
    return attribute_data_[att_id].element_type;
  }

  void SetAttributeElementType(int att_id, MeshAttributeElementType et) {
    attribute_data_[att_id].element_type = et;
  }

  // Deletes attribute with id |att_id|.
  void DeleteAttribute(int att_id) {
    PointCloud::DeleteAttribute(att_id);
    if (att_id >= 0 && att_id < static_cast<int>(attribute_data_.size())) {
      attribute_data_.erase(attribute_data_.begin() + att_id);
    }
  }

 protected:
  // Container for faces.
  IndexTypeVector<FaceIndex, Face> faces_;

  // Attribute metadata for each attribute of the mesh.
  std::vector<AttributeData> attribute_data_;

  // Method that needs to be called when a new attribute is added.
  // TODO(ostava): Add better documentation for the function arguments.
  void AddAttributeData(int att_id, MeshAttributeElementType element_type) {
    if (att_id >= static_cast<int>(attribute_data_.size())) {
      attribute_data_.resize(att_id + 1);
    }
    attribute_data_[att_id].element_type = element_type;
  }

 public:
  // Transcoder functionality
  void SetName(const std::string &name) { name_ = name; }
  const std::string &GetName() const { return name_; }

  const MaterialLibrary &GetMaterialLibrary() const { return material_library_; }
  MaterialLibrary &GetMaterialLibrary() { return material_library_; }

  const TextureLibrary &GetNonMaterialTextureLibrary() const { return non_material_texture_library_; }
  TextureLibrary &GetNonMaterialTextureLibrary() { return non_material_texture_library_; }

  const StructuralMetadata &GetStructuralMetadata() const { return structural_metadata_; }
  StructuralMetadata &GetStructuralMetadata() { return structural_metadata_; }

  void RemoveUnusedMaterials() {
      // TODO: Implement
  }
  void RemoveUnusedMaterials(bool remove_unused_textures) {
      // TODO: Implement
  }

  // Mesh Features
  MeshFeaturesIndex AddMeshFeatures(std::unique_ptr<MeshFeatures> mesh_features) {
      mesh_features_.push_back(std::move(mesh_features));
      return MeshFeaturesIndex(mesh_features_.size() - 1);
  }
  
  int NumMeshFeatures() const { return static_cast<int>(mesh_features_.size()); }
  
  const MeshFeatures &GetMeshFeatures(MeshFeaturesIndex index) const {
      return *mesh_features_[index.value()];
  }
  
  MeshFeatures &GetMeshFeatures(MeshFeaturesIndex index) {
      return *mesh_features_[index.value()];
  }
  
  void RemoveMeshFeatures(MeshFeaturesIndex index) {
      if (index.value() < mesh_features_.size()) {
          mesh_features_.erase(mesh_features_.begin() + index.value());
      }
  }

  // Property Attributes
  int NumPropertyAttributesIndices() const { return static_cast<int>(property_attributes_indices_.size()); }
  int GetPropertyAttributesIndex(int i) const { return property_attributes_indices_[i]; }
  int AddPropertyAttributesIndex(int index) { 
      property_attributes_indices_.push_back(index); 
      return static_cast<int>(property_attributes_indices_.size() - 1);
  }

  // Compression Options
  void SetCompressionOptions(const DracoCompressionOptions &options) { compression_options_ = options; }
  const DracoCompressionOptions &GetCompressionOptions() const { return compression_options_; }

  int NumMeshFeaturesMaterialMasks(MeshFeaturesIndex index) const {
      if (index.value() >= mesh_features_material_masks_.size()) return 0;
      return static_cast<int>(mesh_features_material_masks_[index.value()].size());
  }
  int GetMeshFeaturesMaterialMask(MeshFeaturesIndex index, int mask_index) const {
      return mesh_features_material_masks_[index.value()][mask_index];
  }
  void AddMeshFeaturesMaterialMask(MeshFeaturesIndex index, int material_index) {
      if (index.value() >= mesh_features_material_masks_.size()) {
          mesh_features_material_masks_.resize(index.value() + 1);
      }
      mesh_features_material_masks_[index.value()].push_back(material_index);
  }
  
  int NumPropertyAttributesIndexMaterialMasks(int index) const {
      if (index >= property_attributes_material_masks_.size()) return 0;
      return static_cast<int>(property_attributes_material_masks_[index].size());
  }
  int GetPropertyAttributesIndexMaterialMask(int index, int mask_index) const {
      return property_attributes_material_masks_[index][mask_index];
  }
  void AddPropertyAttributesIndexMaterialMask(int index, int material_index) {
      if (index >= property_attributes_material_masks_.size()) {
          property_attributes_material_masks_.resize(index + 1);
      }
      property_attributes_material_masks_[index].push_back(material_index);
  }
  
  static void CopyMeshFeaturesForMaterial(const Mesh &src, Mesh *dest, int material_index) {
      // TODO: Implement
  }
  static void CopyPropertyAttributesIndicesForMaterial(const Mesh &src, Mesh *dest, int material_index) {
      // TODO: Implement
  }
  
  static void UpdateMeshFeaturesTexturePointer(
      const std::unordered_map<const Texture *, int> &texture_to_index_map,
      TextureLibrary *texture_library,
      MeshFeatures *mesh_features) {
      // TODO: Implement
  }

 private:
  std::string name_;
  MaterialLibrary material_library_;
  TextureLibrary non_material_texture_library_;
  StructuralMetadata structural_metadata_;
  std::vector<std::unique_ptr<MeshFeatures>> mesh_features_;
  std::vector<int> property_attributes_indices_;
  DracoCompressionOptions compression_options_;
  
  std::vector<std::vector<int>> mesh_features_material_masks_;
  std::vector<std::vector<int>> property_attributes_material_masks_;

  // Struct that defines mapping between face value indices and corner ids.
  // TODO(ostava): This struct may not be necessary and we may be able to use
  // existing corner table.
  struct FaceValueMapping {
    // Value indices of the attribute stored at the three corners of the face.
    AttributeValueIndex value_index[3];
    // Corner indices of the three face corners.
    CornerIndex corner_index[3];
  };

  // Face to face-value-index mapping for a specific attribute id. This mapping
  // is used by encoders that have different values for different corners
  // of the same face (e.g. seam edges in UV coordinates).
  // TODO(ostava): Consider using std::unordered_map for faster lookup.
  std::vector<FaceValueMapping> attribute_value_to_index_map_;
};

}  // namespace draco

#endif  // DRACO_MESH_MESH_H_