# Draco Implementation Status & Analysis

## Current Status (2025-01-25)

The Rust implementation of Draco is in a highly active debugging phase, specifically focused on **Edgebreaker Connectivity Decoding** and **Attribute Data Synchronization**.

### ✅ Completed & Verified
-   **Bitstream Decoding**: RANS bit decoding and basic symbol stream parsing are functional.
-   **Parallelogram Prediction**: Integer overflow issues fixed. Prediction logic corrected.
-   **Attribute Data Mapping**: `vertex_to_data_map` logic aligned between Encoder and Decoder.
-   **Symbol Stream Parsing**: Confirmed that `decode_symbol_stream` yields symbols in the correct order without needing manual reversal.
-   **DFS Traversal**: The Decoder now uses `generate_point_ids_and_corners_dfs` seeded by connectivity corners to match the Encoder's traversal order.
-   **Vertex Count Mismatch**: Solved! 
    -   **Fix 1**: `Split` symbol now merges the two vertices at the split point (using `merge_vertices`).
    -   **Fix 2**: `Init Face` logic respects `is_interior` flag (creates new vertices/face only if interior; otherwise just seeds traversal).
    -   **Result**: Decoded vertex count matches original exactly (e.g. 25 vertices for 5x5 grid).

### 🚧 Active Investigation / In Progress
-   **Attribute Drift (Position Mismatch)**:
    -   **Issue**: Decoded vertices have correct count but wrong positions (e.g., Point 0 is missing, data shifted).
    -   **Hypothesis**: The `point_ids` sequence used for delta decoding differs between Encoder and Decoder. Since the topology graph was modified (merged vertices), the DFS traversal might verify vertices in a slightly different order.
    -   **Next Step**: Compare strict `point_ids` sequence log between Encoder and Decoder.

---

## Edgebreaker Symbols Reference

From `mesh_edgebreaker_shared.h`, the Edgebreaker algorithm encodes mesh topology using 5 symbols based on the visited state of neighboring faces:

```
     *-------*          *-------*          *-------*
    / \     / \        / \     / \        / \     / \
   /   \   /   \      /   \   /   \      /   \   /   \
  /     \ /     \    /     \ /     \    /     \ /     \
 *-------v-------*  *-------v-------*  *-------v-------*
  \     /x\     /          /x\     /    \     /x\
   \   /   \   /          /   \   /      \   /   \
    \ /  C  \ /          /  L  \ /        \ /  R  \
     *-------*          *-------*          *-------*

     *       *
    / \     / \
    /   \   /   \
    /     \ /     \
 *-------v-------*          v
  \     /x\     /          /x\
   \   /   \   /          /   \
    \ /  S  \ /          /  E  \
     *-------*          *-------*
```

| Symbol | Value | Name | New Vertices | Stack Op | Description |
|--------|-------|------|--------------|----------|-------------|
| **C** | 0 | Continue | 0 | - | Interior vertex, both neighbors visited, continue |
| **S** | 1 | Split | 0 (Merge) | push 2 | Both neighbors unvisited, split traversal. **MERGES VERTICES**. |
| **L** | 3 | Left | 1 | - | Right neighbor visited, go left |
| **R** | 5 | Right | 1 | - | Left neighbor visited, go right |
| **E** | 7 | End | 3 | push 1 | Both neighbors visited, start new component |

**Key Insight**: The bit pattern (C=0, S=100, L=110, R=101, E=111) enables efficient entropy encoding.

---

## Architectural Flow Analysis

### 1. The "Spirale Reversi" Algorithm

From `src/draco/compression/mesh/mesh_edgebreaker_decoder_impl.h` lines 32-50:
> The implementation is based on the algorithm presented in Isenburg et al'02 "Spirale Reversi: Reverse decoding of the Edgebreaker encoding". The encoding is still based on the standard edgebreaker method... One difference is caused by the properties of the spirale reversi algorithm that decodes the symbols from the last one to the first one. To make the decoding more efficient, we encode all symbols in the reverse order, therefore the decoder can process them one by one.

**Critical Implication**: The encoder processes symbols **top-down** (Root→Leaf), but stores them in **reverse order**. The decoder then processes them **bottom-up** (Leaf→Root), maintaining valid connectivity at all times.

---

### 2. Encoder Flow (`src/draco/compression/mesh/mesh_edgebreaker_encoder_impl.cc`)

The encoder's `EncodeConnectivity()` method (lines 269-436) orchestrates the entire topology encoding process through multiple phases.

#### Phase 1: Initialization (lines 300-328)

