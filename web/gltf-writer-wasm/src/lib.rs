//! glTF/GLB Writer WASM module.
//!
//! Provides glTF 2.0 file generation functionality for web applications.
//! Supports both .gltf (JSON) and .glb (binary) formats with optional Draco compression.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input mesh data from JavaScript.
#[derive(Serialize, Deserialize, Clone)]
pub struct MeshInput {
    /// Mesh name
    pub name: Option<String>,
    /// Vertex positions as flat array [x0, y0, z0, x1, y1, z1, ...]
    pub positions: Vec<f32>,
    /// Face indices as flat array (triangles)
    pub indices: Vec<u32>,
    /// Vertex normals (optional)
    pub normals: Option<Vec<f32>>,
    /// Texture coordinates (optional)
    pub uvs: Option<Vec<f32>>,
}

/// Scene node input.
#[derive(Serialize, Deserialize, Clone)]
pub struct NodeInput {
    pub name: Option<String>,
    pub mesh_index: Option<usize>,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
    pub children: Vec<usize>,
}

/// Export options.
#[derive(Serialize, Deserialize, Default)]
pub struct ExportOptions {
    /// Use Draco compression
    pub use_draco: Option<bool>,
    /// Draco quantization bits for positions (default: 14)
    pub position_quantization: Option<i32>,
    /// Draco quantization bits for normals (default: 10)
    pub normal_quantization: Option<i32>,
    /// Draco quantization bits for UVs (default: 12)
    pub texcoord_quantization: Option<i32>,
    /// Output format: "glb" or "gltf"
    pub format: Option<String>,
}

/// Export result.
#[derive(Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    /// JSON content (for .gltf format or embedded)
    pub json_data: Option<String>,
    /// Binary data (for .glb format)
    pub binary_data: Option<Vec<u8>>,
    pub error: Option<String>,
}

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Get the version of this WASM module.
#[wasm_bindgen]
pub fn version() -> String {
    "0.1.0".to_string()
}

/// Get the module name.
#[wasm_bindgen]
pub fn module_name() -> String {
    "glTF Writer".to_string()
}

/// Get supported file extensions.
#[wasm_bindgen]
pub fn supported_extensions() -> Vec<String> {
    vec!["gltf".to_string(), "glb".to_string()]
}

