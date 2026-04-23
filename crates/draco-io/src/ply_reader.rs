//! PLY format reader for meshes and point clouds (ASCII only).
//!
//! Provides both a struct-based API (`PlyReader`) and convenience functions.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

use draco_core::draco_types::DataType;
use draco_core::geometry_attribute::{GeometryAttributeType, PointAttribute};
use draco_core::mesh::Mesh;

use crate::traits::{PointCloudReader, Reader};

#[derive(Debug)]
struct ParsedPlyColorData {
    num_components: u8,
    values: Vec<[u8; 4]>,
}

#[derive(Debug)]
struct ParsedPlyData {
    positions: ParsedPlyPositionData,
    faces: Vec<[u32; 3]>,
    normals: Option<Vec<[f32; 3]>>,
    colors: Option<ParsedPlyColorData>,
}

#[derive(Debug)]
enum ParsedPlyPositionData {
    Float32(Vec<[f32; 3]>),
    Int32(Vec<[i32; 3]>),
}

impl ParsedPlyPositionData {
    fn len(&self) -> usize {
        match self {
            ParsedPlyPositionData::Float32(values) => values.len(),
            ParsedPlyPositionData::Int32(values) => values.len(),
        }
    }

    fn to_f32_positions(&self) -> Vec<[f32; 3]> {
        match self {
            ParsedPlyPositionData::Float32(values) => values.clone(),
            ParsedPlyPositionData::Int32(values) => values
                .iter()
                .map(|value| [value[0] as f32, value[1] as f32, value[2] as f32])
                .collect(),
        }
    }
}

fn parse_ply_scalar_type(token: &str) -> Option<DataType> {
    match token {
        "char" | "int8" => Some(DataType::Int8),
        "uchar" | "uint8" => Some(DataType::Uint8),
        "short" | "int16" => Some(DataType::Int16),
        "ushort" | "uint16" => Some(DataType::Uint16),
        "int" | "int32" => Some(DataType::Int32),
        "uint" | "uint32" => Some(DataType::Uint32),
        "float" | "float32" => Some(DataType::Float32),
        "double" | "float64" => Some(DataType::Float64),
        _ => None,
    }
}

/// PLY format reader.
///
/// Reads vertex positions from ASCII PLY files.
#[derive(Debug)]
pub struct PlyReader {
    path: std::path::PathBuf,
}

impl PlyReader {
    /// Open a PLY file for reading.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ));
        }
        Ok(Self { path })
    }

    /// Read all positions from the PLY file.
    pub fn read_positions(&mut self) -> io::Result<Vec<[f32; 3]>> {
        Ok(read_ply_ascii(&self.path)?.positions.to_f32_positions())
    }

    /// Read a mesh with positions (and faces if present).
    pub fn read_mesh(&mut self) -> io::Result<Mesh> {
        let parsed = read_ply_ascii(&self.path)?;
        let mut mesh = Mesh::new();

        if parsed.positions.len() == 0 {
            return Ok(mesh);
        }

        mesh.set_num_points(parsed.positions.len());
        mesh.set_num_faces(parsed.faces.len());

        // Create position attribute
        match &parsed.positions {
            ParsedPlyPositionData::Float32(values) => {
                mesh.add_attribute(make_f32x3_attribute(
                    GeometryAttributeType::Position,
                    values,
                ));
            }
            ParsedPlyPositionData::Int32(values) => {
                mesh.add_attribute(make_i32x3_attribute(
                    GeometryAttributeType::Position,
                    values,
                ));
            }
        }

        if let Some(normals) = parsed.normals.as_ref() {
            mesh.add_attribute(make_f32x3_attribute(
                GeometryAttributeType::Normal,
                normals,
            ));
        }

        if let Some(colors) = parsed.colors.as_ref() {
            mesh.add_attribute(make_u8_attribute(
                GeometryAttributeType::Color,
                colors.num_components,
                true,
                &colors.values,
            ));
        }

        for (i, face) in parsed.faces.iter().enumerate() {
            mesh.set_face(
                draco_core::geometry_indices::FaceIndex(i as u32),
                [
                    draco_core::geometry_indices::PointIndex(face[0]),
                    draco_core::geometry_indices::PointIndex(face[1]),
                    draco_core::geometry_indices::PointIndex(face[2]),
                ],
            );
        }
        
        if mesh.num_faces() > 0 {
            // Match C++ Draco behavior: deduplicate point IDs in face-traversal order.
            // This ensures binary compatibility when encoding.
            mesh.deduplicate_point_ids();
        }

        Ok(mesh)
    }
}

