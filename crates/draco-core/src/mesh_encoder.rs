use crate::mesh::Mesh;
use crate::point_cloud::PointCloud;
use crate::draco_types::DataType;
use crate::encoder_buffer::EncoderBuffer;
use crate::status::{Status, DracoError};
use crate::encoder_options::EncoderOptions;
use crate::compression_config::EncodedGeometryType;
use crate::corner_table::CornerTable;
use crate::point_cloud_encoder::GeometryEncoder;
use crate::sequential_integer_attribute_encoder::SequentialIntegerAttributeEncoder;
use crate::sequential_attribute_encoder::SequentialAttributeEncoder;
use crate::sequential_normal_attribute_encoder::SequentialNormalAttributeEncoder;
use crate::geometry_indices::{PointIndex, FaceIndex};
use crate::mesh_edgebreaker_encoder::MeshEdgebreakerEncoder;
use crate::compression_config::MeshEncodingMethod;
use crate::attribute_quantization_transform::AttributeQuantizationTransform;
use crate::geometry_attribute::{PointAttribute, GeometryAttributeType};
use crate::attribute_transform::AttributeTransform;
use crate::version::{DEFAULT_MESH_VERSION, has_header_flags, uses_varint_encoding, uses_varint_unique_id};

/// MeshEncoder provides basic functionality for encoding mesh data.
/// This is an abstract base that can be specialized for different mesh encoding methods.
pub struct MeshEncoder {
    mesh: Option<Mesh>,
    options: EncoderOptions,
    num_encoded_faces: usize,
    corner_table: Option<CornerTable>,
    point_ids: Vec<PointIndex>,
    data_to_corner_map: Option<Vec<u32>>,
    method: i32,
}

impl GeometryEncoder for MeshEncoder {
    fn point_cloud(&self) -> Option<&PointCloud> {
        self.mesh.as_ref().map(|m| m as &PointCloud)
    }

    fn mesh(&self) -> Option<&Mesh> {
        self.mesh.as_ref()
    }

    fn corner_table(&self) -> Option<&CornerTable> {
        self.corner_table.as_ref()
    }

    fn options(&self) -> &EncoderOptions {
        &self.options
    }

    fn get_geometry_type(&self) -> EncodedGeometryType {
        EncodedGeometryType::TriangularMesh
    }

    fn get_encoding_method(&self) -> Option<i32> {
        Some(self.method)
    }

    fn get_data_to_corner_map(&self) -> Option<Vec<u32>> {
        self.data_to_corner_map.clone()
    }
}

impl MeshEncoder {
    pub fn new() -> Self {
        Self {
            mesh: None,
            options: EncoderOptions::default(),
            num_encoded_faces: 0,
            corner_table: None,
            point_ids: Vec::new(),
            data_to_corner_map: None,
            method: 0,
        }
    }

    pub fn set_mesh(&mut self, mesh: Mesh) {
        self.mesh = Some(mesh);
    }

    pub fn mesh(&self) -> Option<&Mesh> {
        self.mesh.as_ref()
    }

    pub fn num_encoded_faces(&self) -> usize {
        self.num_encoded_faces
    }

    pub fn corner_table(&self) -> Option<&CornerTable> {
        self.corner_table.as_ref()
    }