```cpp
// 1. Create corner table from mesh
corner_table_ = CreateCornerTableFromPositionAttribute(mesh_);

// 2. Find holes and assign hole IDs to boundary vertices
FindHoles();  // -> vertex_hole_id_[vertex] = hole_id or -1

// 3. Initialize attribute data structures
InitAttributeData();

// 4. Reset encoding state
visited_faces_.assign(num_faces, false);
visited_vertex_ids_.assign(num_vertices, false);
processed_connectivity_corners_.clear();
processed_connectivity_corners_.reserve(num_faces);
init_face_connectivity_corners.clear();
```

#### Phase 2: Component Traversal (lines 339-400)

For each unvisited corner in the mesh, the encoder initiates a new connected component traversal:

```cpp
for (CornerIndex c_id(0); c_id < num_corners; ++c_id) {
    const FaceIndex face_id = corner_table_->Face(c_id);
    if (visited_faces_[face_id]) continue;

    // Determine if this component starts from interior or boundary
    CornerIndex start_corner;
    const bool interior_config = FindInitFaceConfiguration(face_id, &start_corner);
    traversal_encoder_.EncodeStartFaceConfiguration(interior_config);

    if (interior_config) {
        // Interior start: mark all 3 vertices as visited
        visited_vertex_ids_[vert_id] = true;
        visited_vertex_ids_[next_vert_id] = true;
        visited_vertex_ids_[prev_vert_id] = true;
        visited_faces_[face_id] = true;

        // Store corner for later attribute encoding
        init_face_connectivity_corners.push_back(corner_table_->Next(corner_index));

        // Start DFS from opposite face
        EncodeConnectivityFromCorner(opposite_corner);
    } else {
        // Boundary start: encode hole first, then DFS
        EncodeHole(corner_table_->Next(start_corner), true);
        EncodeConnectivityFromCorner(start_corner);
    }
}
```

#### Phase 3: DFS Traversal (`EncodeConnectivityFromCorner`, lines 506-617)

This is the core topology encoding algorithm. At each step, the encoder:

1. **Processes the current face** (line 529):
   ```cpp
   processed_connectivity_corners_.push_back(corner_id);  // CRITICAL: Record visitation order
   traversal_encoder_.NewCornerReached(corner_id);
   ```

2. **Examines the tip vertex** to determine the symbol:
   - **New interior vertex** → `TOPOLOGY_C` (Continue)
   - **Both neighbors visited** → `TOPOLOGY_E` (End)
   - **Right neighbor visited** → `TOPOLOGY_R` (go Right)
   - **Left neighbor visited** → `TOPOLOGY_L` (go Left)
   - **Neither visited** → `TOPOLOGY_S` (Split)

3. **Updates traversal stack** accordingly:
   - C: Continue to right corner
   - R/L: Move to unvisited side
   - S: Split - push right face, then left face
   - E: Pop stack, end current branch

**Key locations in `src/draco/compression/mesh/mesh_edgebreaker_encoder_impl.cc`:**
- Line 529: `processed_connectivity_corners_.push_back(corner_id)` - Records visitation order
- Line 540: `traversal_encoder_.EncodeSymbol(TOPOLOGY_C)` - Encodes C symbol
- Line 569: `traversal_encoder_.EncodeSymbol(TOPOLOGY_E)` - Encodes E symbol
- Line 573: `traversal_encoder_.EncodeSymbol(TOPOLOGY_R)` - Encodes R symbol
- Line 586: `traversal_encoder_.EncodeSymbol(TOPOLOGY_L)` - Encodes L symbol
- Line 590: `traversal_encoder_.EncodeSymbol(TOPOLOGY_S)` - Encodes S symbol

#### Phase 4: Corner Order Reversal (lines 401-409) - **THE KEY INSIGHT**

```cpp
// CRITICAL: Reverse the DFS visitation order to match decoder's face creation order
std::reverse(processed_connectivity_corners_.begin(),
             processed_connectivity_corners_.end());

// Append init face corners (they will be processed last by decoder)
processed_connectivity_corners_.insert(processed_connectivity_corners_.end(),
                                       init_face_connectivity_corners.begin(),
                                       init_face_connectivity_corners.end());
```

**Why this works:**
- Encoder DFS visits faces in: `[root, child1, child2, ..., leaf]`
- After reversal: `[leaf, ..., child2, child1, root]`
- Decoder creates faces sequentially: Face 0, Face 1, Face 2, ...
- Therefore: Decoder Face 0 = Encoder's leaf, Decoder Face N = Encoder's root
- This alignment is crucial for attribute encoding/decoding synchronization!

#### Phase 5: Attribute Sequencer Setup (lines 92-112)

