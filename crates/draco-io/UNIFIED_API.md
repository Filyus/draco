# Draco I/O - Unified Writer Traits

This document describes the unified API introduced through the `Writer` and `Reader` traits.

## Overview

All format writers now implement a common `Writer` trait, providing a consistent interface across OBJ, PLY, FBX, and glTF/GLB formats. This enables:

- **Polymorphic code**: Write functions that work with any format
- **Consistent API**: Same methods across all writers
- **Easy format switching**: Change output format with minimal code changes
- **Type safety**: Compile-time guarantees through Rust's type system

## Writer Trait

```rust
pub trait Writer: Sized {
    fn new() -> Self;
    fn add_mesh(&mut self, mesh: &Mesh, name: Option<&str>) -> io::Result<()>;
    fn write<P: AsRef<Path>>(&self, path: P) -> io::Result<()>;
    fn vertex_count(&self) -> usize;
    fn face_count(&self) -> usize;
}
```

### Implementations

| Writer | Format | Special Features |
|--------|--------|------------------|
| `ObjWriter` | Wavefront OBJ | Named groups, normals |
| `PlyWriter` | Stanford PLY | Colors, ASCII format |
| `FbxWriter` | Autodesk FBX | Optional zlib compression |
| `GltfWriter` | glTF 2.0 / GLB | Draco compression, multiple output formats |

## Basic Usage

### Simple Example

```rust
use draco_io::{Writer, ObjWriter};
use draco_core::mesh::Mesh;

let mesh: Mesh = /* ... */;

// Create writer
let mut writer = ObjWriter::new();

// Add mesh(es)
writer.add_mesh(&mesh, Some("MyMesh"))?;

// Write to file
writer.write("output.obj")?;
```

### Generic Function

Write format-agnostic code:

```rust
fn save_mesh<W: Writer>(mut writer: W, mesh: &Mesh, path: &str) -> io::Result<()> {
    writer.add_mesh(mesh, Some("Model"))?;
    println!("Vertices: {}, Faces: {}", 
        writer.vertex_count(), 
        writer.face_count());
    writer.write(path)
}

// Works with any format
save_mesh(ObjWriter::new(), &mesh, "out.obj")?;
save_mesh(PlyWriter::new(), &mesh, "out.ply")?;
save_mesh(FbxWriter::new(), &mesh, "out.fbx")?;
save_mesh(GltfWriter::new(), &mesh, "out.glb")?;
```

## Format-Specific Features

While the trait provides a common interface, each writer retains format-specific capabilities:

### OBJ Writer

```rust
let mut obj = ObjWriter::new();

// Trait methods
obj.add_mesh(&mesh, Some("Cube"));  // Named object groups

// Format-specific methods
obj.add_points(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);  // Point clouds
obj.add_point([0.5, 0.5, 0.5]);

obj.write("output.obj")?;
```

### PLY Writer

```rust
let mut ply = PlyWriter::new();

// Trait methods
ply.add_mesh(&mesh, None);  // Name ignored (PLY doesn't support it)

// Format-specific methods
ply.add_points_with_colors(
    &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
    &[[255, 0, 0, 255], [0, 255, 0, 255]]
);

println!("Has colors: {}", ply.has_colors());
ply.write("output.ply")?;
```

### FBX Writer

```rust
// Builder pattern with options
let mut fbx = FbxWriter::new()
    .with_compression(true)
    .with_compression_threshold(1000);

fbx.add_mesh(&mesh, Some("Character"));
fbx.add_mesh(&mesh2, Some("Weapon"));

println!("Compression: {}", fbx.is_compression_enabled());
fbx.write("output.fbx")?;
```

### glTF Writer

```rust
let mut gltf = GltfWriter::new();

// Trait methods (uses default 14-bit quantization)
gltf.add_mesh(&mesh, Some("Model"))?;

// Format-specific methods with custom quantization
gltf.add_draco_mesh(&high_quality_mesh, Some("Hero"), 16)?;

// Multiple output formats
gltf.write_glb("output.glb")?;                    // Binary GLB
gltf.write_gltf("out.gltf", "out.bin")?;         // Separate files
gltf.write_gltf_embedded("embedded.gltf")?;      // Pure text with base64
```

## PointCloudWriter Trait

For point cloud support (no faces):

```rust
pub trait PointCloudWriter: Writer {
    fn add_points(&mut self, points: &[[f32; 3]]);
    fn add_point(&mut self, point: [f32; 3]);
}
```

**Implementations**: `ObjWriter`, `PlyWriter`

