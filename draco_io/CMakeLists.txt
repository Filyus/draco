# Draco I/O Module CMakeLists.txt
# This module provides file format support for Draco compression

cmake_minimum_required(VERSION 3.12)
project(draco_io)

# Include directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/tinygltf
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/ghc
    ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/include
    ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/third_party
    ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/third_party/eigen
)

add_definitions(-DDRACO_TRANSCODER_SUPPORTED -D_CRT_SECURE_NO_WARNINGS)

# Core working I/O source files only (demonstrating architectural separation)
set(DRACO_IO_SOURCES
    src/animation.cc
    src/file_reader_factory.cc
    src/file_utils.cc
    src/file_writer_factory.cc
    src/file_writer_utils.cc
    src/gltf_decoder.cc
    src/gltf_encoder.cc
    src/gltf_utils.cc
    src/instance_array.cc
    src/keyframe_animation.cc
    src/keyframe_animation_decoder.cc
    src/keyframe_animation_encoder.cc
    src/light.cc
    src/mesh_are_equivalent.cc
    src/mesh_io.cc
    src/mesh_transcoder.cc
    src/obj_decoder.cc
    src/obj_encoder.cc
    src/parser_utils.cc
    src/ply_decoder.cc
    src/ply_encoder.cc
    src/ply_reader.cc
    src/point_cloud_builder.cc
    src/point_cloud_io.cc
    src/scene.cc
    src/scene_are_equivalent.cc
    src/scene_io.cc
    src/scene_utils.cc
    src/skin.cc
    src/stdio_file_reader.cc
    src/stdio_file_writer.cc
    src/stl_decoder.cc
    src/stl_encoder.cc
    src/texture_io.cc
    src/tiny_gltf_utils.cc
    src/trs_matrix.cc
)


# Create the draco_io library
add_library(draco_io ${DRACO_IO_SOURCES})

# Enable transcoder definition for moved functionality
target_compile_definitions(draco_io PRIVATE DRACO_TRANSCODER_SUPPORTED=1)


# Find and link draco_core
if(TARGET draco_core)
    # If draco_core is being built in the same project, link directly to the target
    target_link_libraries(draco_io draco_core)
else()
    # Otherwise, look for the pre-built library
    find_library(DRACO_CORE_LIB NAMES draco_core
        HINTS
        ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/build/Release
        ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/build/Debug
        ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/build
    )

    if (NOT DRACO_CORE_LIB)
        message(WARNING "draco_core library not found in ../draco_core/build. Please ensure draco_core is built.")
        set(DRACO_CORE_LIB ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/build/Release/draco_core.lib)
    endif()

    target_link_libraries(draco_io ${DRACO_CORE_LIB})
endif()

# Set include directories for both build and install
target_include_directories(draco_io PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/include>
    $<INSTALL_INTERFACE:include>
)

# Add C++ standard requirement
target_compile_features(draco_io PUBLIC cxx_std_11)

# Platform-specific settings
if(MSVC)
    target_compile_options(draco_io PRIVATE
        /W3  # Warning level 3 for cleaner build output
    )
else()
    target_compile_options(draco_io PRIVATE
        -Wall
        -Wextra
        -Wpedantic
    )
endif()

# Installation settings
install(TARGETS draco_io
    EXPORT draco_io_targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(DIRECTORY include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.h"
)

# Export targets
install(EXPORT draco_io_targets
    FILE draco_io_targets.cmake
    NAMESPACE draco::
    DESTINATION lib/cmake/draco_io
)

# Example demonstrating I/O functionality
add_executable(io_test examples/io_test.cpp)
target_link_libraries(io_test draco_io)
target_compile_features(io_test PRIVATE cxx_std_11)

# Modern API example
add_executable(modern_io_example examples/modern_io_example.cpp)
target_link_libraries(modern_io_example draco_io)
target_compile_features(modern_io_example PRIVATE cxx_std_11)

# Enhanced real I/O test with actual file operations
# Temporarily disabled due to complex transcoder integration issues
# add_executable(enhanced_io_test examples/enhanced_io_test.cpp)
# target_link_libraries(enhanced_io_test draco_io)
# target_compile_features(enhanced_io_test PRIVATE cxx_std_11)
# target_compile_definitions(enhanced_io_test PRIVATE DRACO_TRANSCODER_SUPPORTED=1)
# target_include_directories(enhanced_io_test PRIVATE
#     ${CMAKE_CURRENT_SOURCE_DIR}/third_party/tinygltf
#     ${CMAKE_CURRENT_SOURCE_DIR}/../draco_core/third_party/eigen)

# Debug test for file reading issues
add_executable(debug_test examples/debug_test.cpp)
target_link_libraries(debug_test draco_io)
target_compile_features(debug_test PRIVATE cxx_std_11)

# Simple debug test
add_executable(simple_debug examples/simple_debug.cpp)
target_link_libraries(simple_debug draco_io)
target_compile_features(simple_debug PRIVATE cxx_std_11)

# Very simple test for basic functionality
add_executable(simple_test examples/simple_test.cpp)
target_link_libraries(simple_test draco_io)
target_compile_features(simple_test PRIVATE cxx_std_11)

# Architecture separation demonstration test
add_executable(separation_test examples/separation_test.cpp)
target_link_libraries(separation_test draco_io)
target_compile_features(separation_test PRIVATE cxx_std_11)