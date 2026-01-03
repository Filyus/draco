# Draco Architecture & Modularization

## Overview

The Draco library has been refactored into two distinct modules to improve separation of concerns and reduce dependencies:

1.  **`draco_core`**: The core compression and geometry library.
2.  **`draco_io`**: The input/output library for reading and writing various file formats.

## Architecture Comparison

### Original Monolithic Structure
*   **Source**: All code resided in `src/draco/`.
*   **Dependencies**: Tightly coupled. Core geometry classes depended on IO utilities (e.g., `Texture` depended on `image_compression_options.h` in `io/`).
*   **Build**: A single `draco` target included both compression logic and file I/O.

### New Modular Structure
*   **Separation**:
    *   `draco_core/`: Pure geometry and compression. **No Disk I/O**.
    *   `draco_io/`: File format readers/writers. Depends on `draco_core`.
*   **Decoupling**:
    *   Path manipulation logic moved to `draco_core/path_utils`.
    *   Disk access logic isolated in `draco_io/file_utils`.
    *   Texture options moved to `draco_texture`.

## Module Responsibilities

### `draco_core`
*   **Location**: `draco_core/`
*   **Responsibilities**:
    *   Mesh and Point Cloud data structures (`Mesh`, `PointCloud`).
    *   Compression and Decompression algorithms.
    *   Geometry processing (quantization, prediction schemes).
    *   Metadata handling.
    *   **Path Utilities**: String manipulation for file paths (extension handling, path splitting) without disk access.
*   **Dependencies**: None (Standalone).
*   **Key Changes**:
    *   `draco/io` functionality has been completely migrated out of `draco_core`.
    *   `path_utils.h` (and `.cc`) introduced to handle path string logic.
    *   `image_compression_options.h` moved to `draco/texture/`.

### `draco_io`
*   **Location**: `draco_io/`
*   **Responsibilities**:
    *   File Readers and Writers (OBJ, PLY, STL, GLTF).
    *   Disk I/O operations (`ReadFileToBuffer`, `WriteBufferToFile`).
    *   Integration with third-party format libraries (TinyGLTF).
*   **Dependencies**: `draco_core`.
*   **Key Changes**:
    *   Contains `draco/io/file_utils.h` which handles actual disk access.
    *   Links against `draco_core` to create geometry objects from files.

## Refactoring Details

### File Utilities Split
To resolve circular dependencies and architectural violations, the original `file_utils` was split:

*   **`draco_core/include/draco/core/path_utils.h`**:
    *   Contains: `SplitPath`, `ReplaceFileExtension`, `LowercaseFileExtension`, etc.
    *   Implementation: `draco_core/src/path_utils.cc`.
    *   *Reasoning*: These functions only manipulate strings and do not require OS-level disk access.

*   **`draco_io/include/draco/io/file_utils.h`**:
    *   Contains: `ReadFileToBuffer`, `WriteBufferToFile`, `GetFileSize`.
    *   Implementation: `draco_io/src/file_utils.cc`.
    *   *Reasoning*: These functions require disk access and are strictly I/O operations.
    *   *Note*: Includes `draco/core/path_utils.h` to provide a complete utility set for I/O tasks.

### Texture & Image Options
*   `image_compression_options.h` was moved from `draco/io` to `draco/texture/image_compression_options.h`.
*   This allows `Texture` classes in `draco_core` to reference compression options without depending on the `io` module.

## Build Instructions

The modules are built separately using CMake.

### Building `draco_core`
```bash
cd draco_core
mkdir build && cd build
cmake ..
cmake --build . --config Release
```

### Building `draco_io`
`draco_io` requires `draco_core` to be built first.

```bash
cd draco_io
mkdir build && cd build
cmake ..
cmake --build . --config Release
```
