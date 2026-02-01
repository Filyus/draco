# How Draco Works

Draco is a library for compressing and decompressing 3D geometric meshes and point clouds. This document explains the core algorithms and data structures that make Draco efficient.

## Table of Contents

1. [Overview](#overview)
2. [Data Structures](#data-structures)
3. [Mesh Encoding](#mesh-encoding)
4. [Point Cloud Encoding](#point-cloud-encoding)
5. [Attribute Encoding](#attribute-encoding)
6. [Entropy Coding](#entropy-coding)
7. [Decoding Process](#decoding-process)

---

## Overview

Draco achieves high compression ratios through three main techniques:

1. **Connectivity Compression** - Efficiently encodes mesh topology (how vertices connect to form triangles)
2. **Attribute Quantization** - Reduces precision of vertex attributes (positions, normals, UVs) to configurable levels
3. **Prediction Schemes** - Predicts attribute values based on neighboring vertices, encoding only the residuals
4. **Entropy Coding** - Uses rANS (range Asymmetric Numeral Systems) for final bitstream compression

### Compression Pipeline

```
Input Mesh
    │
    ▼
┌─────────────────────┐
│  Connectivity       │  ← Edgebreaker or Sequential encoding
│  Encoding           │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Attribute          │  ← Quantization + Prediction
│  Encoding           │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Entropy            │  ← rANS coding
│  Coding             │
└─────────────────────┘
    │
    ▼
Compressed .drc file
```

---

## Data Structures

### Corner Table

The **Corner Table** is Draco's primary mesh representation. It provides O(1) traversal of mesh connectivity.

```
Corner: An angle of a triangle at a vertex
        Each triangle has 3 corners (indices 0, 1, 2)

For triangle T with corners c0, c1, c2:
  - corner_to_vertex[c] → vertex index
  - opposite_corner[c]  → corner in adjacent triangle
  - next(c) = (c + 1) mod 3 within same triangle
  - prev(c) = (c + 2) mod 3 within same triangle
```

**Visual Example:**
```
        v2
       /  \
      /    \
   c2/      \c1
    /   T0   \
   /          \
  v0----c0----v1
       |
   opposite(c0)
       |
  v3----c3----v1
    \   T1   /
     \      /
    c5\    /c4
       \  /
        v4
```

### Key Operations

| Operation | Description | Complexity |
|-----------|-------------|------------|
| `next(c)` | Next corner in same face | O(1) |
| `prev(c)` | Previous corner in same face | O(1) |
| `opposite(c)` | Opposite corner across edge | O(1) |
| `vertex(c)` | Vertex at corner | O(1) |
| `face(c)` | Face containing corner | O(1) |
| `swing_right(c)` | `next(opposite(next(c)))` | O(1) |
| `swing_left(c)` | `prev(opposite(prev(c)))` | O(1) |

---

## Mesh Encoding

Draco supports two connectivity encoding methods:

### 1. Sequential Encoding

Simple but less efficient. Stores:
- Vertex count
- Face count  
- Face indices as a flat array

Used for: Small meshes, non-manifold geometry, or when speed is prioritized over compression.

### 2. Edgebreaker Encoding

A sophisticated algorithm that achieves near-optimal connectivity compression (~1-2 bits per triangle).

#### How Edgebreaker Works

Edgebreaker traverses the mesh using a depth-first search, encoding the topology using just 5 symbols:

| Symbol | Meaning | Bits |
|--------|---------|------|
| **C** | Connect to a new vertex | ~1 bit |
| **L** | Turn left (vertex already visited) | ~1 bit |
| **R** | Turn right (vertex already visited) | ~1 bit |
| **S** | Split - creates a branch | ~2 bits |
| **E** | End - closes a hole | ~2 bits |

#### Traversal Algorithm

```
1. Start at a boundary edge or arbitrary triangle
2. Mark the active triangle as visited
3. Check the opposite triangle across the "gate" edge:
   
   If opposite is:
   - Unvisited, no vertices seen → emit C, push new gate
   - Unvisited, left vertex seen → emit R, push new gate  
   - Unvisited, right vertex seen → emit L, push new gate
   - Unvisited, both vertices seen → emit S, split into two branches
   - Already visited or boundary → emit E, pop from stack

4. Repeat until all triangles visited
```

#### Example Encoding

```
Input: 4-vertex quad (2 triangles)

    v2----v3
    | \    |
    |  \   |
    |   \  |
    |    \ |
    v0----v1

Traversal:
  Start at face 0 (v0, v1, v2)
  → emit C (new vertex v2)
  Move to face 1 (v1, v3, v2)  
  → emit C (new vertex v3)
  No more faces
  → emit E (end)

Symbol sequence: [C, C, E]
```

#### Vertex Ordering

Edgebreaker produces vertices in **traversal order**, not original order. The encoder builds a `vertex_to_data_map` that maps:
- Original vertex index → Position in encoded stream

---

## Point Cloud Encoding

For point clouds (no connectivity), Draco uses **KD-Tree** based encoding:

1. Build a KD-tree over all points
2. Encode the tree structure
3. Quantize point positions relative to tree cells
4. Apply prediction within tree neighborhoods

---

## Attribute Encoding

### Quantization

Floating-point attributes are quantized to integers:

```
quantized_value = round((value - min) / (max - min) * (2^bits - 1))
```

Common quantization levels:
- Positions: 11-14 bits (millimeter precision at room scale)
- Normals: 8-10 bits
- UVs: 10-12 bits
- Colors: 8 bits per channel

### Prediction Schemes

Instead of storing raw values, Draco stores **corrections** (residuals) from predicted values:

```
correction = actual_value - predicted_value
```

Corrections are typically small and compress well.

#### 1. Parallelogram Prediction

For mesh attributes, uses the parallelogram rule:

```
Given triangle with vertices A, B, C where C is being encoded:

    C (predicted)
   /|\
  / | \
 /  |  \
A---+---B
    |
    D (opposite vertex)

predicted_C = A + B - D
correction = actual_C - predicted_C
```

This exploits the fact that meshes are often smooth surfaces.

#### 2. Difference Prediction

For sequential/point cloud data:
```
predicted[i] = value[i-1]
correction[i] = value[i] - value[i-1]
```

#### 3. Multi-Parallelogram Prediction

Averages predictions from multiple adjacent parallelograms for smoother results.

### Prediction Encoding Flow

```
┌──────────────┐
│ Original     │
│ Attribute    │
│ Values       │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Reorder by   │  ← Use vertex_to_data_map from connectivity
│ Traversal    │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Quantize     │  ← Float → Int with configurable bits
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Compute      │  ← Apply parallelogram or other scheme
│ Predictions  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Calculate    │  ← correction = actual - predicted
│ Corrections  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Transform    │  ← Convert signed → unsigned (zigzag)
│ to Unsigned  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Entropy      │  ← rANS encoding
│ Encode       │
└──────────────┘
```

---

## Entropy Coding

### rANS (Range Asymmetric Numeral Systems)

Draco uses rANS for final compression. rANS achieves near-entropy compression with fast encode/decode.

#### Key Concepts

1. **Symbol Frequencies**: Count occurrences of each symbol
2. **Probability Table**: Normalize frequencies to sum to power of 2
3. **State Machine**: Encode symbols by transforming a state value

```
Encoding:
  state = initial_state
  for each symbol:
    state = encode_symbol(state, symbol, probability_table)
  output final state

Decoding:
  state = read_final_state()
  for i in reverse:
    symbol, state = decode_symbol(state, probability_table)
```

### Bit Encoding

For simple binary data, Draco uses direct bit packing with optional RLE (Run-Length Encoding).

---

## Decoding Process

Decoding reverses the encoding pipeline:

```
Compressed .drc file
    │
    ▼
┌─────────────────────┐
│  Parse Header       │  ← Version, encoder type, counts
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Decode             │  ← Rebuild corner table
│  Connectivity       │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Decode             │  ← rANS decode → corrections
│  Attributes         │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Apply              │  ← actual = predicted + correction
│  Predictions        │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Dequantize         │  ← Int → Float
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Remap              │  ← Traversal order → point order
│  Attributes         │
└─────────────────────┘
    │
    ▼
Output Mesh
```

### Edgebreaker Decoding

The decoder reconstructs the corner table from the symbol sequence:

1. Read symbols (C, L, R, S, E)
2. Maintain active boundary as a stack of edges
3. For each symbol:
   - **C**: Create new vertex, extend boundary
   - **L/R**: Connect to existing vertex, close one side
   - **S**: Split boundary into two
   - **E**: Merge two boundary sections

### Attribute Mapping

After decoding, attributes are in **traversal order**. The decoder builds:
- `point_to_attribute_map`: Maps mesh points → attribute value indices

This mapping ensures that even though encoder and decoder may traverse differently, the final mesh has correct attribute values at each vertex.

---

## File Format

### Header Structure

```
Bytes   Field
─────   ─────
0-4     Magic: "DRACO"
5       Major version
6       Minor version  
7       Encoder type (0=point cloud, 1=mesh)
8       Encoder method (0=sequential, 1=edgebreaker)
9       Flags
10-13   Face count (for meshes)
14-17   Vertex/point count
18+     Attribute descriptors
```

### Attribute Descriptor

```
Bytes   Field
─────   ─────
0       Attribute type (position, normal, color, etc.)
1       Data type (float32, int32, etc.)
2       Component count (3 for xyz, 2 for uv, etc.)
3       Normalized flag
4-7     Unique ID
```

---

## Performance Characteristics

### Compression Ratios

| Content Type | Typical Ratio |
|--------------|---------------|
| Simple geometry | 10:1 - 20:1 |
| Complex meshes | 5:1 - 15:1 |
| Point clouds | 8:1 - 12:1 |
| With textures | 3:1 - 8:1 |

### Speed (typical hardware)

| Operation | Speed |
|-----------|-------|
| Encoding | 1-5 MB/s |
| Decoding | 10-50 MB/s |
| WASM Decoding | 5-20 MB/s |

---

## References

- [Edgebreaker Paper](https://www.cc.gatech.edu/~jarek/papers/EdgeBreaker.pdf) - Rossignac, 1999
- [Draco GitHub](https://github.com/google/draco) - Google's original C++ implementation
- [rANS Paper](https://arxiv.org/abs/1402.3392) - Duda, 2014
- [glTF Draco Extension](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_draco_mesh_compression/README.md)
