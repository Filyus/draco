# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
Draco is Google's 3D geometry compression library for meshes and point clouds. Written in C++ with JavaScript/WebAssembly bindings, it provides efficient compression for 3D graphics applications.

## Quick Start

### Building
```bash
# Basic build (Linux/macOS)
mkdir build && cd build
cmake ../
make -j$(nproc)

# Windows with Visual Studio 2022
mkdir build && cd build
cmake ../ -G "Visual Studio 17 2022"
cmake --build . --config Release

# With tests
cmake ../ -DDRACO_TESTS=ON && make draco_tests
# Windows tests: cmake --build . --config Release --target draco_tests

# JavaScript/WebAssembly
export EMSCRIPTEN=/path/to/emscripten
cmake ../ -DCMAKE_TOOLCHAIN_FILE=/path/to/Emscripten.cmake
```

### Essential Commands
- `draco_encoder` - Encode 3D geometry (OBJ, STL, PLY)
- `draco_decoder` - Decode Draco files to standard formats
- `draco_transcoder` - glTF transcoding (requires transcoder build)
- `draco_tests` - Run unit tests

## Architecture Overview

### Core Modules
- `core/` - Data types, buffers, status handling
- `compression/` - Compression algorithms and pipeline
- `mesh/` - Mesh data structures and algorithms
- `attributes/` - Attribute processing and transforms
- `javascript/` - Emscripten bindings for web

### Key Design Patterns
- Status-based error handling (no exceptions)
- Modular encoder/decoder architecture
- Attribute-based compression pipeline
- Platform abstraction through CMake

## Development Guidelines

### Code Style
- Follow Google C++ Style Guide
- Use Status/StatusOr for error handling
- Prefer RAII patterns
- Document APIs thoroughly

### Testing
- Use Googletest framework
- Individual test files per component
- Run tests with: `make draco_tests`

### Building for Different Platforms
- **Windows (Visual Studio 2022)**: Use `-G "Visual Studio 17 2022"` for CMake generation, then `cmake --build . --config Release`
- **WebAssembly**: `-DCMAKE_TOOLCHAIN_FILE=Emscripten.cmake -DDRACO_WASM=ON`
- **iOS**: Use toolchain files in `cmake/toolchains/`
- **Android**: Use `-DCMAKE_TOOLCHAIN_FILE=cmake/toolchains/android.cmake`
- **Transcoder**: Enable with `-DDRACO_TRANSCODER_SUPPORTED=ON`

### Important Notes
- Use out-of-source builds (`mkdir build && cd build`)
- Static builds are default; use `-DBUILD_SHARED_LIBS=ON` for shared libraries
- JavaScript builds require Emscripten SDK
- Windows builds require Visual Studio 2022 with C++ development tools
- On Windows, use Developer Command Prompt or ensure cl.exe is in PATH
- Build produces static `draco.lib` and executables in `build/Release/`
- No submodules required for basic library and tools builds

## Git Submodules

Draco uses 4 third-party dependencies as Git submodules:

- **googletest** (`third_party/googletest`) - Testing framework
- **eigen** (`third_party/eigen`) - Linear algebra library
- **tinygltf** (`third_party/tinygltf`) - glTF loader/saver (transcoder only)
- **filesystem** (`third_party/filesystem`) - std::filesystem implementation

To update submodules: `git submodule update --init --recursive`
- Required for transcoder builds and testing

## File Format Support
- **Input**: OBJ, STL, PLY
- **Output**: OBJ, STL, PLY
- **Transcoder**: glTF compression/decompression

## Key Files and Directories
- `src/draco/core/` - Core data types and utilities
- `src/draco/compression/` - Main compression logic
- `src/draco/tools/` - Command-line utilities
- `javascript/` - Pre-built JS/WASM files
- `testdata/` - Test geometry files
- `cmake/` - Build system configuration

## Rust Migration Plan

Draco is undergoing a systematic migration from C++ to Rust for improved memory safety, type safety, and maintainability. This is a bottom-up migration strategy that maintains full C++ compatibility during the transition.

### Key Documents
- **[PLAN_DESCRIPTION.md](./PLAN_DESCRIPTION.md)** - Comprehensive migration strategy and phase breakdown
- **[PLAN.yaml](./PLAN.yaml)** - Structured migration timeline and component mapping
- **[CPP_INTEGRATION.md](./CPP_INTEGRATION.md)** - Detailed C++ integration patterns and ABI layer design

### Migration Status
- ✅ **Phase 1: Core Foundation** (Completed) - 47 tests passing
- 🔄 **Phase 2: Buffer and Stream Management** (In Progress)
- ⏳ **Phase 3: Attribute System** (Pending)
- ⏳ **Phase 4: Data Structures** (Pending)
- ⏳ **Phase 5: Compression Pipeline** (Pending)
- ⏳ **Phase 6: I/O and Tools** (Pending)

### Rust Workspace Structure
```
crates/
├── draco-core/          # Core utilities (Phase 1)
├── draco-attributes/    # Attribute system (Phase 3)
├── draco-compression/   # Compression algorithms (Phase 5)
├── draco-io/           # File I/O and parsers (Phase 6)
├── draco-point-cloud/  # Point cloud structures (Phase 4)
├── draco-mesh/         # Mesh structures (Phase 4)
└── draco-tools/        # CLI tools (Phase 6)
```

### Integration Strategy
- **C ABI Layer**: Rust components exposed through C-compatible interface
- **Feature Flags**: `DRACO_RUST_CORE`, `DRACO_RUST_IO`, etc. for gradual adoption
- **Build Integration**: CMake integration with cargo-cbuild for static libraries
- **Parallel Testing**: Both C++ and Rust implementations tested for identical output

## Common Issues
1. **Build errors**: Ensure out-of-source builds
2. **Missing submodules**: Run `git submodule update --init --recursive`
3. **Emscripten builds**: Verify EMSCRIPTEN environment variable
4. **Transcoder builds**: Requires additional third-party dependencies
5. **CMake warning**: Policy CMP0148 warning can be ignored (uses deprecated FindPythonInterp)
6. **Rust builds**: Install cargo-cbuild with `cargo install cargo-c` for C integration