impl Reader for PlyReader {
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        PlyReader::open(path)
    }

    fn read_meshes(&mut self) -> io::Result<Vec<Mesh>> {
        let m = self.read_mesh()?;
        Ok(vec![m])
    }
}

impl crate::traits::SceneReader for PlyReader {
    fn read_scene(&mut self) -> io::Result<crate::traits::Scene> {
        let meshes = self.read_meshes()?;
        let mut parts = Vec::with_capacity(meshes.len());
        let mut root = crate::traits::SceneNode::new(self.path.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()));
        for mesh in meshes {
            let part = crate::traits::SceneObject { name: None, mesh: mesh.clone(), transform: None };
            root.parts.push(part.clone());
            parts.push(part);
        }
        Ok(crate::traits::Scene { name: root.name.clone(), parts, root_nodes: vec![root] })
    }
}

impl PointCloudReader for PlyReader {
    fn read_points(&mut self) -> io::Result<Vec<[f32; 3]>> {
        self.read_positions()
    }
}

// ============================================================================
// Convenience Functions (for backward compatibility)
// ============================================================================

/// Parse point positions from an ASCII PLY file.
/// Returns a vec of [x, y, z] positions.
pub fn read_ply_positions<P: AsRef<Path>>(path: P) -> io::Result<Vec<[f32; 3]>> {
    Ok(read_ply_ascii(path)?.positions.to_f32_positions())
}

fn make_f32x3_attribute(
    attribute_type: GeometryAttributeType,
    values: &[[f32; 3]],
) -> PointAttribute {
    let mut attribute = PointAttribute::new();
    attribute.init(attribute_type, 3, DataType::Float32, false, values.len());

    let buffer = attribute.buffer_mut();
    for (i, value) in values.iter().enumerate() {
        let bytes: Vec<u8> = value.iter().flat_map(|component| component.to_le_bytes()).collect();
        buffer.write(i * 12, &bytes);
    }

    attribute
}

fn make_i32x3_attribute(
    attribute_type: GeometryAttributeType,
    values: &[[i32; 3]],
) -> PointAttribute {
    let mut attribute = PointAttribute::new();
    attribute.init(attribute_type, 3, DataType::Int32, false, values.len());

    let buffer = attribute.buffer_mut();
    for (i, value) in values.iter().enumerate() {
        let bytes: Vec<u8> = value.iter().flat_map(|component| component.to_le_bytes()).collect();
        buffer.write(i * 12, &bytes);
    }

    attribute
}

fn make_u8_attribute(
    attribute_type: GeometryAttributeType,
    num_components: u8,
    normalized: bool,
    values: &[[u8; 4]],
) -> PointAttribute {
    let mut attribute = PointAttribute::new();
    attribute.init(
        attribute_type,
        num_components,
        DataType::Uint8,
        normalized,
        values.len(),
    );

    let buffer = attribute.buffer_mut();
    for (i, value) in values.iter().enumerate() {
        let end = num_components as usize;
        buffer.write(i * end, &value[..end]);
    }

    attribute
}

