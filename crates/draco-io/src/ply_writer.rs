//! PLY format writer for meshes and point clouds.
//!
//! Supports writing:
//! - ASCII PLY format
//! - Binary little-endian PLY format
//! - Vertex positions
//! - Vertex normals (if present)
//! - Vertex colors (if present)
//! - Triangle faces (for meshes)
//!
//! # Example
//!
//! ```ignore
//! use draco_io::PlyWriter;
//! use draco_core::mesh::Mesh;
//!
//! let mesh: Mesh = /* ... */;
//! let mut writer = PlyWriter::new();
//! writer.add_mesh(&mesh);
//! writer.write("output.ply")?;
//!
//! // Or write point cloud
//! let mut writer = PlyWriter::new();
//! writer.add_points(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]);
//! writer.write("points.ply")?;
//! ```

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use draco_core::geometry_attribute::GeometryAttributeType;
use draco_core::geometry_indices::FaceIndex;
use draco_core::mesh::Mesh;

use crate::traits::{PointCloudWriter, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PlyWriteFormat {
    #[default]
    Ascii,
    BinaryLittleEndian,
}

/// PLY format writer.
///
/// This struct provides a builder-style API for writing PLY files.
/// Meshes or points are added, then written with `write()`.
///
/// # Example
///
/// ```ignore
/// use draco_io::PlyWriter;
///
/// let mut writer = PlyWriter::new();
/// writer.add_mesh(&mesh);
/// writer.write("cube.ply")?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct PlyWriter {
    /// Output format
    format: PlyWriteFormat,
    /// Collected vertex positions
    positions: Vec<[f32; 3]>,
    /// Collected vertex normals
    normals: Vec<[f32; 3]>,
    /// Collected vertex colors (RGBA 0-255)
    colors: Vec<[u8; 4]>,
    /// Collected faces (0-based indices)
    faces: Vec<[u32; 3]>,
}

impl PlyWriter {
    /// Create a new PLY writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the writer to emit binary little-endian PLY.
    pub fn with_binary_little_endian(mut self) -> Self {
        self.format = PlyWriteFormat::BinaryLittleEndian;
        self
    }

    /// Enable or disable binary little-endian output.
    pub fn set_binary_little_endian(&mut self, enabled: bool) -> &mut Self {
        self.format = if enabled {
            PlyWriteFormat::BinaryLittleEndian
        } else {
            PlyWriteFormat::Ascii
        };
        self
    }

    /// Returns true when the writer is configured for binary little-endian output.
    pub fn is_binary_little_endian(&self) -> bool {
        self.format == PlyWriteFormat::BinaryLittleEndian
    }

    /// Add raw point positions (for point cloud output).
    pub fn add_points(&mut self, points: &[[f32; 3]]) {
        self.positions.extend_from_slice(points);
    }

    /// Add a single point.
    pub fn add_point(&mut self, point: [f32; 3]) {
        self.positions.push(point);
    }

    /// Add points with colors.
    pub fn add_points_with_colors(&mut self, points: &[[f32; 3]], colors: &[[u8; 4]]) {
        // Pad colors if needed
        while self.colors.len() < self.positions.len() {
            self.colors.push([255, 255, 255, 255]);
        }
        self.positions.extend_from_slice(points);
        self.colors.extend_from_slice(colors);
    }

    /// Get the number of vertices added.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Get the number of faces added.
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Check if the writer has normals.
    pub fn has_normals(&self) -> bool {
        !self.normals.is_empty()
    }

    /// Check if the writer has colors.
    pub fn has_colors(&self) -> bool {
        !self.colors.is_empty()
    }