```cpp
CreateVertexTraversalSequencer(encoding_data):
    traversal_sequencer = new MeshTraversalSequencer<TraverserT>(mesh_, encoding_data)
    att_observer = MeshAttributeIndicesEncodingObserver(corner_table_, mesh_,
                                                         traversal_sequencer,
                                                         encoding_data)
    att_traverser = DepthFirstTraverser<...>()
    att_traverser.Init(corner_table_.get(), att_observer)

    // CRITICAL: Set the corner order to match decoder's sequential face processing
    traversal_sequencer->SetCornerOrder(processed_connectivity_corners_)
    traversal_sequencer->SetTraverser(att_traverser)
    return sequencer
```

**File reference:** `src/draco/compression/mesh/mesh_edgebreaker_encoder_impl.cc:109`

---

### 3. Decoder Flow (`src/draco/compression/mesh/mesh_edgebreaker_decoder_impl.cc`)

The decoder's `DecodeConnectivity()` method (lines 247-443, 535-974) reconstructs the mesh topology symbol-by-symbol.

#### Phase 1: Initialization (lines 247-443)

```cpp
// Decode header information
DecodeVarint(&num_encoded_vertices, decoder_->buffer());
DecodeVarint(&num_faces, decoder_->buffer());
decoder_->buffer()->Decode(&num_attribute_data);
DecodeVarint(&num_encoded_symbols, decoder_->buffer());
DecodeVarint(&num_encoded_split_symbols, decoder_->buffer());

// Initialize corner table with capacity
corner_table_->Reset(num_faces, num_encoded_vertices + num_encoded_split_symbols);

// Mark all vertices as holes initially (only C marks interior)
is_vert_hole_.assign(num_encoded_vertices + num_encoded_split_symbols, true);

// Decode topology split events and hole events
DecodeHoleAndTopologySplitEvents(decoder_->buffer());
```

#### Phase 2: Symbol-by-Symbol Decoding (lines 535-974)

The decoder processes symbols in **sequential order**, creating faces as it goes:

```cpp
int num_faces = 0;
for (int symbol_id = 0; symbol_id < num_symbols; ++symbol_id) {
    const FaceIndex face(num_faces++);  // Face 0, 1, 2, 3, ...
    const CornerIndex corner(3 * face.value());  // Corner 0, 3, 6, 9, ...

    const uint32_t symbol = traversal_decoder_.DecodeSymbol();

    switch (symbol) {
        case TOPOLOGY_C:
            // Connect two boundary edges, reuse existing vertex
            SetOppositeCorners(corner_a, corner + 1);
            SetOppositeCorners(corner_b, corner + 2);
            is_vert_hole_[vertex_x] = false;  // Mark as interior
            break;

        case TOPOLOGY_R:
        case TOPOLOGY_L:
            // Create one new vertex
            VertexIndex new_vert = corner_table_->AddNewVertex();
            SetOppositeCorners(opp_corner, corner_a);
            break;

        case TOPOLOGY_S:
            // MERGE two vertices at corners p and n
            VertexIndex vertex_p = corner_table_->Vertex(corner_table_->Previous(corner_a));
            VertexIndex vertex_n = corner_table_->Vertex(corner_table_->Next(corner_b));
            traversal_decoder_.MergeVertices(vertex_p, vertex_n);  // CRITICAL
            corner_table_->MakeVertexIsolated(vertex_n);  // Remove vertex_n
            break;

        case TOPOLOGY_E:
            // Create three new vertices (start of new component)
            corner_table_->AddNewVertex();  // x 3
            active_corner_stack.push_back(corner);
            break;
    }

    active_corner_stack.back() = corner;
}
```

**Key locations in `src/draco/compression/mesh/mesh_edgebreaker_decoder_impl.cc`:**
- Line 563: `const FaceIndex face(num_faces++)` - Sequential face creation
- Line 609: `const CornerIndex corner(3 * face.value())` - Corner calculation
- Line 628: `is_vert_hole_[vertex_x] = false` - Mark C vertices as interior
- Line 672: `corner_table_->AddNewVertex()` - Create new vertex for R/L
- Line 749: `traversal_decoder_.MergeVertices(vertex_p, vertex_n)` - **MERGE** for S symbol
- Line 768: `corner_table_->MakeVertexIsolated(vertex_n)` - Remove merged vertex
- Line 775-781: Create 3 vertices for E symbol

#### Phase 3: Init Face Processing (lines 849-932)

After processing all symbols, the decoder handles init faces:

