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
        read_ply_positions(&self.path)
    }

    /// Read a mesh with positions (and faces if present).
    pub fn read_mesh(&mut self) -> io::Result<Mesh> {
        let positions = self.read_positions()?;
        let mut mesh = Mesh::new();

        if positions.is_empty() {
            return Ok(mesh);
        }

        // Create position attribute
        let mut pos_att = PointAttribute::new();
        pos_att.init(
            GeometryAttributeType::Position,
            3,
            DataType::Float32,
            false,
            positions.len(),
        );

        let buffer = pos_att.buffer_mut();
        for (i, pos) in positions.iter().enumerate() {
            let bytes: Vec<u8> = pos.iter().flat_map(|v| v.to_le_bytes()).collect();
            buffer.write(i * 12, &bytes);
        }

        mesh.add_attribute(pos_att);

        // TODO: Parse faces if needed (basic PLY reader doesn't parse faces yet)

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
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Read header.
    let mut in_header = true;
    let mut vertex_count = 0usize;
    let mut prop_x_idx = None;
    let mut prop_y_idx = None;
    let mut prop_z_idx = None;
    let mut prop_idx = 0;
    
    for line in lines.by_ref() {
        let line = line?;
        let trimmed = line.trim();
        
        if trimmed == "end_header" {
            in_header = false;
            break;
        }
        
        if trimmed.starts_with("element vertex ") {
            vertex_count = trimmed
                .strip_prefix("element vertex ")
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid vertex count"))?;
        } else if trimmed.starts_with("property float x") || trimmed.starts_with("property double x") {
            prop_x_idx = Some(prop_idx);
            prop_idx += 1;
        } else if trimmed.starts_with("property float y") || trimmed.starts_with("property double y") {
            prop_y_idx = Some(prop_idx);
            prop_idx += 1;
        } else if trimmed.starts_with("property float z") || trimmed.starts_with("property double z") {
            prop_z_idx = Some(prop_idx);
            prop_idx += 1;
        } else if trimmed.starts_with("property ") {
            prop_idx += 1;
        }
    }
    
    if in_header {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No end_header found"));
    }
    
    let x_idx = prop_x_idx.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No x property"))?;
    let y_idx = prop_y_idx.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No y property"))?;
    let z_idx = prop_z_idx.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "No z property"))?;
    
    // Read vertex data.
    let mut positions = Vec::with_capacity(vertex_count);
    for line in lines {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() <= x_idx.max(y_idx).max(z_idx) {
            continue;
        }
        
        let x: f32 = parts[x_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad x value"))?;
        let y: f32 = parts[y_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad y value"))?;
        let z: f32 = parts[z_idx].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Bad z value"))?;
        
        positions.push([x, y, z]);
        
        if positions.len() >= vertex_count {
            break;
        }
    }
    
    Ok(positions)
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
}