    /// Write the PLY file to the given path.
    pub fn write<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        self.write_to(&mut writer)
    }

    /// Write the PLY data to a writer.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let has_normals = self.normals.len() == self.positions.len();
        let has_colors = self.colors.len() == self.positions.len();

        self.write_header(writer, has_normals, has_colors)?;

        match self.format {
            PlyWriteFormat::Ascii => self.write_ascii_body(writer, has_normals, has_colors),
            PlyWriteFormat::BinaryLittleEndian => {
                self.write_binary_little_endian_body(writer, has_normals, has_colors)
            }
        }
    }

    fn write_header<W: Write>(
        &self,
        writer: &mut W,
        has_normals: bool,
        has_colors: bool,
    ) -> io::Result<()> {
        writeln!(writer, "ply")?;
        match self.format {
            PlyWriteFormat::Ascii => writeln!(writer, "format ascii 1.0")?,
            PlyWriteFormat::BinaryLittleEndian => {
                writeln!(writer, "format binary_little_endian 1.0")?
            }
        }
        writeln!(writer, "comment Generated by draco-io")?;
        writeln!(writer, "element vertex {}", self.positions.len())?;
        writeln!(writer, "property float x")?;
        writeln!(writer, "property float y")?;
        writeln!(writer, "property float z")?;

        if has_normals {
            writeln!(writer, "property float nx")?;
            writeln!(writer, "property float ny")?;
            writeln!(writer, "property float nz")?;
        }

        if has_colors {
            writeln!(writer, "property uchar red")?;
            writeln!(writer, "property uchar green")?;
            writeln!(writer, "property uchar blue")?;
            writeln!(writer, "property uchar alpha")?;
        }

        if !self.faces.is_empty() {
            writeln!(writer, "element face {}", self.faces.len())?;
            writeln!(writer, "property list uchar int vertex_indices")?;
        }

        writeln!(writer, "end_header")?;
        Ok(())
    }

    fn write_ascii_body<W: Write>(
        &self,
        writer: &mut W,
        has_normals: bool,
        has_colors: bool,
    ) -> io::Result<()> {
        for (i, [x, y, z]) in self.positions.iter().enumerate() {
            write!(writer, "{:.6} {:.6} {:.6}", x, y, z)?;

            if has_normals {
                let [nx, ny, nz] = self.normals[i];
                write!(writer, " {:.6} {:.6} {:.6}", nx, ny, nz)?;
            }

            if has_colors {
                let [r, g, b, a] = self.colors[i];
                write!(writer, " {} {} {} {}", r, g, b, a)?;
            }

            writeln!(writer)?;
        }

        // Write faces
        for face in &self.faces {
            writeln!(writer, "3 {} {} {}", face[0], face[1], face[2])?;
        }

        Ok(())
    }

    fn write_binary_little_endian_body<W: Write>(
        &self,
        writer: &mut W,
        has_normals: bool,
        has_colors: bool,
    ) -> io::Result<()> {
        for (i, [x, y, z]) in self.positions.iter().enumerate() {
            writer.write_all(&x.to_le_bytes())?;
            writer.write_all(&y.to_le_bytes())?;
            writer.write_all(&z.to_le_bytes())?;

            if has_normals {
                let [nx, ny, nz] = self.normals[i];
                writer.write_all(&nx.to_le_bytes())?;
                writer.write_all(&ny.to_le_bytes())?;
                writer.write_all(&nz.to_le_bytes())?;
            }

            if has_colors {
                writer.write_all(&self.colors[i])?;
            }
        }

        for face in &self.faces {
            writer.write_all(&[3u8])?;
            for index in face {
                let index = i32::try_from(*index).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "PLY binary writer only supports face indices up to i32::MAX",
                    )
                })?;
                writer.write_all(&index.to_le_bytes())?;
            }
        }

        Ok(())
    }
}

/// Read a float3 from an attribute at a given point index.
fn read_float3(mesh: &Mesh, att_id: i32, point_idx: usize) -> [f32; 3] {
    let att = mesh.attribute(att_id);
    let byte_stride = att.byte_stride() as usize;
    let buffer = att.buffer();
    let mut bytes = [0u8; 12];
    buffer.read(point_idx * byte_stride, &mut bytes);
    [
        f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
    ]
}