fn read_ply_ascii<P: AsRef<Path>>(path: P) -> io::Result<ParsedPlyData> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Read header.
    let mut in_header = true;
    let mut vertex_count = 0usize;
    let mut face_count = 0usize;
    let mut position_data_type = DataType::Float32;
    let mut prop_x_idx = None;
    let mut prop_y_idx = None;
    let mut prop_z_idx = None;
    let mut prop_nx_idx = None;
    let mut prop_ny_idx = None;
    let mut prop_nz_idx = None;
    let mut prop_nx_type = None;
    let mut prop_ny_type = None;
    let mut prop_nz_type = None;
    let mut prop_r_idx = None;
    let mut prop_g_idx = None;
    let mut prop_b_idx = None;
    let mut prop_a_idx = None;
    let mut prop_r_type = None;
    let mut prop_g_type = None;
    let mut prop_b_type = None;
    let mut prop_a_type = None;
    let mut prop_idx = 0;
    let mut current_element: Option<String> = None;
    let mut is_ascii = false;

    if let Some(line) = lines.next() {
        let line = line?;
        if line.trim() != "ply" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Missing PLY header"));
        }
    } else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Empty PLY file"));
    }
    
    for line in lines.by_ref() {
        let line = line?;
        let trimmed = line.trim();
        
        if trimmed == "end_header" {
            in_header = false;
            break;
        }
        
        if trimmed.starts_with("format ascii") {
            is_ascii = true;
        } else if trimmed.starts_with("element ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                current_element = Some(parts[1].to_string());
                match parts[1] {
                    "vertex" => {
                        vertex_count = parts[2].parse().map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidData, "Invalid vertex count")
                        })?;
                        prop_idx = 0;
                    }
                    "face" => {
                        face_count = parts[2].parse().map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidData, "Invalid face count")
                        })?;
                    }
                    _ => {
                        prop_idx = 0;
                    }
                }
            }
        } else if current_element.as_deref() == Some("vertex") && trimmed.starts_with("property ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let prop_type = parts.get(1).and_then(|token| parse_ply_scalar_type(token));
                let prop_name = if parts[1] == "list" {
                    parts.get(4).copied()
                } else {
                    parts.get(2).copied()
                };
                if let Some(prop_name) = prop_name {
                    if matches!(prop_name, "x" | "y" | "z") {
                        position_data_type = match parts.get(1).copied() {
                            Some("float") | Some("float32") | Some("double") | Some("float64") => DataType::Float32,
                            Some("int") | Some("int32") => DataType::Int32,
                            _ => DataType::Float32,
                        };
                    }
                    match prop_name {
                        "x" => prop_x_idx = Some(prop_idx),
                        "y" => prop_y_idx = Some(prop_idx),
                        "z" => prop_z_idx = Some(prop_idx),
                        "nx" => {
                            prop_nx_idx = Some(prop_idx);
                            prop_nx_type = prop_type;
                        }
                        "ny" => {
                            prop_ny_idx = Some(prop_idx);
                            prop_ny_type = prop_type;
                        }
                        "nz" => {
                            prop_nz_idx = Some(prop_idx);
                            prop_nz_type = prop_type;
                        }
                        "red" => {
                            prop_r_idx = Some(prop_idx);
                            prop_r_type = prop_type;
                        }
                        "green" => {
                            prop_g_idx = Some(prop_idx);
                            prop_g_type = prop_type;
                        }
                        "blue" => {
                            prop_b_idx = Some(prop_idx);
                            prop_b_type = prop_type;
                        }
                        "alpha" => {
                            prop_a_idx = Some(prop_idx);
                            prop_a_type = prop_type;
                        }
                        _ => {}
                    }
                }
                prop_idx += 1;
            }
        }
    }
    
    if in_header {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No end_header found"));
    }

    if !is_ascii {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Only ASCII PLY files are currently supported",
        ));
    }
    
    let x_idx = prop_x_idx.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No x property"))?;
    let y_idx = prop_y_idx.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No y property"))?;
    let z_idx = prop_z_idx.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No z property"))?;
    let has_normals = prop_nx_idx.is_some()
        && prop_ny_idx.is_some()
        && prop_nz_idx.is_some()
        && prop_nx_type == Some(DataType::Float32)
        && prop_ny_type == Some(DataType::Float32)
        && prop_nz_type == Some(DataType::Float32);
    let color_indices: Vec<usize> = [prop_r_idx, prop_g_idx, prop_b_idx, prop_a_idx]
        .into_iter()
        .flatten()
        .collect();
    if !color_indices.is_empty() {
        for color_type in [prop_r_type, prop_g_type, prop_b_type, prop_a_type]
            .into_iter()
            .flatten()
        {
            if color_type != DataType::Uint8 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Color properties must be uint8",
                ));
            }
        }
    }
    
    // Read vertex data.
    let mut float_positions = matches!(position_data_type, DataType::Float32)
        .then(|| Vec::with_capacity(vertex_count));
    let mut int_positions = matches!(position_data_type, DataType::Int32)
        .then(|| Vec::with_capacity(vertex_count));
    let mut normals = has_normals.then(|| Vec::with_capacity(vertex_count));
    let mut colors = (!color_indices.is_empty()).then(|| ParsedPlyColorData {
        num_components: color_indices.len() as u8,
        values: Vec::with_capacity(vertex_count),
    });
    for _ in 0..vertex_count {
        let line = match lines.next() {
            Some(line) => line?,
            None => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let mut max_required_idx = x_idx.max(y_idx).max(z_idx);
        if let Some(nx_idx) = prop_nx_idx {
            max_required_idx = max_required_idx.max(nx_idx);
        }
        if let Some(ny_idx) = prop_ny_idx {
            max_required_idx = max_required_idx.max(ny_idx);
        }
        if let Some(nz_idx) = prop_nz_idx {
            max_required_idx = max_required_idx.max(nz_idx);
        }
        for color_idx in &color_indices {
            max_required_idx = max_required_idx.max(*color_idx);
        }
        if parts.len() <= max_required_idx {
            continue;
        }
        
        match position_data_type {
            DataType::Int32 => {
                int_positions.as_mut().unwrap().push([
                    parts[x_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad x value"))?,
                    parts[y_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad y value"))?,
                    parts[z_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad z value"))?,
                ]);
            }
            _ => {
                float_positions.as_mut().unwrap().push([
                    parts[x_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad x value"))?,
                    parts[y_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad y value"))?,
                    parts[z_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad z value"))?,
                ]);
            }
        }

        if let Some(normals) = normals.as_mut() {
            normals.push([
                parts[prop_nx_idx.unwrap()].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad nx value"))?,
                parts[prop_ny_idx.unwrap()].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad ny value"))?,
                parts[prop_nz_idx.unwrap()].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad nz value"))?,
            ]);
        }

        if let Some(colors) = colors.as_mut() {
            let mut color = [0u8; 4];
            for (component, color_idx) in color_indices.iter().enumerate() {
                color[component] = parts[*color_idx].parse().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Bad color component value")
                })?;
            }
            colors.values.push(color);
        }
    }

    let mut faces = Vec::with_capacity(face_count);
    for _ in 0..face_count {
        let line = match lines.next() {
            Some(line) => line?,
            None => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let indices: Vec<u32> = trimmed
            .split_whitespace()
            .map(|part| {
                part.parse::<u32>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Bad face index value")
                })
            })
            .collect::<io::Result<Vec<u32>>>()?;

        if indices.is_empty() {
            continue;
        }

        let polygon_size = indices[0] as usize;
        if polygon_size < 3 || indices.len() < polygon_size + 1 {
            continue;
        }

        for j in 1..polygon_size - 1 {
            faces.push([indices[1], indices[j + 1], indices[j + 2]]);
        }
    }

    Ok(ParsedPlyData {
        positions: match position_data_type {
            DataType::Int32 => ParsedPlyPositionData::Int32(int_positions.unwrap_or_default()),
            _ => ParsedPlyPositionData::Float32(float_positions.unwrap_or_default()),
        },
        faces,
        normals,
        colors,
    })
}

/// Write point positions to an ASCII PLY file.
pub fn write_ply_positions<P: AsRef<Path>>(path: P, points: &[[f32; 3]]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    
    writeln!(file, "ply")?;
    writeln!(file, "format ascii 1.0")?;
    writeln!(file, "element vertex {}", points.len())?;
    writeln!(file, "property float x")?;
    writeln!(file, "property float y")?;
    writeln!(file, "property float z")?;
    writeln!(file, "end_header")?;
    
    for p in points {
        writeln!(file, "{:.6} {:.6} {:.6}", p[0], p[1], p[2])?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draco_core::geometry_attribute::GeometryAttributeType;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_read_write_ply() {
        let expected = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, -1.0, -1.0],
        ];
        
        let file = NamedTempFile::new().unwrap();
        write_ply_positions(file.path(), &expected).unwrap();
        
        let positions = read_ply_positions(file.path()).unwrap();
        assert_eq!(positions.len(), expected.len());
        
        for (i, (a, b)) in positions.iter().zip(expected.iter()).enumerate() {
            let diff = (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs();
            assert!(diff < 1e-5, "Position mismatch at index {i}: {a:?} vs {b:?}");
        }
    }

    #[test]
    fn test_read_mesh_parses_and_triangulates_faces() {
        let file = NamedTempFile::new().unwrap();
        let ply = r#"ply
format ascii 1.0
element vertex 4
property float x
property float y
property float z
element face 2
property list uchar int vertex_indices
end_header
0 0 0
1 0 0
1 1 0
0 1 0
3 0 1 2
4 0 1 2 3
"#;

        std::fs::write(file.path(), ply).unwrap();

        let mut reader = PlyReader::open(file.path()).unwrap();
        let mesh = reader.read_mesh().unwrap();

        assert_eq!(mesh.num_points(), 4);
        assert_eq!(mesh.num_faces(), 3);
        assert_eq!(mesh.face(draco_core::geometry_indices::FaceIndex(0)), [0u32.into(), 1u32.into(), 2u32.into()]);
        assert_eq!(mesh.face(draco_core::geometry_indices::FaceIndex(1)), [0u32.into(), 1u32.into(), 2u32.into()]);
        assert_eq!(mesh.face(draco_core::geometry_indices::FaceIndex(2)), [0u32.into(), 2u32.into(), 3u32.into()]);
    }

    #[test]
    fn test_read_mesh_parses_normals_and_colors() {
        let file = NamedTempFile::new().unwrap();
        let ply = r#"ply
format ascii 1.0
element vertex 2
property float x
property float y
property float z
property float nx
property float ny
property float nz
property uchar red
property uchar green
property uchar blue
property uchar alpha
end_header
0 0 0 0 0 1 10 20 30 40
1 0 0 0 1 0 50 60 70 80
"#;

        std::fs::write(file.path(), ply).unwrap();

        let mut reader = PlyReader::open(file.path()).unwrap();
        let mesh = reader.read_mesh().unwrap();

        assert_eq!(mesh.num_points(), 2);
        assert_eq!(mesh.num_faces(), 0);
        assert_eq!(mesh.num_attributes(), 3);

        let normal_att = mesh.named_attribute(GeometryAttributeType::Normal).unwrap();
        assert_eq!(normal_att.data_type(), DataType::Float32);
        assert_eq!(normal_att.num_components(), 3);
        assert!(!normal_att.normalized());

        let normal_data = normal_att.buffer().data();
        let first_normal = [
            f32::from_le_bytes(normal_data[0..4].try_into().unwrap()),
            f32::from_le_bytes(normal_data[4..8].try_into().unwrap()),
            f32::from_le_bytes(normal_data[8..12].try_into().unwrap()),
        ];
        assert_eq!(first_normal, [0.0, 0.0, 1.0]);

        let color_att = mesh.named_attribute(GeometryAttributeType::Color).unwrap();
        assert_eq!(color_att.data_type(), DataType::Uint8);
        assert_eq!(color_att.num_components(), 4);
        assert!(color_att.normalized());
        assert_eq!(color_att.buffer().data(), &[10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn test_read_mesh_preserves_int32_positions() {
        let file = NamedTempFile::new().unwrap();
        let ply = r#"ply
format ascii 1.0
element vertex 2
property int x
property int y
property int z
end_header
1 2 3
4 5 6
"#;

        std::fs::write(file.path(), ply).unwrap();

        let mut reader = PlyReader::open(file.path()).unwrap();
        let mesh = reader.read_mesh().unwrap();

        let position_att = mesh.named_attribute(GeometryAttributeType::Position).unwrap();
        assert_eq!(position_att.data_type(), DataType::Int32);
        assert_eq!(position_att.num_components(), 3);
        assert!(!position_att.normalized());

        let position_data = position_att.buffer().data();
        let first_position = [
            i32::from_le_bytes(position_data[0..4].try_into().unwrap()),
            i32::from_le_bytes(position_data[4..8].try_into().unwrap()),
            i32::from_le_bytes(position_data[8..12].try_into().unwrap()),
        ];
        assert_eq!(first_position, [1, 2, 3]);
    }

    #[test]
    fn test_read_mesh_ignores_non_float_normals() {
        let file = NamedTempFile::new().unwrap();
        let ply = r#"ply
format ascii 1.0
element vertex 1
property float x
property float y
property float z
property int nx
property int ny
property int nz
end_header
0 0 0 0 0 1
"#;

        std::fs::write(file.path(), ply).unwrap();

        let mut reader = PlyReader::open(file.path()).unwrap();
        let mesh = reader.read_mesh().unwrap();

        assert_eq!(mesh.named_attribute_id(GeometryAttributeType::Normal), -1);
    }

    #[test]
    fn test_read_mesh_rejects_non_uint8_colors() {
        let file = NamedTempFile::new().unwrap();
        let ply = r#"ply
format ascii 1.0
element vertex 1
property float x
property float y
property float z
property int red
property int green
property int blue
end_header
0 0 0 1 2 3
"#;

        std::fs::write(file.path(), ply).unwrap();

        let mut reader = PlyReader::open(file.path()).unwrap();
        let error = reader.read_mesh().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("Color properties must be uint8"));
    }
}

