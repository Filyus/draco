# Draco Project Instructions

## Project Overview
Draco is a library for compressing and decompressing 3D geometric meshes and point clouds. It is intended to improve the storage and transmission of 3D graphics.

## Tech Stack
- **Languages**: C++ (Core), CMake (Build System), Python (Scripts), JavaScript/WebAssembly (Web builds).
- **Build System**: CMake.
- **Testing Framework**: Googletest.

## Coding Style
- Follow the [Google C++ Style Guide](https://google.github.io/styleguide/cppguide.html).
- Ensure code is formatted correctly.

## Build Instructions

### General
1.  Create a build directory: `mkdir build_dir && cd build_dir`
2.  Run CMake: `cmake ../`
3.  Build: `cmake --build .`

### Windows (Visual Studio)
To generate Visual Studio projects:
```bash
cmake ../ -G "Visual Studio 17 2022" -A x64
```
(Adjust Visual Studio version as needed).

### Transcoder Support
To build with transcoding support:
1.  Initialize submodules: `git submodule update --init`
2.  Run CMake with the flag: `cmake ../ -DDRACO_TRANSCODER_SUPPORTED=ON`

### WebAssembly / JavaScript
-   See `BUILDING.md` for details on building for the web using Emscripten.
-   Targets include `draco_js_dec`, `draco_js_enc`, `draco_wasm_dec`, `draco_wasm_enc`.

## Key Targets
-   `draco`: The main library.
-   `draco_encoder`: Command line tool for encoding.
-   `draco_decoder`: Command line tool for decoding.
-   `draco_transcoder`: Tool for transcoding (requires optional build flag).

## CMake Configuration
-   `CMakeLists.txt` is the main entry point.
-   Helper macros are in `cmake/`.
-   `draco_options.cmake` controls build options.

## Contribution Guidelines
-   Sign the Google Individual Contributor License Agreement (CLA).
-   All submissions require code review via GitHub pull requests.
-   Ensure tests pass before submitting.

## Implementation Standards
-   **No Simplifications**: Implement complete, production-ready code. Do not use simplified versions, shortcuts, or hardcoded values where dynamic logic is required.
-   **Full Fidelity**: When porting logic, ensure all edge cases and complex logic from the original implementation are preserved.
-   **No Assumptions**: Do not make assumptions about the codebase or user intent. Always gather necessary context using available tools before implementing solutions.
-   **Progress Tracking**: Maintain the [implementation_state.yaml](implementation_state.yaml) file. Whenever a new structure or function is implemented, or an existing one is updated, ensure the corresponding entry in [implementation_state.yaml](implementation_state.yaml) is updated with the correct `progress` and `note` to reflect the current state of the port compared to the C++ original.
