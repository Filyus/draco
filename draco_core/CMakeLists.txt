# Draco Core Module CMakeLists.txt
# This module provides the modular foundation for Draco compression

cmake_minimum_required(VERSION 3.12)
project(draco_core)

# Include directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/eigen
)

add_definitions(-DDRACO_TRANSCODER_SUPPORTED)

# Add more core sources incrementally
set(DRACO_CORE_SOURCES
    # Core module sources
    src/status.cc
    src/draco_types.cc
    src/data_buffer.cc
    src/bit_utils.cc
    src/encoder_buffer.cc
    src/decoder_buffer.cc
    src/hash_utils.cc
    src/options.cc
    src/bounding_box.cc
    src/divide.cc
    src/quantization_utils.cc
    src/path_utils.cc

    # Attributes module sources
    src/geometry_attribute.cc
    src/point_attribute.cc
    src/attribute_transform.cc
    src/attribute_octahedron_transform.cc
    src/attribute_quantization_transform.cc

    # Geometry module sources
    src/point_cloud.cc
    src/mesh.cc
    # src/mesh_are_equivalent.cc  # Moved to draco_io (transcoder functionality)
    src/mesh_cleanup.cc
    src/mesh_misc_functions.cc
    src/mesh_stripifier.cc
    src/mesh_splitter.cc
    src/triangle_soup_mesh_builder.cc

    # Material sources
    src/material.cc
    src/material_library.cc
    src/material_utils.cc

    # Texture sources
    src/texture_library.cc
    src/texture_map.cc
    src/texture_transform.cc
    src/texture_utils.cc
    src/source_image.cc

    # Mesh features sources
    src/mesh_features.cc

    # Metadata sources (extended)
    src/structural_metadata.cc
    src/structural_metadata_schema.cc
    src/property_attribute.cc
    src/property_table.cc

    # Compression module sources
    src/encode.cc
    src/decode.cc
    src/expert_encode.cc
    src/draco_compression_options.cc

    # Point cloud encoder sources
    src/point_cloud_encoder.cc
    src/point_cloud_kd_tree_encoder.cc
    src/point_cloud_sequential_encoder.cc

    # Point cloud decoder sources
    src/point_cloud_decoder.cc
    src/point_cloud_kd_tree_decoder.cc
    src/point_cloud_sequential_decoder.cc

    # Mesh encoder sources
    src/mesh_encoder.cc
    src/mesh_edgebreaker_encoder.cc
    src/mesh_sequential_encoder.cc

    # Mesh decoder sources
    src/mesh_decoder.cc
    src/mesh_edgebreaker_decoder.cc
    src/mesh_sequential_decoder.cc

    # Attributes compression sources
    src/attributes_decoder.cc
    src/attributes_encoder.cc
    src/kd_tree_attributes_decoder.cc
    src/kd_tree_attributes_encoder.cc
    src/sequential_attribute_decoder.cc
    src/sequential_attribute_encoder.cc
    src/sequential_attribute_encoders_controller.cc
    src/sequential_attribute_decoders_controller.cc
    src/sequential_integer_attribute_decoder.cc
    src/sequential_normal_attribute_decoder.cc
    src/sequential_quantization_attribute_decoder.cc

    # Metadata sources
    src/metadata_encoder.cc
    src/metadata_decoder.cc
    src/metadata.cc
    src/geometry_metadata.cc

    # Additional dependency sources
    src/mesh_edgebreaker_encoder_impl.cc
    src/mesh_edgebreaker_decoder_impl.cc
    src/corner_table.cc
    src/mesh_attribute_corner_table.cc
    src/prediction_scheme_encoder_factory.cc
    src/sequential_quantization_attribute_encoder.cc
    src/sequential_integer_attribute_encoder.cc
    src/sequential_normal_attribute_encoder.cc

    # Point cloud algorithm sources
    src/integer_points_kd_tree_decoder.cc
    src/dynamic_integer_points_kd_tree_decoder.cc
    src/float_points_tree_decoder.cc

    # Bit coders sources
    src/adaptive_rans_bit_decoder.cc
    src/adaptive_rans_bit_encoder.cc
    src/direct_bit_decoder.cc
    src/direct_bit_encoder.cc
    src/rans_bit_decoder.cc
    src/rans_bit_encoder.cc
    src/symbol_bit_decoder.cc
    src/symbol_bit_encoder.cc

    # Entropy coding sources
    src/shannon_entropy.cc
    src/symbol_decoding.cc
    src/symbol_encoding.cc
)


# Create the draco_core library as a regular library
add_library(draco_core ${DRACO_CORE_SOURCES})

# Enable transcoder support in core (needed for Mesh members)
target_compile_definitions(draco_core PUBLIC DRACO_TRANSCODER_SUPPORTED=1)

# Set include directories
target_include_directories(draco_core PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

# Add C++ standard requirement
target_compile_features(draco_core PUBLIC cxx_std_11)

# Platform-specific settings
if(MSVC)
    target_compile_options(draco_core PRIVATE
        /W3  # Warning level 3 for cleaner build output
    )
else()
    target_compile_options(draco_core PRIVATE
        -Wall
        -Wextra
        -Wpedantic
    )
endif()

# Installation settings
install(TARGETS draco_core
    EXPORT draco_core_targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(DIRECTORY include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.h"
)

# Export targets
install(EXPORT draco_core_targets
    FILE draco_core_targets.cmake
    NAMESPACE draco::
    DESTINATION lib/cmake/draco_core
)

# Basic test demonstrating core functionality
add_executable(basic_test examples/basic_test.cpp)
target_link_libraries(basic_test draco_core)

# Comprehensive test with real data and compression
add_executable(comprehensive_test examples/comprehensive_test.cpp)
target_link_libraries(comprehensive_test draco_core)

# Set C++ standard requirement for examples
target_compile_features(basic_test PRIVATE cxx_std_11)
target_compile_features(comprehensive_test PRIVATE cxx_std_11)

# Modern API example (when ready)
# add_executable(modern_api_example examples/modern_api_example.cpp)
# target_link_libraries(modern_api_example draco_core)
# target_compile_features(modern_api_example PRIVATE cxx_std_11)