```cpp
while (!active_corner_stack.empty()) {
    const CornerIndex corner = active_corner_stack.pop_back();
    const bool interior = traversal_decoder_.DecodeStartFaceConfiguration();

    if (interior) {
        // Create new face connecting to 3 existing faces
        const FaceIndex face(num_faces++);
        const CornerIndex new_corner(3 * face.value());

        SetOppositeCorners(new_corner, corner);
        SetOppositeCorners(new_corner + 1, corner_b);
        SetOppositeCorners(new_corner + 2, corner_c);

        // Map to existing vertices (no new vertices created)
        corner_table_->MapCornerToVertex(new_corner, vert_x);
        corner_table_->MapCornerToVertex(new_corner + 1, vert_p);
        corner_table_->MapCornerToVertex(new_corner + 2, vert_n);

        init_corners_.push_back(new_corner);
    } else {
        // Boundary configuration - just record the corner
        init_corners_.push_back(corner);
    }
}
```

**Critical outcome:** Init faces receive the **highest face IDs** (N, N+1, N+2, ...), placing them **after** all symbol-created faces (0, 1, ..., N-1).

#### Phase 4: Attribute Sequencer Setup (lines 106-126)

```cpp
CreateVertexTraversalSequencer(encoding_data):
    traversal_sequencer = new MeshTraversalSequencer<TraverserT>(mesh, encoding_data)
    att_observer = MeshAttributeIndicesEncodingObserver(corner_table_, mesh,
                                                         traversal_sequencer,
                                                         encoding_data)
    att_traverser = DepthFirstTraverser<...>()
    att_traverser.Init(corner_table_.get(), att_observer)

    // NO SetCornerOrder() call! Uses default sequential processing

    traversal_sequencer->SetTraverser(att_traverser)
    return sequencer
```

**File reference:** `src/draco/compression/mesh/mesh_edgebreaker_decoder_impl.cc:107-126`

---

### 4. The Traversal Synchronization Mechanism

The synchronization between encoder and decoder attribute processing is achieved through `MeshTraversalSequencer`.

#### Encoder: Explicit Corner Order

**File:** `src/draco/compression/mesh/traverser/mesh_traversal_sequencer.h` lines 76-98

```cpp
bool GenerateSequenceInternal() override {
    traverser_.OnTraversalStart();
    if (corner_order_) {
        // Encoder path: Process corners in the specified (reversed) order
        for (uint32_t i = 0; i < corner_order_->size(); ++i) {
            ProcessCorner(corner_order_->at(i));
        }
    }
    traverser_.OnTraversalEnd();
    return true;
}
```

The encoder calls `SetCornerOrder()` with the reversed `processed_connectivity_corners_`.

#### Decoder: Default Sequential Order

**File:** `src/draco/compression/mesh/traverser/mesh_traversal_sequencer.h` lines 88-94

```cpp
bool GenerateSequenceInternal() override {
    traverser_.OnTraversalStart();
    if (corner_order_) {
        // Not taken by decoder
    } else {
        // Decoder path: Process faces sequentially
        const int32_t num_faces = traverser_.corner_table()->num_faces();
        for (int i = 0; i < num_faces; ++i) {
            ProcessCorner(CornerIndex(3 * i));  // Corner 0, 3, 6, 9, ...
        }
    }
    traverser_.OnTraversalEnd();
    return true;
}
```

The decoder **does not** call `SetCornerOrder()`, so it defaults to sequential face processing.

#### Why This Matches

| Stage | Encoder Face Order | Decoder Face Order | Match? |
|-------|-------------------|-------------------|--------|
| Symbol faces | DFS order → reversed | Sequential 0,1,2,... | ✓ |
| Init faces | Appended last | IDs N, N+1, ... | ✓ |

**Concrete example:**
- Encoder DFS visits: `[F0, F1, F2, F3]` where F0=init, F1,F2,F3=DFS
- After reversal + append: `[F3, F2, F1, F0]`
- Decoder creates faces sequentially: `Face 0=F3', Face 1=F2', Face 2=F1', Face 3=F0'`
- Both process corners in same order: `[3, 6, 9, 0]`

---

### 5. DFS Traversal Implementation

**File:** `src/draco/compression/mesh/traverser/depth_first_traverser.h`

Both encoder and decoder use the same `DepthFirstTraverser` for attribute sequencing:

```cpp
bool TraverseFromCorner(CornerIndex corner_id) {
    corner_traversal_stack_.clear();
    corner_traversal_stack_.push_back(corner_id);

    while (!corner_traversal_stack_.empty()) {
        corner_id = corner_traversal_stack_.back();
        FaceIndex face_id(corner_id.value() / 3);

        if (this->IsFaceVisited(face_id)) {
            corner_traversal_stack_.pop_back();
            continue;
        }

        while (true) {
            this->MarkFaceVisited(face_id);
            this->traversal_observer().OnNewFaceVisited(face_id);

            const VertexIndex vert_id = this->corner_table()->Vertex(corner_id);
            if (!this->IsVertexVisited(vert_id)) {
                this->MarkVertexVisited(vert_id);
                this->traversal_observer().OnNewVertexVisited(vert_id, corner_id);
                if (!this->corner_table()->IsOnBoundary(vert_id)) {
                    corner_id = this->corner_table()->GetRightCorner(corner_id);
                    face_id = FaceIndex(corner_id.value() / 3);
                    continue;  // Continue traversal
                }
            }

            // Check neighboring faces
            const CornerIndex right_corner_id = this->corner_table()->GetRightCorner(corner_id);
            const CornerIndex left_corner_id = this->corner_table()->GetLeftCorner(corner_id);

            if (this->IsFaceVisited(right_face_id) && this->IsFaceVisited(left_face_id)) {
                corner_traversal_stack_.pop_back();
                break;
            } else if (this->IsFaceVisited(right_face_id)) {
                corner_id = left_corner_id;
            } else if (this->IsFaceVisited(left_face_id)) {
                corner_id = right_corner_id;
            } else {
                // Split: process right first, then left
                corner_traversal_stack_.back() = left_corner_id;
                corner_traversal_stack_.push_back(right_corner_id);
                break;
            }
        }
    }
    return true;
}
```

**Key observation:** The DFS traversal mirrors the connectivity encoding exactly, ensuring vertices are visited in the same order.

---

### 6. Attribute Encoding Observer

**File:** `src/draco/compression/mesh/traverser/mesh_attribute_indices_encoding_observer.h`

This observer builds the `point_ids` sequence during traversal:

```cpp
inline void OnNewVertexVisited(VertexIndex vertex, CornerIndex corner) {
    // Extract point_id from mesh face data
    const PointIndex point_id = mesh_->face(FaceIndex(corner.value() / 3))[corner.value() % 3];

    // Build the sequence that decoder will use
    sequencer_->AddPointId(point_id);

    // Record corner for prediction schemes
    encoding_data_->encoded_attribute_value_index_to_corner_map.push_back(corner);

    // Map vertex to encoding order
    encoding_data_->vertex_to_encoded_attribute_value_index_map[vertex.value()] =
        encoding_data_->num_values;

    encoding_data_->num_values++;
}
```

---

### 7. The Critical Invariant

**The `point_ids` sequence MUST be identical between encoder and decoder.**

This is achieved through:

| Aspect | Encoder | Decoder |
|--------|---------|---------|
| **Corner order source** | `processed_connectivity_corners_` (reversed DFS + init) | Sequential face IDs |
| **SetCornerOrder call?** | ✓ Yes (line 109) | ✗ No |
| **Default behavior** | Uses specified order | Sequential: 0, 3, 6, 9, ... |
| **Symbol face corners** | Reversed DFS order | Sequential: 0, 3, 6, ... |
| **Init face corners** | Appended last | Highest IDs: 3N, 3(N+1), ... |
| **Result** | `[reversed_DFS..., init...]` | `[symbol_faces..., init_faces...]` |

**Verification:**
- Encoder DFS: visits faces in topological order
- After reversal: last-visited face first
- Decoder sequential: Face 0 = first symbol = encoder's last-visited
- Init faces: both process last (appended by encoder, high IDs by decoder)

---

### 8. Critical Data Structures

#### `MeshAttributeIndicesEncodingData`

**File:** `src/draco/compression/attributes/mesh_attribute_indices_encoding_data.h`

```cpp
struct MeshAttributeIndicesEncodingData {
    // For each encoded value, which corner was used?
    // Used by prediction schemes to find reference corners
    std::vector<CornerIndex> encoded_attribute_value_index_to_corner_map;

    // For each vertex, which encoded value ID?
    // -1 if not encoded/decoded yet
    std::vector<int32_t> vertex_to_encoded_attribute_value_index_map;

    int num_values;  // Total encoded/decoded attribute values

    void Init(int num_vertices) {
        vertex_to_encoded_attribute_value_index_map.resize(num_vertices);
        encoded_attribute_value_index_to_corner_map.reserve(num_vertices);
    }
};
```

**Usage:**
- **Encoding**: Populated during DFS traversal by `MeshAttributeIndicesEncodingObserver`
- **Decoding**: Pre-initialized with -1, filled during attribute decoder DFS traversal
- **Purpose**: Enables prediction schemes (e.g., parallelogram) to find previously encoded values
