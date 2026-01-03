// Copyright 2021 The Draco Authors.
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
#ifndef DRACO_IO_MESH_TRANSCODER_H_
#define DRACO_IO_MESH_TRANSCODER_H_

#ifdef DRACO_TRANSCODER_SUPPORTED

#include <memory>
#include <unordered_map>
#include <string>
#include <vector>

#include "draco/core/hash_utils.h"
#include "draco/mesh/mesh.h"
#include "draco/material/material_library.h"
#include "draco/mesh/mesh_features.h"
#include "draco/mesh/mesh_indices.h"
#include "draco/metadata/structural_metadata.h"
#include "draco/texture/texture_library.h"

namespace draco {

// Mesh transcoder functionality that was moved from draco_core.
// This provides all I/O-specific mesh operations including materials,
// features, and texture handling.
class MeshTranscoder {
 public:
  // Copies all mesh data including materials, features, and textures.
  // This replaces the full Mesh::Copy() method that was in draco_core.
  static void CopyMeshWithMaterials(Mesh *dst, const Mesh &src);

  // Sets mesh name.
  static void SetName(Mesh *mesh, const std::string &name);
  static const std::string &GetName(const Mesh &mesh);

  // Material library access.
  static const MaterialLibrary &GetMaterialLibrary(const Mesh &mesh);
  static MaterialLibrary &GetMaterialLibrary(Mesh *mesh);

  // Removes all materials that are not referenced by any face of the mesh.
  static void RemoveUnusedMaterials(Mesh *mesh);
  static void RemoveUnusedMaterials(Mesh *mesh, bool remove_unused_material_indices);

  // Non-material texture library access.
  static const TextureLibrary &GetNonMaterialTextureLibrary(const Mesh &mesh);
  static TextureLibrary &GetNonMaterialTextureLibrary(Mesh *mesh);

  // Mesh feature ID sets as defined by EXT_mesh_features glTF extension.
  static MeshFeaturesIndex AddMeshFeatures(
      Mesh *mesh, std::unique_ptr<MeshFeatures> mesh_features);
  static int NumMeshFeatures(const Mesh &mesh);
  static const MeshFeatures &GetMeshFeatures(const Mesh &mesh, MeshFeaturesIndex index);
  static MeshFeatures &GetMeshFeatures(Mesh *mesh, MeshFeaturesIndex index);
  static void RemoveMeshFeatures(Mesh *mesh, MeshFeaturesIndex index);

  // Updates texture pointers in mesh features after copying.
  static void UpdateMeshFeaturesTexturePointer(
      TextureLibrary *texture_library, MeshFeatures *mesh_features);

  // Copy structural metadata.
  static void CopyStructuralMetadata(
      Mesh *dst, const StructuralMetadata &src_structural_metadata);

 private:
  // Internal helper for copying mesh features.
  static void CopyMeshFeatures(Mesh *dst, const Mesh &src);
};

}  // namespace draco

#endif  // DRACO_TRANSCODER_SUPPORTED

#endif  // DRACO_IO_MESH_TRANSCODER_H_