/// Read a color from an attribute at a given point index.
fn read_color(mesh: &Mesh, att_id: i32, point_idx: usize) -> [u8; 4] {
    let att = mesh.attribute(att_id);
    let byte_stride = att.byte_stride() as usize;
    let buffer = att.buffer();

    // Colors can be stored in different formats
    let num_components = att.num_components() as usize;
    let component_size = byte_stride / num_components;

    if component_size == 1 {
        // u8 colors
        let mut bytes = [255u8; 4];
        let read_len = num_components.min(4);
        buffer.read(point_idx * byte_stride, &mut bytes[..read_len]);
        bytes
    } else if component_size == 4 {
        // f32 colors (0.0-1.0) - convert to u8
        let mut float_bytes = [0u8; 16];
        let read_len = (num_components * 4).min(16);
        buffer.read(point_idx * byte_stride, &mut float_bytes[..read_len]);

        let mut result = [255u8; 4];
        for i in 0..num_components.min(4) {
            let f = f32::from_le_bytes([
                float_bytes[i * 4],
                float_bytes[i * 4 + 1],
                float_bytes[i * 4 + 2],
                float_bytes[i * 4 + 3],
            ]);
            result[i] = (f.clamp(0.0, 1.0) * 255.0) as u8;
        }
        result
    } else {
        [255, 255, 255, 255] // Default white
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Writer for PlyWriter {
    fn new() -> Self {
        Self::default()
    }

    fn add_mesh(&mut self, mesh: &Mesh, _name: Option<&str>) -> io::Result<()> {
        // PLY format doesn't support mesh names
        let vertex_offset = self.positions.len() as u32;

        // Extract positions
        let pos_att_id = mesh.named_attribute_id(GeometryAttributeType::Position);
        if pos_att_id >= 0 {
            for i in 0..mesh.num_points() {
                self.positions.push(read_float3(mesh, pos_att_id, i));
            }
        }

        // Extract normals if present
        let normal_att_id = mesh.named_attribute_id(GeometryAttributeType::Normal);
        if normal_att_id >= 0 {
            // Pad normals if we've added vertices without normals before
            while self.normals.len() < self.positions.len() - mesh.num_points() {
                self.normals.push([0.0, 0.0, 0.0]);
            }
            for i in 0..mesh.num_points() {
                self.normals.push(read_float3(mesh, normal_att_id, i));
            }
        }

        // Extract colors if present
        let color_att_id = mesh.named_attribute_id(GeometryAttributeType::Color);
        if color_att_id >= 0 {
            // Pad colors if we've added vertices without colors before
            while self.colors.len() < self.positions.len() - mesh.num_points() {
                self.colors.push([255, 255, 255, 255]);
            }
            for i in 0..mesh.num_points() {
                self.colors.push(read_color(mesh, color_att_id, i));
            }
        }

        // Extract faces (0-based indices with offset)
        for i in 0..mesh.num_faces() as u32 {
            let face = mesh.face(FaceIndex(i));
            self.faces.push([
                face[0].0 + vertex_offset,
                face[1].0 + vertex_offset,
                face[2].0 + vertex_offset,
            ]);
        }
        Ok(())
    }

    fn write<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self.write(path)
    }

    fn vertex_count(&self) -> usize {
        self.vertex_count()
    }

    fn face_count(&self) -> usize {
        self.face_count()
    }
}

impl PointCloudWriter for PlyWriter {
    fn add_points(&mut self, points: &[[f32; 3]]) {
        self.add_points(points);
    }

    fn add_point(&mut self, point: [f32; 3]) {
        self.add_point(point);
    }
}

// ============================================================================
// Convenience Functions (for backward compatibility)
// ============================================================================

/// Write a mesh to a PLY file.
///
/// This is a convenience function. For more control, use `PlyWriter` directly.
pub fn write_ply_mesh<P: AsRef<Path>>(path: P, mesh: &Mesh) -> io::Result<()> {
    let mut writer = PlyWriter::new();
    Writer::add_mesh(&mut writer, mesh, None)?;
    writer.write(path)
}

/// Write point positions to a PLY file (point cloud, no faces).
///
/// This is a convenience function. For more control, use `PlyWriter` directly.
pub fn write_ply_positions<P: AsRef<Path>>(path: P, points: &[[f32; 3]]) -> io::Result<()> {
    let mut writer = PlyWriter::new();
    writer.add_points(points);
    writer.write(path)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "decoder")]
    use crate::ply_reader::PlyReader;
    use draco_core::draco_types::DataType;
    use draco_core::geometry_attribute::PointAttribute;
    use draco_core::geometry_indices::PointIndex;
    use std::fs;
    use tempfile::NamedTempFile;

    fn create_triangle_mesh() -> Mesh {
        let mut mesh = Mesh::new();
        let mut pos_att = PointAttribute::new();

        pos_att.init(GeometryAttributeType::Position, 3, DataType::Float32, false, 3);
        let buffer = pos_att.buffer_mut();
        let positions: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        for (i, pos) in positions.iter().enumerate() {
            let bytes: Vec<u8> = pos.iter().flat_map(|v| v.to_le_bytes()).collect();
            buffer.write(i * 12, &bytes);
        }
        mesh.add_attribute(pos_att);

        mesh.set_num_faces(1);
        mesh.set_face(FaceIndex(0), [PointIndex(0), PointIndex(1), PointIndex(2)]);

        mesh
    }

    #[test]
    fn test_ply_writer_new() {
        let writer = PlyWriter::new();
        assert_eq!(writer.vertex_count(), 0);
        assert_eq!(writer.face_count(), 0);
        assert!(!writer.has_normals());
        assert!(!writer.has_colors());
        assert!(!writer.is_binary_little_endian());
    }

    #[test]
    fn test_ply_writer_add_mesh() {
        let mesh = create_triangle_mesh();
        let mut writer = PlyWriter::new();
        Writer::add_mesh(&mut writer, &mesh, None).unwrap();
        assert_eq!(writer.vertex_count(), 3);
        assert_eq!(writer.face_count(), 1);
    }

    #[test]
    fn test_ply_writer_add_points() {
        let mut writer = PlyWriter::new();
        writer.add_points(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
        assert_eq!(writer.vertex_count(), 2);
        assert_eq!(writer.face_count(), 0);
    }

    #[test]
    fn test_ply_writer_add_points_with_colors() {
        let mut writer = PlyWriter::new();
        writer.add_points_with_colors(
            &[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
            &[[255, 0, 0, 255], [0, 255, 0, 255]],
        );
        assert_eq!(writer.vertex_count(), 2);
        assert!(writer.has_colors());
    }

    #[test]
    fn test_write_ply_positions() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ];

        let file = NamedTempFile::new().unwrap();
        write_ply_positions(file.path(), &points).unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("ply"));
        assert!(content.contains("format ascii 1.0"));
        assert!(content.contains("element vertex 3"));
        assert!(content.contains("property float x"));
        assert!(content.contains("end_header"));
        assert!(content.contains("0.000000 0.000000 0.000000"));
        assert!(content.contains("1.000000 0.000000 0.000000"));
    }

    #[test]
    fn test_write_ply_mesh() {
        let mesh = create_triangle_mesh();
        let file = NamedTempFile::new().unwrap();
        write_ply_mesh(file.path(), &mesh).unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("ply"));
        assert!(content.contains("element vertex 3"));
        assert!(content.contains("element face 1"));
        assert!(content.contains("property list uchar int vertex_indices"));
        assert!(content.contains("3 0 1 2")); // face with 0-based indices
    }

    #[test]
    fn test_multiple_meshes() {
        let mesh1 = create_triangle_mesh();
        let mesh2 = create_triangle_mesh();

        let mut writer = PlyWriter::new();
        Writer::add_mesh(&mut writer, &mesh1, None).unwrap();
        Writer::add_mesh(&mut writer, &mesh2, None).unwrap();

        assert_eq!(writer.vertex_count(), 6);
        assert_eq!(writer.face_count(), 2);

        let file = NamedTempFile::new().unwrap();
        writer.write(file.path()).unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("element vertex 6"));
        assert!(content.contains("element face 2"));
        // Second mesh should have offset indices
        assert!(content.contains("3 3 4 5"));
    }

    #[test]
    fn test_ply_with_colors() {
        let mut writer = PlyWriter::new();
        writer.add_points_with_colors(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            &[[255, 0, 0, 255], [0, 255, 0, 255]],
        );

        let file = NamedTempFile::new().unwrap();
        writer.write(file.path()).unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("property uchar red"));
        assert!(content.contains("property uchar green"));
        assert!(content.contains("property uchar blue"));
        assert!(content.contains("property uchar alpha"));
        assert!(content.contains("255 0 0 255"));
        assert!(content.contains("0 255 0 255"));
    }

    #[test]
    fn test_ply_writer_can_switch_to_binary_little_endian() {
        let writer = PlyWriter::new().with_binary_little_endian();
        assert!(writer.is_binary_little_endian());

        let mut writer = PlyWriter::new();
        writer.set_binary_little_endian(true);
        assert!(writer.is_binary_little_endian());
        writer.set_binary_little_endian(false);
        assert!(!writer.is_binary_little_endian());
    }

    #[cfg(feature = "decoder")]
    #[test]
    fn test_write_binary_little_endian_positions_roundtrip() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        ];

        let file = NamedTempFile::new().unwrap();
        let mut writer = PlyWriter::new().with_binary_little_endian();
        writer.add_points(&points);
        writer.write(file.path()).unwrap();

        let content = fs::read(file.path()).unwrap();
        let header_end = content
            .windows(b"end_header\n".len())
            .position(|window| window == b"end_header\n")
            .map(|idx| idx + b"end_header\n".len())
            .unwrap();
        let header = std::str::from_utf8(&content[..header_end]).unwrap();
        assert!(header.contains("format binary_little_endian 1.0"));

        let mut reader = PlyReader::open(file.path()).unwrap();
        let positions = reader.read_positions().unwrap();
        assert_eq!(positions, points);
    }

    #[cfg(feature = "decoder")]
    #[test]
    fn test_write_binary_little_endian_mesh_roundtrip() {
        let mesh = create_triangle_mesh();
        let file = NamedTempFile::new().unwrap();

        let mut writer = PlyWriter::new().with_binary_little_endian();
        Writer::add_mesh(&mut writer, &mesh, None).unwrap();
        writer.write(file.path()).unwrap();

        let bytes = fs::read(file.path()).unwrap();
        let header_end = bytes
            .windows(b"end_header\n".len())
            .position(|window| window == b"end_header\n")
            .map(|idx| idx + b"end_header\n".len())
            .unwrap();
        let header = std::str::from_utf8(&bytes[..header_end]).unwrap();
        assert!(header.contains("format binary_little_endian 1.0"));
        assert!(header.contains("element vertex 3"));
        assert!(header.contains("element face 1"));

        let mut reader = PlyReader::open(file.path()).unwrap();
        let mesh = reader.read_mesh().unwrap();
        assert_eq!(mesh.num_points(), 3);
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(
            mesh.face(FaceIndex(0)),
            [PointIndex(0), PointIndex(1), PointIndex(2)]
        );
    }
}
