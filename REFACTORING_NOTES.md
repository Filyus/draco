# Refactoring Notes - Modularization of Draco

## Goal
Split the monolithic Draco library into two separate modules:
1.  **`draco_core`**: Pure geometry and compression (no disk I/O).
2.  **`draco_io`**: File format readers/writers (depends on `draco_core`).

## Changes Made

### 1. Directory Structure
*   Created `draco_core/` and `draco_io/` directories.
*   Migrated relevant source files from `src/draco/` to these new directories.
*   **`draco_core`** now contains:
    *   `attributes/`, `compression/`, `core/`, `material/`, `mesh/`, `metadata/`, `point_cloud/`, `texture/`.
*   **`draco_io`** now contains:
    *   `io/` (File readers/writers).

### 2. Dependency Breaking (The "Split Implementation" Pattern)
A major circular dependency existed between Core and IO:
*   *Problem*: `draco_core` needs path manipulation functions (e.g., for texture handling). `draco_io` needs disk access functions. Both were in `file_utils.h`.
*   *Solution*:
    *   **`draco_core/include/draco/core/path_utils.h`**: Created new header for path string manipulation.
    *   **`draco_core/src/path_utils.cc`**: Implemented path logic (no disk I/O).
    *   **`draco_io/include/draco/io/file_utils.h`**: Retained disk I/O functions (`ReadFileToBuffer`).
    *   **`draco_io/src/file_utils.cc`**: Implemented disk I/O.

### 3. Header Relocation
*   Moved `image_compression_options.h` from `draco/io` to `draco/texture`.
    *   *Reason*: `Texture` class in Core needs these options, but cannot depend on IO.
*   Ensured `draco_core` has no dependency on IO headers or source files, strictly enforcing the separation.

### 4. Build System
*   Created separate `CMakeLists.txt` for `draco_core` and `draco_io`.
*   **`draco_core`**: Builds as a standalone library.
*   **`draco_io`**: Links against `draco_core`.
*   Added `_CRT_SECURE_NO_WARNINGS` to `draco_io` to suppress MSVC warnings for `fopen`/`sprintf`.

### 5. Verification
*   **`basic_test`**: Verifies Core functionality (Mesh, PointCloud, Encoding).
*   **`io_test`**: Verifies IO functionality (OBJ, PLY, GLTF reading/writing).
*   **`simple_test`**: Verifies end-to-end flow (Create -> Encode -> Decode).

## Status
The refactoring is complete. `draco_core` is now strictly isolated from disk I/O operations.

## Architectural Analysis
*   **`draco_core`**: Focused purely on Geometry (Mesh/PointCloud) and Compression.
*   **`draco_io`**: Handles File I/O, but also acts as the home for **Scene** and **Animation** data structures.
    *   *Decision*: While `Scene` and `Animation` are data structures, they are primarily used for the Transcoder (GLTF support). Keeping them in `draco_io` keeps `draco_core` lightweight.
    *   *Future Consideration*: If `Scene` manipulation becomes a core requirement independent of file I/O, these modules could be moved to `draco_core` or a new `draco_scene` module.

## Library Sizes (Release - Windows MSVC)

### Comparison
| Implementation | Library File(s) | Size | Notes |
| :--- | :--- | :--- | :--- |
| **Original (Monolithic)** | `draco.lib` | **~28.2 MB** | Single library containing all functionality. |
| **New (Modular)** | `draco_core.lib` | **~26.1 MB** | Core geometry & compression only. |
| | `draco_io.lib` | **~28.5 MB** | File I/O, Scene, Animation, Transcoder. |
| | **Total** | **~54.6 MB** | *Note: Increase in total static lib size is likely due to template instantiation duplication between modules, which is deduplicated by the linker when creating executables.* |