```rust
use draco_io::PointCloudWriter;

let mut writer = ObjWriter::new();
writer.add_points(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
writer.write("points.obj")?;
```

## Reader Trait

All readers implement a common `Reader` trait:

```rust
pub trait Reader: Sized {
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self>;
    fn read_mesh(&mut self) -> io::Result<Mesh>;
    fn read_all_meshes(&mut self) -> io::Result<Vec<Mesh>>;
}
```

### Example

```rust
use draco_io::{Reader, ObjReader};

fn load_model<R: Reader>(path: &str) -> io::Result<Mesh> {
    let mut reader = R::open(path)?;
    reader.read_mesh()
}

// Works with any format
let mesh = load_model::<ObjReader>("model.obj")?;
let mesh = load_model::<FbxReader>("model.fbx")?;
```

## Migration Guide

### From Function-Based API

**Before:**
```rust
use draco_io::{write_obj_mesh, write_ply_mesh};

write_obj_mesh("out.obj", &mesh)?;
write_ply_mesh("out.ply", &mesh)?;
```

**After:**
```rust
use draco_io::{ObjWriter, PlyWriter, Writer};

let mut obj = ObjWriter::new();
obj.add_mesh(&mesh, None)?;
obj.write("out.obj")?;

let mut ply = PlyWriter::new();
ply.add_mesh(&mesh, None)?;
ply.write("out.ply")?;
```

**Note**: Convenience functions are still available for backward compatibility.

### Benefits of New API

1. **Multiple meshes**: Add several meshes before writing once
2. **Query capabilities**: Check vertex/face counts before writing
3. **Fluent interface**: Chain operations naturally
4. **Extensibility**: Easy to add new options per format

## Examples

See the `examples/` directory:

- `unified_api.rs` - Demonstrates trait usage across all formats
- `polymorphic.rs` - Runtime format selection with trait objects
- `fbx_demo.rs` - FBX-specific features with compression
- `obj_demo.rs` - OBJ point clouds and meshes
- `ply_demo.rs` - PLY with colors

Run examples:
```bash
cargo run --example unified_api --features compression
cargo run --example polymorphic --features compression
```

Round-trip test with Blender (optional)

- Requires Blender (with Python 'bpy' available).
- Set the path to your Blender executable in the `BLENDER_BIN` environment variable.

Example (Windows PowerShell):

```powershell
$Env:BLENDER_BIN = 'C:\Program Files\Blender Foundation\Blender\blender.exe'
cargo test --test roundtrip --features compression
```

If `BLENDER_BIN` is not set the test will be skipped (fast pass).

The test does the following:
1. Calls `tools/blender_roundtrip.py` via Blender to create a scene (two high-poly meshes) and export `scene.obj`, `scene.ply`, `scene.fbx`, `scene.glb`.
2. Uses the crate readers to read each file, writes them back using the writers, then calls the Blender script again to import the round-tripped files and report mesh and face counts.
3. Verifies that each round-tripped file contains at least one mesh and many faces (sanity check).

Note: Draco compression for GLB is attempted when Blender's glTF exporter supports it; test works either way.
## API Summary

### All Writers

| Method | OBJ | PLY | FBX | glTF | Notes |
|--------|-----|-----|-----|------|-------|
| `new()` | ✓ | ✓ | ✓ | ✓ | Constructor |
| `add_mesh()` | ✓ | ✓ | ✓ | ✓ | Via trait |
| `write()` | ✓ | ✓ | ✓ | ✓ | Via trait |
| `vertex_count()` | ✓ | ✓ | ✓ | ✓ | Via trait |
| `face_count()` | ✓ | ✓ | ✓ | ✓ | Via trait |

### Format-Specific

| Method | Writer | Description |
|--------|--------|-------------|
| `add_points()` | OBJ, PLY | Add point cloud |
| `add_points_with_colors()` | PLY | Colored points |
| `has_colors()` | PLY | Check color support |
| `has_normals()` | PLY | Check normal support |
| `with_compression()` | FBX | Enable zlib compression |
| `with_compression_threshold()` | FBX | Set compression threshold |
| `add_draco_mesh()` | glTF | Custom quantization |
| `write_glb()` | glTF | Binary GLB output |
| `write_gltf()` | glTF | Separate JSON + bin |
| `write_gltf_embedded()` | glTF | Pure text with base64 |

## Testing

All trait implementations include comprehensive tests:

```bash
cargo test --features compression
```

- 34 unit tests
- All formats tested
- Trait methods verified
- Format-specific features tested
