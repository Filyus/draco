//! PLY Reader WASM module.
//!
//! Provides PLY file parsing functionality for web applications.
//! Supports ASCII PLY format.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// Mesh data structure for JavaScript interop.
#[derive(Serialize, Deserialize)]
pub struct MeshData {
    /// Vertex positions as flat array [x0, y0, z0, x1, y1, z1, ...]
    pub positions: Vec<f32>,
    /// Face indices as flat array (triangles)
    pub indices: Vec<u32>,
    /// Vertex normals (if present)
    pub normals: Vec<f32>,
    /// Vertex colors as flat array [r0, g0, b0, a0, ...] (0-255)
    pub colors: Vec<u8>,
}

/// Parse result containing meshes and any warnings/errors.
#[derive(Serialize, Deserialize)]
pub struct ParseResult {
    pub success: bool,
    pub meshes: Vec<MeshData>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    /// PLY header information
    pub header: Option<PlyHeader>,
}

/// PLY header information.
#[derive(Serialize, Deserialize, Clone)]
pub struct PlyHeader {
    pub format: String,
    pub vertex_count: usize,
    pub face_count: usize,
    pub properties: Vec<String>,
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
    "PLY Reader".to_string()
}

/// Get supported file extensions.
#[wasm_bindgen]
pub fn supported_extensions() -> Vec<String> {
    vec!["ply".to_string()]
}