/// Create glTF/GLB content from mesh data.
#[wasm_bindgen]
pub fn create_gltf(meshes_js: JsValue, options_js: JsValue) -> JsValue {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }
    
    let meshes: Vec<MeshInput> = match serde_wasm_bindgen::from_value::<Vec<MeshInput>>(meshes_js) {
        Ok(m) => {
            log(&format!("[GLTF_WRITER] Deserialized {} meshes from JS", m.len()));
            for (i, mesh) in m.iter().enumerate() {
                log(&format!("[GLTF_WRITER] Mesh {}: positions={}, indices={}, normals={:?}, uvs={:?}",
                    i, mesh.positions.len(), mesh.indices.len(),
                    mesh.normals.as_ref().map(|n: &Vec<f32>| n.len()),
                    mesh.uvs.as_ref().map(|u: &Vec<f32>| u.len())));
            }
            m
        }
        Err(e) => {
            let result = ExportResult {
                success: false,
                json_data: None,
                binary_data: None,
                error: Some(format!("Invalid mesh data: {}", e)),
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL);
        }
    };

    let options: ExportOptions = serde_wasm_bindgen::from_value(options_js).unwrap_or_default();
    
    // Catch any panics and convert to error result
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        create_gltf_internal(&meshes, &options)
    }));
    
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            ExportResult {
                success: false,
                json_data: None,
                binary_data: None,
                error: Some(format!("Internal error: {}", msg)),
            }
        }
    };
    
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Create glTF with scene graph.
#[wasm_bindgen]
pub fn create_gltf_with_scene(meshes_js: JsValue, nodes_js: JsValue, options_js: JsValue) -> JsValue {
    let meshes: Vec<MeshInput> = match serde_wasm_bindgen::from_value(meshes_js) {
        Ok(m) => m,
        Err(e) => {
            let result = ExportResult {
                success: false,
                json_data: None,
                binary_data: None,
                error: Some(format!("Invalid mesh data: {}", e)),
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL);
        }
    };

    let nodes: Vec<NodeInput> = match serde_wasm_bindgen::from_value(nodes_js) {
        Ok(n) => n,
        Err(e) => {
            let result = ExportResult {
                success: false,
                json_data: None,
                binary_data: None,
                error: Some(format!("Invalid node data: {}", e)),
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL);
        }
    };

    let options: ExportOptions = serde_wasm_bindgen::from_value(options_js).unwrap_or_default();
    let result = create_gltf_with_scene_internal(&meshes, &nodes, &options);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

fn create_gltf_internal(meshes: &[MeshInput], options: &ExportOptions) -> ExportResult {
    // Create default nodes for each mesh
    let nodes: Vec<NodeInput> = meshes
        .iter()
        .enumerate()
        .map(|(i, m)| NodeInput {
            name: m.name.clone(),
            mesh_index: Some(i),
            translation: None,
            rotation: None,
            scale: None,
            children: vec![],
        })
        .collect();

    create_gltf_with_scene_internal(meshes, &nodes, options)
}

fn create_gltf_with_scene_internal(
    meshes: &[MeshInput],
    nodes: &[NodeInput],
    options: &ExportOptions,
) -> ExportResult {
    let use_draco = options.use_draco.unwrap_or(false);
    let format = options.format.as_deref().unwrap_or("glb");

    // Build binary buffer
    let mut binary_data: Vec<u8> = Vec::new();
    let mut buffer_views: Vec<serde_json::Value> = Vec::new();
    let mut accessors: Vec<serde_json::Value> = Vec::new();
    let mut gltf_meshes: Vec<serde_json::Value> = Vec::new();

    // Add logging imports at top of function for debugging
    #[cfg(feature = "console_error_panic_hook")]
    {
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = console)]
            fn log(s: &str);
        }
        log(&format!("[GLTF_WRITER] Processing {} meshes", meshes.len()));
    }

    for (mesh_idx, mesh) in meshes.iter().enumerate() {
        let vertex_count = mesh.positions.len() / 3;
        let face_count = mesh.indices.len() / 3;

        #[cfg(feature = "console_error_panic_hook")]
        {
            use wasm_bindgen::prelude::*;
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = console)]
                fn log(s: &str);
            }
            log(&format!("[GLTF_WRITER] Mesh {}: positions.len()={}, indices.len()={}, vertex_count={}, face_count={}", 
                mesh_idx, mesh.positions.len(), mesh.indices.len(), vertex_count, face_count));
        }

        if use_draco {
            // Encode with Draco compression
            match encode_draco_mesh(mesh, options) {
                Ok(draco_result) => {
                    let bv_offset = binary_data.len();
                    binary_data.extend_from_slice(&draco_result.data);
                    // Pad to 4-byte alignment
                    while binary_data.len() % 4 != 0 {
                        binary_data.push(0);
                    }

                    let bv_idx = buffer_views.len();
                    buffer_views.push(serde_json::json!({
                        "buffer": 0,
                        "byteOffset": bv_offset,
                        "byteLength": draco_result.data.len()
                    }));

                    // Accessor for positions
                    let pos_accessor_idx = accessors.len();
                    accessors.push(serde_json::json!({
                        "count": vertex_count,
                        "componentType": 5126,
                        "type": "VEC3"
                    }));

                    // Accessor for indices
                    let idx_accessor_idx = accessors.len();
                    accessors.push(serde_json::json!({
                        "count": mesh.indices.len(),
                        "componentType": 5125,
                        "type": "SCALAR"
                    }));

                    let mut attributes = serde_json::json!({
                        "POSITION": pos_accessor_idx
                    });

                    // Use actual Draco attribute IDs
                    let mut draco_attributes = serde_json::json!({
                        "POSITION": draco_result.position_attr_id
                    });

                    // Add normals accessor if present
                    if mesh.normals.as_ref().map_or(false, |n| !n.is_empty()) && draco_result.normal_attr_id >= 0 {
                        let norm_accessor_idx = accessors.len();
                        accessors.push(serde_json::json!({
                            "count": vertex_count,
                            "componentType": 5126,
                            "type": "VEC3"
                        }));
                        attributes["NORMAL"] = serde_json::json!(norm_accessor_idx);
                        draco_attributes["NORMAL"] = serde_json::json!(draco_result.normal_attr_id);
                    }

                    // Add UV accessor if present
                    if mesh.uvs.as_ref().map_or(false, |u| !u.is_empty()) && draco_result.texcoord_attr_id >= 0 {
                        let uv_accessor_idx = accessors.len();
                        accessors.push(serde_json::json!({
                            "count": vertex_count,
                            "componentType": 5126,
                            "type": "VEC2"
                        }));
                        attributes["TEXCOORD_0"] = serde_json::json!(uv_accessor_idx);
                        draco_attributes["TEXCOORD_0"] = serde_json::json!(draco_result.texcoord_attr_id);
                    }

                    gltf_meshes.push(serde_json::json!({
                        "name": mesh.name,
                        "primitives": [{
                            "attributes": attributes,
                            "indices": idx_accessor_idx,
                            "extensions": {
                                "KHR_draco_mesh_compression": {
                                    "bufferView": bv_idx,
                                    "attributes": draco_attributes
                                }
                            }
                        }]
                    }));
                }
                Err(e) => {
                    return ExportResult {
                        success: false,
                        json_data: None,
                        binary_data: None,
                        error: Some(format!("Draco encoding failed: {}", e)),
                    };
                }
            }
        } else {
            // Standard glTF without Draco
            let mut attributes = HashMap::new();

            // Positions
            let pos_bv_offset = binary_data.len();
            for pos in mesh.positions.iter() {
                binary_data.extend_from_slice(&pos.to_le_bytes());
            }
            let pos_bv_idx = buffer_views.len();
            buffer_views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": pos_bv_offset,
                "byteLength": mesh.positions.len() * 4
            }));
            let pos_acc_idx = accessors.len();
            accessors.push(serde_json::json!({
                "bufferView": pos_bv_idx,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3"
            }));
            attributes.insert("POSITION", pos_acc_idx);

            // Normals
            if let Some(ref normals) = mesh.normals {
                if !normals.is_empty() {
                    let norm_bv_offset = binary_data.len();
                    for n in normals.iter() {
                        binary_data.extend_from_slice(&n.to_le_bytes());
                    }
                    let norm_bv_idx = buffer_views.len();
                    buffer_views.push(serde_json::json!({
                        "buffer": 0,
                        "byteOffset": norm_bv_offset,
                        "byteLength": normals.len() * 4
                    }));
                    let norm_acc_idx = accessors.len();
                    accessors.push(serde_json::json!({
                        "bufferView": norm_bv_idx,
                        "componentType": 5126,
                        "count": vertex_count,
                        "type": "VEC3"
                    }));
                    attributes.insert("NORMAL", norm_acc_idx);
                }
            }

            // UVs
            if let Some(ref uvs) = mesh.uvs {
                if !uvs.is_empty() {
                    let uv_bv_offset = binary_data.len();
                    for uv in uvs.iter() {
                        binary_data.extend_from_slice(&uv.to_le_bytes());
                    }
                    let uv_bv_idx = buffer_views.len();
                    buffer_views.push(serde_json::json!({
                        "buffer": 0,
                        "byteOffset": uv_bv_offset,
                        "byteLength": uvs.len() * 4
                    }));
                    let uv_acc_idx = accessors.len();
                    accessors.push(serde_json::json!({
                        "bufferView": uv_bv_idx,
                        "componentType": 5126,
                        "count": vertex_count,
                        "type": "VEC2"
                    }));
                    attributes.insert("TEXCOORD_0", uv_acc_idx);
                }
            }

            // Indices
            let idx_bv_offset = binary_data.len();
            for idx in mesh.indices.iter() {
                binary_data.extend_from_slice(&idx.to_le_bytes());
            }
            let idx_bv_idx = buffer_views.len();
            buffer_views.push(serde_json::json!({
                "buffer": 0,
                "byteOffset": idx_bv_offset,
                "byteLength": mesh.indices.len() * 4
            }));
            let idx_acc_idx = accessors.len();
            accessors.push(serde_json::json!({
                "bufferView": idx_bv_idx,
                "componentType": 5125,
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }));

            gltf_meshes.push(serde_json::json!({
                "name": mesh.name,
                "primitives": [{
                    "attributes": attributes,
                    "indices": idx_acc_idx
                }]
            }));
        }
    }

    // Build nodes
    let gltf_nodes: Vec<serde_json::Value> = nodes
        .iter()
        .map(|n| {
            let mut node = serde_json::json!({});
            if let Some(ref name) = n.name {
                node["name"] = serde_json::json!(name);
            }
            if let Some(mesh_idx) = n.mesh_index {
                node["mesh"] = serde_json::json!(mesh_idx);
            }
            if let Some(t) = n.translation {
                node["translation"] = serde_json::json!(t);
            }
            if let Some(r) = n.rotation {
                node["rotation"] = serde_json::json!(r);
            }
            if let Some(s) = n.scale {
                node["scale"] = serde_json::json!(s);
            }
            if !n.children.is_empty() {
                node["children"] = serde_json::json!(n.children);
            }
            node
        })
        .collect();

    // Root node indices for scene
    let root_nodes: Vec<usize> = (0..nodes.len()).collect();

    // Build glTF JSON
    let mut gltf_json = serde_json::json!({
        "asset": {
            "version": "2.0",
            "generator": "draco-io WASM"
        },
        "scene": 0,
        "scenes": [{
            "nodes": root_nodes
        }],
        "nodes": gltf_nodes,
        "meshes": gltf_meshes,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{
            "byteLength": binary_data.len()
        }]
    });

    if use_draco {
        gltf_json["extensionsUsed"] = serde_json::json!(["KHR_draco_mesh_compression"]);
        gltf_json["extensionsRequired"] = serde_json::json!(["KHR_draco_mesh_compression"]);
    }

    match format {
        "glb" => {
            // Build GLB
            let json_string = serde_json::to_string(&gltf_json).unwrap();
            let json_bytes = json_string.as_bytes();

            // Pad JSON to 4-byte alignment
            let json_padding = (4 - (json_bytes.len() % 4)) % 4;
            let json_chunk_length = json_bytes.len() + json_padding;

            // Pad binary to 4-byte alignment
            let bin_padding = (4 - (binary_data.len() % 4)) % 4;
            let bin_chunk_length = binary_data.len() + bin_padding;

            // Total file length
            let total_length = 12 + 8 + json_chunk_length + 8 + bin_chunk_length;

            let mut glb: Vec<u8> = Vec::with_capacity(total_length);

            // Header
            glb.extend_from_slice(&0x46546C67u32.to_le_bytes()); // "glTF"
            glb.extend_from_slice(&2u32.to_le_bytes()); // version
            glb.extend_from_slice(&(total_length as u32).to_le_bytes());

            // JSON chunk
            glb.extend_from_slice(&(json_chunk_length as u32).to_le_bytes());
            glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
            glb.extend_from_slice(json_bytes);
            for _ in 0..json_padding {
                glb.push(0x20); // space padding
            }

            // Binary chunk
            glb.extend_from_slice(&(bin_chunk_length as u32).to_le_bytes());
            glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
            glb.extend_from_slice(&binary_data);
            for _ in 0..bin_padding {
                glb.push(0);
            }

            ExportResult {
                success: true,
                json_data: None,
                binary_data: Some(glb),
                error: None,
            }
        }
        _ => {
            // Embedded glTF with base64 data URI
            let base64_data = base64_encode(&binary_data);
            gltf_json["buffers"][0]["uri"] = serde_json::json!(format!("data:application/octet-stream;base64,{}", base64_data));

            let json_string = serde_json::to_string_pretty(&gltf_json).unwrap();

            ExportResult {
                success: true,
                json_data: Some(json_string),
                binary_data: None,
                error: None,
            }
        }
    }
}

