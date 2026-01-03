# Draco Core (Rust)

This is a Rust port of the core Draco library.

## Implemented Features

### Core
- `Status` (Error handling)
- `DataBuffer`
- `DataType`
- `EncoderBuffer` / `DecoderBuffer`
- `BitEncoder` / `BitDecoder`

### Geometry
- `PointCloud`
- `Mesh`
- `GeometryAttribute`
- `PointAttribute`
- `GeometryIndices` (PointIndex, FaceIndex, etc.)

### Compression
- `AnsCoder` / `AnsDecoder` (rANS entropy coding)
- `RAnsBitEncoder` / `RAnsBitDecoder`

## Usage

```rust
use draco_core::{PointCloud, Mesh, PointAttribute, GeometryAttributeType, DataType};

let mut mesh = Mesh::new();
let mut pos_att = PointAttribute::new();
pos_att.init(GeometryAttributeType::Position, 3, DataType::Float32, false, 100);
mesh.add_attribute(pos_att);
```