    pub fn encode(&mut self, options: &EncoderOptions, out_buffer: &mut EncoderBuffer) -> Status {
        self.options = options.clone();

        if self.mesh.is_none() {
            return Err(DracoError::DracoError("Mesh not set".to_string()));
        }

        // 1. Encode Header
        self.encode_header(out_buffer)?;

        // 2. Encode geometry data (connectivity + attributes)
        self.encode_geometry_data(out_buffer)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn encode_metadata(&self, buffer: &mut EncoderBuffer) -> Status {
        buffer.encode_varint(0u64); // 0 metadata
        Ok(())
    }

    fn encode_header(&self, buffer: &mut EncoderBuffer) -> Status {
        buffer.encode_data(b"DRACO");
        
        let (mut major, mut minor) = self.options.get_version();
        if major == 0 && minor == 0 {
            // Default to latest mesh version
            (major, minor) = DEFAULT_MESH_VERSION;
        }
        
        buffer.encode_u8(major);
        buffer.encode_u8(minor);
        buffer.set_version(major, minor);
        buffer.encode_u8(self.get_geometry_type() as u8);
        
        let method_int = self.options.get_global_int("encoding_method", -1);
        let method = if method_int == 1 { 1 } else { 0 };
        buffer.encode_u8(method);
        
        if has_header_flags(major, minor) {
            buffer.encode_u16(0); // Flags
        }
        Ok(())
    }

    fn encode_geometry_data(&mut self, out_buffer: &mut EncoderBuffer) -> Status {
        // First encode connectivity
        self.encode_connectivity(out_buffer)?;

        // Check if we should store the number of encoded faces
        if self.options.get_global_int("store_number_of_encoded_faces", 0) != 0 {
            self.compute_number_of_encoded_faces();
        }

        // Then encode attributes
        self.encode_attributes(out_buffer)?;

        Ok(())
    }

    fn encode_connectivity(&mut self, out_buffer: &mut EncoderBuffer) -> Status {
        let mesh = self.mesh.as_ref().unwrap();

        // Build faces array for corner table
        let faces: Vec<[crate::geometry_indices::VertexIndex; 3]> = (0..mesh.num_faces())
            .map(|i| {
                let face = mesh.face(FaceIndex(i as u32));
                [
                    crate::geometry_indices::VertexIndex(face[0].0),
                    crate::geometry_indices::VertexIndex(face[1].0),
                    crate::geometry_indices::VertexIndex(face[2].0),
                ]
            })
            .collect();

        // Initialize corner table for the mesh
        let mut corner_table = CornerTable::new(0);
        corner_table.init(&faces);

        self.corner_table = Some(corner_table);

        let method_int = self.options.get_global_int("encoding_method", -1);
        let method = if method_int == 1 {
            MeshEncodingMethod::MeshEdgebreakerEncoding
        } else {
            MeshEncodingMethod::MeshSequentialEncoding
        };
        self.method = if method == MeshEncodingMethod::MeshEdgebreakerEncoding { 1 } else { 0 };

        match method {
            MeshEncodingMethod::MeshSequentialEncoding => self.encode_sequential_connectivity(out_buffer),
            MeshEncodingMethod::MeshEdgebreakerEncoding => self.encode_edgebreaker_connectivity(out_buffer),
        }
    }

    fn encode_edgebreaker_connectivity(&mut self, out_buffer: &mut EncoderBuffer) -> Status {
        let mesh = self.mesh.as_ref().unwrap();
        let corner_table = self.corner_table.as_ref().unwrap();
        
        let mut encoder = MeshEdgebreakerEncoder::new(mesh.num_faces(), mesh.num_points());
        let (point_ids, data_to_corner_map) =
            encoder.encode_connectivity(mesh, corner_table, out_buffer)?;
        self.point_ids = point_ids;
        // Draco stores corner mapping in attribute (data) order.
        self.data_to_corner_map = Some(data_to_corner_map);

        Ok(())
    }

    fn encode_sequential_connectivity(&mut self, out_buffer: &mut EncoderBuffer) -> Status {
        let mesh = self.mesh.as_ref().unwrap();

        // Encode the number of faces and points
        // Use the buffer's version (set in encode_header) for version checks
        let major = out_buffer.version_major();
        let minor = out_buffer.version_minor();
        if !uses_varint_encoding(major, minor) {
            out_buffer.encode_u32(mesh.num_faces() as u32);
            out_buffer.encode_u32(mesh.num_points() as u32);
        } else {
            out_buffer.encode_varint(mesh.num_faces() as u64);
            out_buffer.encode_varint(mesh.num_points() as u64);
        }

        if mesh.num_faces() > 0 && mesh.num_points() > 0 {
            out_buffer.encode_u8(1); // Raw connectivity
            if mesh.num_points() < 256 {
                for face_id in 0..mesh.num_faces() {
                    let face = mesh.face(FaceIndex(face_id as u32));
                    for i in 0..3 {
                        out_buffer.encode_u8(face[i].0 as u8);
                    }
                }
            } else if mesh.num_points() < 65536 {
                for face_id in 0..mesh.num_faces() {
                    let face = mesh.face(FaceIndex(face_id as u32));
                    for i in 0..3 {
                        out_buffer.encode_u16(face[i].0 as u16);
                    }
                }
            } else {
                for face_id in 0..mesh.num_faces() {
                    let face = mesh.face(FaceIndex(face_id as u32));
                    for i in 0..3 {
                        out_buffer.encode_u32(face[i].0 as u32);
                    }
                }
            }
        }

        // Identity permutation for sequential encoding
        self.point_ids = (0..mesh.num_points())
            .map(|i| PointIndex(i as u32))
            .collect();

        Ok(())
    }

    fn encode_attributes(&mut self, out_buffer: &mut EncoderBuffer) -> Status {
        let mesh = self.mesh.as_ref().unwrap();
        
        let method_int = self.options.get_global_int("encoding_method", -1);
        let is_edgebreaker = method_int == 1;

        // Encode number of attribute decoders (u8).
        // For sequential encoding, there's only ONE attribute encoder containing ALL attributes.
        // For edgebreaker, there may be multiple encoders.
        if is_edgebreaker {
            out_buffer.encode_u8(mesh.num_attributes() as u8);
        } else {
            out_buffer.encode_u8(1u8); // Sequential: single encoder with all attributes
        }

        // Phase 1: attributes decoder identifiers.
        // For sequential encoding, we only have one encoder so this is just one loop iteration.
        // For edgebreaker, we need one per attribute.
        let num_encoders = if is_edgebreaker { mesh.num_attributes() as usize } else { 1 };
        for _ in 0..num_encoders {
            if is_edgebreaker {
                // att_data_id (i8), encoder_type (u8), traversal_method (u8)
                out_buffer.encode_u8((-1i8) as u8);
                out_buffer.encode_u8(0);
                out_buffer.encode_u8(0);
            }
            // For sequential, nothing is written in phase 1 (EncodeAttributesEncoderIdentifier does nothing)
        }

        let mut decoder_types: Vec<u8> = Vec::with_capacity(mesh.num_attributes() as usize);
        // Use the buffer's version (set in encode_header) for version checks
        let major = out_buffer.version_major();
        let minor = out_buffer.version_minor();

        // Phase 2: Encode attribute encoder data
        // For sequential encoding: ONE encoder containing ALL attributes
        //   - Write num_attrs = total attributes
        //   - Write all attribute metadata
        //   - Write all decoder types
        // For edgebreaker: multiple encoders, one per attribute
        //   - Each encoder: num_attrs=1, metadata, decoder_type
        
        if is_edgebreaker {
            // Edgebreaker: multiple encoders, one per attribute
            for i in 0..mesh.num_attributes() {
                let att = mesh.attribute(i as i32);
                let quantization_bits = self.options.get_attribute_int(i as i32, "quantization_bits", -1);
                let is_quantized = quantization_bits > 0
                    && (att.data_type() == DataType::Float32 || att.data_type() == DataType::Float64);
                let is_normal = att.attribute_type() == GeometryAttributeType::Normal;

                // num_attributes = 1 for this encoder
                if !uses_varint_encoding(major, minor) {
                    out_buffer.encode_u32(1);
                } else {
                    out_buffer.encode_varint(1u64);
                }
                
                out_buffer.encode_u8(att.attribute_type() as u8);
                out_buffer.encode_u8(att.data_type() as u8);
                out_buffer.encode_u8(att.num_components());
                out_buffer.encode_u8(if att.normalized() { 1 } else { 0 });
                
                if !uses_varint_unique_id(major, minor) {
                    out_buffer.encode_u16(att.unique_id() as u16);
                } else {
                    out_buffer.encode_varint(att.unique_id() as u64);
                }

                let decoder_type: u8 = if is_quantized {
                    if is_normal { 3 } else { 2 }
                } else if att.data_type() != DataType::Float32 {
                    1
                } else {
                    0
                };
                out_buffer.encode_u8(decoder_type);
                decoder_types.push(decoder_type);
            }
        } else {
            // Sequential: single encoder with all attributes
            // Write num_attrs = total number of attributes
            if !uses_varint_encoding(major, minor) {
                out_buffer.encode_u32(mesh.num_attributes() as u32);
            } else {
                out_buffer.encode_varint(mesh.num_attributes() as u64);
            }
            
            // Write all attribute metadata first
            for i in 0..mesh.num_attributes() {
                let att = mesh.attribute(i as i32);
                
                out_buffer.encode_u8(att.attribute_type() as u8);
                out_buffer.encode_u8(att.data_type() as u8);
                out_buffer.encode_u8(att.num_components());
                out_buffer.encode_u8(if att.normalized() { 1 } else { 0 });
                
                if !uses_varint_unique_id(major, minor) {
                    out_buffer.encode_u16(att.unique_id() as u16);
                } else {
                    out_buffer.encode_varint(att.unique_id() as u64);
                }
            }
            
            // Write all decoder types after all metadata (SequentialAttributeEncodersController pattern)
            for i in 0..mesh.num_attributes() {
                let att = mesh.attribute(i as i32);
                let quantization_bits = self.options.get_attribute_int(i as i32, "quantization_bits", -1);
                let is_quantized = quantization_bits > 0
                    && (att.data_type() == DataType::Float32 || att.data_type() == DataType::Float64);
                let is_normal = att.attribute_type() == GeometryAttributeType::Normal;

                let decoder_type: u8 = if is_quantized {
                    if is_normal { 3 } else { 2 }
                } else if att.data_type() != DataType::Float32 {
                    1
                } else {
                    0
                };
                out_buffer.encode_u8(decoder_type);
                decoder_types.push(decoder_type);
            }
        }

        // Phase 3: Encode attribute values (all attributes first) 
        // C++ order: all EncodePortableAttribute calls, then all EncodeDataNeededByPortableTransform calls
        
        // Store transforms and encoders for later use in transform data encoding
        let mut quantization_transforms: Vec<Option<AttributeQuantizationTransform>> = Vec::new();
        let mut portable_attributes: Vec<Option<PointAttribute>> = Vec::new();
        let mut normal_encoders: Vec<Option<SequentialNormalAttributeEncoder>> = Vec::new();
        
        // First pass: encode all attribute VALUES
        for i in 0..mesh.num_attributes() {
            let att = mesh.attribute(i as i32);
            let decoder_type = decoder_types[i as usize];
            let quantization_bits = self.options.get_attribute_int(i as i32, "quantization_bits", -1);

            match decoder_type {
                3 => {
                    // Normal attribute with octahedral encoding
                    let mut encoder = SequentialNormalAttributeEncoder::new();
                    if !encoder.init(self.point_cloud().unwrap(), i as i32, &self.options) {
                        return Err(DracoError::DracoError("Failed to init normal encoder".to_string()));
                    }
                    if !encoder.encode_values(self.point_cloud().unwrap(), &self.point_ids, out_buffer, &self.options, self) {
                        return Err(DracoError::DracoError("Failed to encode normal values".to_string()));
                    }
                    normal_encoders.push(Some(encoder));
                    quantization_transforms.push(None);
                    portable_attributes.push(None);
                }
                2 => {
                    // Quantized attribute
                    let mut q_transform = AttributeQuantizationTransform::new();
                    if !q_transform.compute_parameters(att, quantization_bits) {
                        return Err(DracoError::DracoError("Failed to compute quantization parameters".to_string()));
                    }
                    let mut portable = PointAttribute::default();
                    if !q_transform.transform_attribute(att, &self.point_ids, &mut portable) {
                        return Err(DracoError::DracoError("Failed to quantize attribute".to_string()));
                    }

                    let mut att_encoder = SequentialIntegerAttributeEncoder::new();
                    att_encoder.init(i as i32);
                    if !att_encoder.encode_values(
                        mesh as &PointCloud,
                        &self.point_ids,
                        out_buffer,
                        &self.options,
                        self,
                        Some(&portable),
                        true,
                    ) {
                        return Err(DracoError::DracoError(format!("Failed to encode attribute {}", i)));
                    }
                    
                    quantization_transforms.push(Some(q_transform));
                    portable_attributes.push(Some(portable));
                    normal_encoders.push(None);
                }
                1 => {
                    // Integer attribute
                    let mut att_encoder = SequentialIntegerAttributeEncoder::new();
                    att_encoder.init(i as i32);
                    if !att_encoder.encode_values(
                        mesh as &PointCloud,
                        &self.point_ids,
                        out_buffer,
                        &self.options,
                        self,
                        None,
                        true,
                    ) {
                        return Err(DracoError::DracoError(format!("Failed to encode attribute {}", i)));
                    }
                    quantization_transforms.push(None);
                    portable_attributes.push(None);
                    normal_encoders.push(None);
                }
                0 => {
                    // Generic/float attribute
                    let mut att_encoder = SequentialAttributeEncoder::new();
                    att_encoder.init(i as i32);
                    if !att_encoder.encode_values(mesh as &PointCloud, &self.point_ids, out_buffer) {
                        return Err(DracoError::DracoError(format!("Failed to encode attribute {}", i)));
                    }
                    quantization_transforms.push(None);
                    portable_attributes.push(None);
                    normal_encoders.push(None);
                }
                _ => {
                    return Err(DracoError::DracoError(format!("Unsupported encoder type {}", decoder_type)));
                }
            }
        }
        
        // Second pass: encode all TRANSFORM DATA
        for i in 0..mesh.num_attributes() {
            let decoder_type = decoder_types[i as usize];

            match decoder_type {
                3 => {
                    // Normal attribute - encode octahedral transform data
                    if let Some(ref encoder) = normal_encoders[i as usize] {
                        if !encoder.encode_data_needed_by_portable_transform(out_buffer) {
                            return Err(DracoError::DracoError("Failed to encode normal transform data".to_string()));
                        }
                    }
                }
                2 => {
                    // Quantized attribute - encode quantization parameters
                    if let Some(ref q_transform) = quantization_transforms[i as usize] {
                        if !q_transform.encode_parameters(out_buffer) {
                            return Err(DracoError::DracoError("Failed to encode quantization parameters".to_string()));
                        }
                    }
                }
                1 | 0 => {
                    // No transform data for integer/generic attributes
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn compute_number_of_encoded_faces(&mut self) {
        if let Some(ref mesh) = self.mesh {
            self.num_encoded_faces = mesh.num_faces();
        }
    }
}

impl Default for MeshEncoder {
    fn default() -> Self {
        Self::new()
    }
}