fn encode_draco_mesh(mesh: &MeshInput, options: &ExportOptions) -> Result<DracoEncodingResult, String> {
    use draco_core::encoder_buffer::EncoderBuffer;
    use draco_core::encoder_options::EncoderOptions;
    use draco_core::mesh::Mesh as DracoMesh;
    use draco_core::mesh_encoder::MeshEncoder;
    use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
    use draco_core::draco_types::DataType;
    use draco_core::geometry_indices::PointIndex;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }

    log(&format!("[DRACO] Starting encode_draco_mesh"));
    log(&format!("[DRACO] Mesh name: {:?}", mesh.name));
    log(&format!("[DRACO] Positions length: {}", mesh.positions.len()));
    log(&format!("[DRACO] Indices length: {}", mesh.indices.len()));
    log(&format!("[DRACO] Has normals: {}", mesh.normals.is_some()));
    log(&format!("[DRACO] Has UVs: {}", mesh.uvs.is_some()));

    // Validate input data
    if mesh.positions.len() % 3 != 0 {
        return Err(format!("Invalid positions array length: {}", mesh.positions.len()));
    }
    if mesh.indices.len() % 3 != 0 {
        return Err(format!("Invalid indices array length: {}", mesh.indices.len()));
    }
    
    let vertex_count = mesh.positions.len() / 3;
    let face_count = mesh.indices.len() / 3;
    
    log(&format!("[DRACO] Vertex count: {}", vertex_count));
    log(&format!("[DRACO] Face count: {}", face_count));
    
    if vertex_count == 0 {
        return Err("No vertices in mesh".to_string());
    }
    if face_count == 0 {
        return Err("No faces in mesh".to_string());
    }
    
    // Check that all indices are valid
    let max_index = mesh.indices.iter().max().copied().unwrap_or(0);
    let min_index = mesh.indices.iter().min().copied().unwrap_or(0);
    log(&format!("[DRACO] Index range: {} to {}", min_index, max_index));
    
    if max_index >= vertex_count as u32 {
        return Err(format!("Index out of bounds: {} >= {}", max_index, vertex_count));
    }

    log(&format!("[DRACO] Creating Draco mesh"));
    let mut draco_mesh = DracoMesh::new();
    draco_mesh.set_num_points(vertex_count);

    log(&format!("[DRACO] Adding position attribute"));
    // Add position attribute
    let mut pos_attr = PointAttribute::new();
    pos_attr.init(
        GeometryAttributeType::Position,
        3,
        DataType::Float32,
        false,
        vertex_count,
    );
    let pos_buffer = pos_attr.buffer_mut();
    for (i, chunk) in mesh.positions.chunks(3).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        pos_buffer.write(i * 12, &bytes);
    }
    draco_mesh.add_attribute(pos_attr);

    // Add normal attribute if present
    if let Some(ref normals) = mesh.normals {
        if !normals.is_empty() {
            log(&format!("[DRACO] Adding normal attribute ({} values)", normals.len()));
            let mut norm_attr = PointAttribute::new();
            norm_attr.init(
                GeometryAttributeType::Normal,
                3,
                DataType::Float32,
                false,
                vertex_count,
            );
            let norm_buffer = norm_attr.buffer_mut();
            for (i, chunk) in normals.chunks(3).enumerate() {
                let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
                norm_buffer.write(i * 12, &bytes);
            }
            draco_mesh.add_attribute(norm_attr);
        }
    }

    // Add UV attribute if present
    if let Some(ref uvs) = mesh.uvs {
        if !uvs.is_empty() {
            log(&format!("[DRACO] Adding UV attribute ({} values)", uvs.len()));
            let mut uv_attr = PointAttribute::new();
            uv_attr.init(
                GeometryAttributeType::TexCoord,
                2,
                DataType::Float32,
                false,
                vertex_count,
            );
            let uv_buffer = uv_attr.buffer_mut();
            for (i, chunk) in uvs.chunks(2).enumerate() {
                let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
                uv_buffer.write(i * 8, &bytes);
            }
            draco_mesh.add_attribute(uv_attr);
        }
    }

    log(&format!("[DRACO] Adding faces"));
    // Add faces
    for i in 0..face_count {
        let i0 = PointIndex(mesh.indices[i * 3]);
        let i1 = PointIndex(mesh.indices[i * 3 + 1]);
        let i2 = PointIndex(mesh.indices[i * 3 + 2]);
        draco_mesh.add_face([i0, i1, i2]);
    }

    log(&format!("[DRACO] Creating encoder"));
    // Encode
    let mut encoder = MeshEncoder::new();
    let mut encoder_buffer = EncoderBuffer::new();

    let mut enc_options = EncoderOptions::default();

    if let Some(pq) = options.position_quantization {
        let att_id = draco_mesh.named_attribute_id(GeometryAttributeType::Position);
        if att_id != -1 {
            enc_options.set_attribute_int(att_id, "quantization_bits", pq);
        }
    }
    if let Some(nq) = options.normal_quantization {
        let att_id = draco_mesh.named_attribute_id(GeometryAttributeType::Normal);
        if att_id != -1 {
            enc_options.set_attribute_int(att_id, "quantization_bits", nq);
        }
    }
    if let Some(tq) = options.texcoord_quantization {
        let att_id = draco_mesh.named_attribute_id(GeometryAttributeType::TexCoord);
        if att_id != -1 {
            enc_options.set_attribute_int(att_id, "quantization_bits", tq);
        }
    }

    log(&format!("[DRACO] Setting mesh on encoder"));
    encoder.set_mesh(draco_mesh.clone());
    
    // Get the actual attribute IDs assigned by Draco
    let pos_attr_id = draco_mesh.named_attribute_id(GeometryAttributeType::Position);
    let norm_attr_id = draco_mesh.named_attribute_id(GeometryAttributeType::Normal);
    let uv_attr_id = draco_mesh.named_attribute_id(GeometryAttributeType::TexCoord);
    
    log(&format!("[DRACO] Attribute IDs - Position: {}, Normal: {}, TexCoord: {}", pos_attr_id, norm_attr_id, uv_attr_id));

    // Diagnostic: print quantization bits for each attribute (fallback -1 means not set)
    let pos_q = enc_options.get_attribute_int(pos_attr_id, "quantization_bits", -1);
    log(&format!("[DRACO] Position quantization bits (att {}) = {}", pos_attr_id, pos_q));
    if norm_attr_id >= 0 {
        let norm_q = enc_options.get_attribute_int(norm_attr_id, "quantization_bits", -1);
        log(&format!("[DRACO] Normal quantization bits (att {}) = {}", norm_attr_id, norm_q));
    }
    if uv_attr_id >= 0 {
        let uv_q = enc_options.get_attribute_int(uv_attr_id, "quantization_bits", -1);
        log(&format!("[DRACO] TexCoord quantization bits (att {}) = {}", uv_attr_id, uv_q));
    }

    log(&format!("[DRACO] Starting encode..."));
    match encoder.encode(&enc_options, &mut encoder_buffer) {
        Ok(_) => {
            log(&format!("[DRACO] Encode successful, buffer size: {}", encoder_buffer.data().len()));

            // Verify produced buffer is decodable by the Rust decoder to catch encoding issues
            {
                use draco_core::decoder_buffer::DecoderBuffer;
                use draco_core::mesh_decoder::MeshDecoder;
                let mut decoder_buffer_check = DecoderBuffer::new(encoder_buffer.data());
                let mut mesh_check = DracoMesh::new();
                let mut decoder_check = MeshDecoder::new();
                match decoder_check.decode(&mut decoder_buffer_check, &mut mesh_check) {
                    Ok(_) => {
                        log(&format!("[DRACO] Post-encode check: decoded successfully (points={}, faces={})", mesh_check.num_points(), mesh_check.num_faces()));
                        return Ok(DracoEncodingResult {
                            data: encoder_buffer.data().to_vec(),
                            position_attr_id: pos_attr_id,
                            normal_attr_id: norm_attr_id,
                            texcoord_attr_id: uv_attr_id,
                        });
                    }
                    Err(e) => {
                        log(&format!("[DRACO] Post-encode check FAILED: {:?}", e));
                        // If decoding fails and quantization was enabled for position, retry without quantization.
                        let pos_q = enc_options.get_attribute_int(pos_attr_id, "quantization_bits", -1);
                        if pos_q > 0 {
                            log(&format!("[DRACO] Retrying encode WITHOUT quantization (pos was {})", pos_q));
                            // Clear quantization bit for position (and normals/uvs if set)
                            let mut retry_enc_options = enc_options.clone();
                            if pos_attr_id != -1 { retry_enc_options.set_attribute_int(pos_attr_id, "quantization_bits", -1); }
                            if norm_attr_id != -1 { retry_enc_options.set_attribute_int(norm_attr_id, "quantization_bits", -1); }
                            if uv_attr_id != -1 { retry_enc_options.set_attribute_int(uv_attr_id, "quantization_bits", -1); }

                            let mut retry_encoder = MeshEncoder::new();
                            retry_encoder.set_mesh(draco_mesh.clone());
                            let mut retry_buffer = EncoderBuffer::new();
                            match retry_encoder.encode(&retry_enc_options, &mut retry_buffer) {
                                Ok(_) => {
                                    log(&format!("[DRACO] Retry encode successful, buffer size: {}", retry_buffer.data().len()));
                                    // Double-check decode
                                    let mut decoder_buffer_check2 = DecoderBuffer::new(retry_buffer.data());
                                    let mut mesh_check2 = DracoMesh::new();
                                    let mut decoder_check2 = MeshDecoder::new();
                                    match decoder_check2.decode(&mut decoder_buffer_check2, &mut mesh_check2) {
                                        Ok(_) => {
                                            log(&format!("[DRACO] Retry post-encode check: decoded successfully (points={}, faces={})", mesh_check2.num_points(), mesh_check2.num_faces()));
                                            return Ok(DracoEncodingResult {
                                                data: retry_buffer.data().to_vec(),
                                                position_attr_id: pos_attr_id,
                                                normal_attr_id: norm_attr_id,
                                                texcoord_attr_id: uv_attr_id,
                                            });
                                        }
                                        Err(e2) => {
                                            log(&format!("[DRACO] Retry post-encode check FAILED: {:?}", e2));
                                            return Err(format!("Encode succeeded but post-encode decode failed: {:?}", e2));
                                        }
                                    }
                                }
                                Err(re) => {
                                    log(&format!("[DRACO] Retry encode FAILED: {:?}", re));
                                    return Err(format!("Retry encode failed: {:?}", re));
                                }
                            }
                        } else {
                            return Err(format!("Encode produced undecodable output: {:?}", e));
                        }
                    }
                }
            }
        }
        Err(e) => {
            log(&format!("[DRACO] Encode FAILED: {:?}", e));
            Err(format!("{:?}", e))
        }
    }
}

/// Result from Draco encoding including attribute IDs
struct DracoEncodingResult {
    data: Vec<u8>,
    position_attr_id: i32,
    normal_attr_id: i32,
    texcoord_attr_id: i32,
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut result = String::new();
    let mut i = 0;
    
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() { data[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as usize } else { 0 };
        
        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        
        if i + 1 < data.len() {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        
        if i + 2 < data.len() {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
        
        i += 3;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_gltf() {
        let mesh = MeshInput {
            name: Some("triangle".to_string()),
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            indices: vec![0, 1, 2],
            normals: None,
            uvs: None,
        };

        let result = create_gltf_internal(&[mesh], &ExportOptions::default());
        assert!(result.success);
        assert!(result.binary_data.is_some());
    }
}