/// Parse PLY file content from a string (ASCII PLY).
#[wasm_bindgen]
pub fn parse_ply(content: &str) -> JsValue {
    let result = parse_ply_internal(content);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Parse PLY file content from bytes.
#[wasm_bindgen]
pub fn parse_ply_bytes(data: &[u8]) -> JsValue {
    // Try to detect if it's ASCII or binary
    match std::str::from_utf8(data) {
        Ok(content) => parse_ply(content),
        Err(_) => {
            // Could be binary PLY - try to parse header and detect format
            let result = parse_binary_ply(data);
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
    }
}

#[derive(Debug, Clone)]
struct PlyProperty {
    name: String,
    data_type: String,
    is_list: bool,
    list_count_type: Option<String>,
    list_elem_type: Option<String>,
}

fn parse_ply_internal(content: &str) -> ParseResult {
    let mut lines = content.lines().peekable();
    let mut warnings: Vec<String> = Vec::new();

    // Parse header
    let first_line = lines.next().unwrap_or("").trim();
    if first_line != "ply" {
        return ParseResult {
            success: false,
            meshes: vec![],
            error: Some("Invalid PLY file: missing 'ply' header".to_string()),
            warnings: vec![],
            header: None,
        };
    }

    let mut format = String::new();
    let mut vertex_count = 0usize;
    let mut face_count = 0usize;
    let mut vertex_properties: Vec<PlyProperty> = Vec::new();
    let mut face_properties: Vec<PlyProperty> = Vec::new();
    let mut current_element = String::new();
    let mut property_names: Vec<String> = Vec::new();

    // Parse header lines
    loop {
        let line = match lines.next() {
            Some(l) => l.trim(),
            None => break,
        };

        if line == "end_header" {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "format" => {
                if parts.len() >= 2 {
                    format = parts[1].to_string();
                }
            }
            "element" => {
                if parts.len() >= 3 {
                    current_element = parts[1].to_string();
                    let count: usize = parts[2].parse().unwrap_or(0);
                    if current_element == "vertex" {
                        vertex_count = count;
                    } else if current_element == "face" {
                        face_count = count;
                    }
                }
            }
            "property" => {
                if parts.len() >= 3 {
                    let prop = if parts[1] == "list" && parts.len() >= 5 {
                        PlyProperty {
                            name: parts[4].to_string(),
                            data_type: parts[3].to_string(),
                            is_list: true,
                            list_count_type: Some(parts[2].to_string()),
                            list_elem_type: Some(parts[3].to_string()),
                        }
                    } else {
                        PlyProperty {
                            name: parts[2].to_string(),
                            data_type: parts[1].to_string(),
                            is_list: false,
                            list_count_type: None,
                            list_elem_type: None,
                        }
                    };

                    property_names.push(prop.name.clone());

                    if current_element == "vertex" {
                        vertex_properties.push(prop);
                    } else if current_element == "face" {
                        face_properties.push(prop);
                    }
                }
            }
            _ => {}
        }
    }

    if format != "ascii" {
        return ParseResult {
            success: false,
            meshes: vec![],
            error: Some(format!("Binary PLY format '{}' not supported via string parsing. Use parse_ply_bytes for binary files.", format)),
            warnings: vec![],
            header: Some(PlyHeader {
                format,
                vertex_count,
                face_count,
                properties: property_names,
            }),
        };
    }

    // Find property indices
    let x_idx = vertex_properties.iter().position(|p| p.name == "x");
    let y_idx = vertex_properties.iter().position(|p| p.name == "y");
    let z_idx = vertex_properties.iter().position(|p| p.name == "z");
    let nx_idx = vertex_properties.iter().position(|p| p.name == "nx");
    let ny_idx = vertex_properties.iter().position(|p| p.name == "ny");
    let nz_idx = vertex_properties.iter().position(|p| p.name == "nz");
    let r_idx = vertex_properties.iter().position(|p| p.name == "red" || p.name == "r");
    let g_idx = vertex_properties.iter().position(|p| p.name == "green" || p.name == "g");
    let b_idx = vertex_properties.iter().position(|p| p.name == "blue" || p.name == "b");
    let a_idx = vertex_properties.iter().position(|p| p.name == "alpha" || p.name == "a");

    let has_positions = x_idx.is_some() && y_idx.is_some() && z_idx.is_some();
    let has_normals = nx_idx.is_some() && ny_idx.is_some() && nz_idx.is_some();
    let has_colors = r_idx.is_some() && g_idx.is_some() && b_idx.is_some();

    if !has_positions {
        return ParseResult {
            success: false,
            meshes: vec![],
            error: Some("PLY file missing position properties (x, y, z)".to_string()),
            warnings: vec![],
            header: Some(PlyHeader {
                format,
                vertex_count,
                face_count,
                properties: property_names,
            }),
        };
    }

    let mut positions: Vec<f32> = Vec::with_capacity(vertex_count * 3);
    let mut normals: Vec<f32> = Vec::new();
    let mut colors: Vec<u8> = Vec::new();

    if has_normals {
        normals.reserve(vertex_count * 3);
    }
    if has_colors {
        colors.reserve(vertex_count * 4);
    }

    // Parse vertices
    for i in 0..vertex_count {
        let line = match lines.next() {
            Some(l) => l.trim(),
            None => {
                warnings.push(format!("Unexpected end of file at vertex {}", i));
                break;
            }
        };

        let values: Vec<f32> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if let (Some(xi), Some(yi), Some(zi)) = (x_idx, y_idx, z_idx) {
            positions.push(*values.get(xi).unwrap_or(&0.0));
            positions.push(*values.get(yi).unwrap_or(&0.0));
            positions.push(*values.get(zi).unwrap_or(&0.0));
        }

        if has_normals {
            if let (Some(nxi), Some(nyi), Some(nzi)) = (nx_idx, ny_idx, nz_idx) {
                normals.push(*values.get(nxi).unwrap_or(&0.0));
                normals.push(*values.get(nyi).unwrap_or(&0.0));
                normals.push(*values.get(nzi).unwrap_or(&0.0));
            }
        }

        if has_colors {
            if let (Some(ri), Some(gi), Some(bi)) = (r_idx, g_idx, b_idx) {
                colors.push((*values.get(ri).unwrap_or(&255.0)) as u8);
                colors.push((*values.get(gi).unwrap_or(&255.0)) as u8);
                colors.push((*values.get(bi).unwrap_or(&255.0)) as u8);
                if let Some(ai) = a_idx {
                    colors.push((*values.get(ai).unwrap_or(&255.0)) as u8);
                } else {
                    colors.push(255);
                }
            }
        }
    }

    // Parse faces
    let mut indices: Vec<u32> = Vec::with_capacity(face_count * 3);

    for i in 0..face_count {
        let line = match lines.next() {
            Some(l) => l.trim(),
            None => {
                warnings.push(format!("Unexpected end of file at face {}", i));
                break;
            }
        };

        let values: Vec<u32> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if values.is_empty() {
            continue;
        }

        let count = values[0] as usize;
        if values.len() < count + 1 {
            warnings.push(format!("Face {} has incomplete indices", i));
            continue;
        }

        // Triangulate (fan triangulation for polygons)
        for j in 1..count - 1 {
            indices.push(values[1]);
            indices.push(values[j + 1]);
            indices.push(values[j + 2]);
        }
    }

    let mesh = MeshData {
        positions,
        indices,
        normals,
        colors,
    };

    ParseResult {
        success: true,
        meshes: vec![mesh],
        error: None,
        warnings,
        header: Some(PlyHeader {
            format,
            vertex_count,
            face_count,
            properties: property_names,
        }),
    }
}

fn parse_binary_ply(_data: &[u8]) -> ParseResult {
    // Basic binary PLY parsing would go here
    // For now, return an error indicating binary is not yet fully supported
    ParseResult {
        success: false,
        meshes: vec![],
        error: Some("Binary PLY format parsing is not yet implemented. Please use ASCII PLY files.".to_string()),
        warnings: vec![],
        header: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ply() {
        let ply = r#"ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
element face 1
property list uchar int vertex_indices
end_header
0 0 0
1 0 0
0.5 1 0
3 0 1 2
"#;

        let result = parse_ply_internal(ply);
        assert!(result.success);
        assert_eq!(result.meshes.len(), 1);
        assert_eq!(result.meshes[0].positions.len(), 9); // 3 vertices * 3 components
        assert_eq!(result.meshes[0].indices.len(), 3); // 1 triangle
    }
}
