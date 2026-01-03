// Copyright 2019 The Draco Authors.
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
#ifndef DRACO_COMPRESSION_DRACO_COMPRESSION_OPTIONS_H_
#define DRACO_COMPRESSION_DRACO_COMPRESSION_OPTIONS_H_

#include <string>
#include <vector>

#include "draco/core/status.h"

namespace draco {

class SpatialQuantizationOptions {
 public:
  enum Mode {
    UNDEFINED,
    LOCAL_QUANTIZATION_BITS,
    GLOBAL_GRID
  };

  SpatialQuantizationOptions() : mode_(UNDEFINED), quantization_bits_(0), spacing_(0.f) {}
  SpatialQuantizationOptions(int quantization_bits);

  void SetQuantizationBits(int quantization_bits);
  SpatialQuantizationOptions &SetGrid(float spacing);
  bool AreQuantizationBitsDefined() const;
  
  int quantization_bits() const { return quantization_bits_; }
  float spacing() const { return spacing_; }

  bool operator==(const SpatialQuantizationOptions &other) const;
  bool operator!=(const SpatialQuantizationOptions &other) const { return !(*this == other); }

 private:
  Mode mode_;
  int quantization_bits_;
  float spacing_;
};

// Class holding encoding options for the Draco compression.
class DracoCompressionOptions {
 public:
  DracoCompressionOptions()
      : compression_level(7),
        quantization_position(11),
        quantization_bits_tex_coord(10),
        quantization_bits_normal(8),
        quantization_bits_color(8),
        quantization_bits_generic(8),
        quantization_bits_tangent(8),
        quantization_bits_weight(8),
        quantization_range(-1.f),
        quantization_origin(nullptr),
        create_metadata(false),
        preserve_polygons(false),
        use_built_in_attribute_compression(true) {}

  // 0 - 10, 10 is the best compression.
  int compression_level;
  SpatialQuantizationOptions quantization_position;
  int quantization_bits_tex_coord;
  int quantization_bits_normal;
  int quantization_bits_color;
  int quantization_bits_generic;
  int quantization_bits_tangent;
  int quantization_bits_weight;
  float quantization_range;
  const float *quantization_origin;
  bool create_metadata;
  bool preserve_polygons;
  bool use_built_in_attribute_compression;
  std::vector<std::string> metadata_quantization;
  std::vector<std::string> metadata_original_name;

  Status Check() const { return OkStatus(); } // TODO: Implement validation
  bool operator==(const DracoCompressionOptions &other) const { return true; } // TODO: Implement
  bool operator!=(const DracoCompressionOptions &other) const { return !(*this == other); }
};

}  // namespace draco

#endif  // DRACO_COMPRESSION_DRACO_COMPRESSION_OPTIONS_H_
