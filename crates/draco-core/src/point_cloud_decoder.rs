use crate::point_cloud::PointCloud;
use crate::decoder_buffer::DecoderBuffer;
use crate::status::{Status, DracoError};
use crate::compression_config::EncodedGeometryType;
use crate::geometry_indices::PointIndex;
use crate::geometry_attribute::{PointAttribute, GeometryAttributeType};
use crate::draco_types::DataType;
use crate::sequential_integer_attribute_decoder::SequentialIntegerAttributeDecoder;
use crate::kd_tree_attributes_decoder::KdTreeAttributesDecoder;
use crate::mesh::Mesh;
use crate::corner_table::CornerTable;

use crate::attribute_octahedron_transform::AttributeOctahedronTransform;
use crate::attribute_quantization_transform::AttributeQuantizationTransform;
use crate::attribute_transform::AttributeTransform;
use crate::version::has_header_flags;

pub trait GeometryDecoder {
    fn point_cloud(&self) -> Option<&PointCloud>;
    fn mesh(&self) -> Option<&Mesh>;
    fn corner_table(&self) -> Option<&CornerTable>;
    fn get_geometry_type(&self) -> EncodedGeometryType;
    fn get_attribute_encoding_method(&self, _att_id: i32) -> Option<i32> { None }
}

pub struct PointCloudDecoder {
    geometry_type: EncodedGeometryType,
    method: u8,
    version_major: u8,
    version_minor: u8,
}

impl GeometryDecoder for PointCloudDecoder {
    fn point_cloud(&self) -> Option<&PointCloud> {
        None // PointCloudDecoder constructs PointCloud, doesn't hold it?
        // Actually decode takes &mut PointCloud.
        // So we can't return it here easily unless we store it.
        // But GeometryDecoder is usually passed to attribute decoders.
        // Attribute decoders take PointCloud as argument.
    }

    fn mesh(&self) -> Option<&Mesh> {
        None
    }

    fn corner_table(&self) -> Option<&CornerTable> {
        None
    }

    fn get_geometry_type(&self) -> EncodedGeometryType {
        self.geometry_type
    }
}

impl Default for PointCloudDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl PointCloudDecoder {
    pub fn new() -> Self {
        Self {
            geometry_type: EncodedGeometryType::PointCloud,
            method: 0,
            version_major: 0,
            version_minor: 0,
        }
    }

    pub fn decode(&mut self, in_buffer: &mut DecoderBuffer, out_pc: &mut PointCloud) -> Status {
        // 1. Decode Header
        self.decode_header(in_buffer)?;
        
        // 2. Decode Geometry Data
        self.decode_geometry_data(in_buffer, out_pc)
    }

    fn decode_header(&mut self, buffer: &mut DecoderBuffer) -> Status {
        let mut magic = [0u8; 5];
        buffer.decode_bytes(&mut magic)?;
        if &magic != b"DRACO" {
            return Err(DracoError::DracoError("Invalid magic".to_string()));
        }
        
           self.version_major = buffer.decode_u8()?;
           self.version_minor = buffer.decode_u8()?;
           buffer.set_version(self.version_major, self.version_minor);
        
        let g_type = buffer.decode_u8()?;
        self.geometry_type = match g_type {
            0 => EncodedGeometryType::PointCloud,
            1 => EncodedGeometryType::TriangularMesh,
            _ => return Err(DracoError::DracoError("Invalid geometry type".to_string())),
        };
        
        self.method = buffer.decode_u8()?;

        // Flags are present starting with bitstream v1.3.
        if has_header_flags(self.version_major, self.version_minor) {
            let _flags = buffer
                .decode_u16()
                .map_err(|_| DracoError::DracoError("Failed to decode flags".to_string()))?;
            let _ = _flags;
        }
        
        Ok(())
    }

    fn decode_geometry_data(&mut self, buffer: &mut DecoderBuffer, pc: &mut PointCloud) -> Status {
        let bitstream_version: u16 = ((self.version_major as u16) << 8) | (self.version_minor as u16);
        // Note: Draco point cloud bitstreams encode the number of points as a
        // fixed-width int32 for both sequential (method=0) and KD-tree
        // (method=1) encodings (see C++ PointCloudSequentialDecoder and
        // PointCloudKdTreeDecoder). It is NOT varint encoded, even for v2.x.
        let num_points: usize = buffer.decode_u32()? as usize;
        pc.set_num_points(num_points);

        let num_attributes_decoders = buffer.decode_u8()? as usize;
        
        if self.method == 1 {
            // KD-tree encoding.
            for _ in 0..num_attributes_decoders {
                let mut att_decoder = KdTreeAttributesDecoder::new(0);
                if !att_decoder.decode_attributes_decoder_data(pc, buffer) {
                    return Err(DracoError::DracoError("Failed to decode attribute metadata".to_string()));
                }
                if !att_decoder.decode_attributes(pc, buffer) {
                    return Err(DracoError::DracoError("Failed to decode attributes".to_string()));
                }
            }
        } else {
            // Sequential encoding.
            let point_ids: Vec<PointIndex> = (0..num_points).map(|i| PointIndex(i as u32)).collect();

            struct PendingQuant {
                att_id: i32,
                portable: PointAttribute,
                transform: AttributeQuantizationTransform,
            }

            struct PendingNormal {
                att_id: i32,
                portable: PointAttribute,
                quantization_bits: u8,
            }

            for _ in 0..num_attributes_decoders {
                let num_attributes_in_decoder: usize = if bitstream_version < 0x0200 {
                    buffer.decode_u32()? as usize
                } else {
                    buffer.decode_varint()? as usize
                };
                if num_attributes_in_decoder == 0 {
                    return Err(DracoError::DracoError("Invalid number of attributes".to_string()));
                }

                let mut att_ids: Vec<i32> = Vec::with_capacity(num_attributes_in_decoder);
                let mut decoder_types: Vec<u8> = Vec::with_capacity(num_attributes_in_decoder);
                let mut pending_quant: Vec<PendingQuant> = Vec::new();
                let mut pending_normals: Vec<PendingNormal> = Vec::new();

                for _ in 0..num_attributes_in_decoder {
                    let att_type_val = buffer.decode_u8()?;
                    let att_type = match att_type_val {
                        0 => GeometryAttributeType::Position,
                        1 => GeometryAttributeType::Normal,
                        2 => GeometryAttributeType::Color,
                        3 => GeometryAttributeType::TexCoord,
                        4 => GeometryAttributeType::Generic,
                        _ => GeometryAttributeType::Invalid,
                    };

                    let data_type_val = buffer.decode_u8()?;
                    let data_type = match data_type_val {
                        1 => DataType::Int8,
                        2 => DataType::Uint8,
                        3 => DataType::Int16,
                        4 => DataType::Uint16,
                        5 => DataType::Int32,
                        6 => DataType::Uint32,
                        7 => DataType::Int64,
                        8 => DataType::Uint64,
                        9 => DataType::Float32,
                        10 => DataType::Float64,
                        11 => DataType::Bool,
                        _ => DataType::Invalid,
                    };

                    let num_components = buffer.decode_u8()?;
                    let normalized = buffer.decode_u8()? != 0;
                    let unique_id: u32 = if bitstream_version < 0x0103 {
                        buffer.decode_u16()? as u32
                    } else {
                        buffer.decode_varint()? as u32
                    };

                    let mut att = PointAttribute::new();
                    att.init(att_type, num_components, data_type, normalized, num_points);
                    att.set_unique_id(unique_id);
                    let att_id = pc.add_attribute(att);
                    att_ids.push(att_id);
                }

                for _ in 0..num_attributes_in_decoder {
                    decoder_types.push(buffer.decode_u8()?);
                }

                for (local_i, &att_id) in att_ids.iter().enumerate() {
                    let decoder_type = decoder_types[local_i];
                    match decoder_type {
                        1 => {
                            let mut att_decoder = SequentialIntegerAttributeDecoder::new();
                            att_decoder.init(self, att_id);
                            if !att_decoder.decode_values(pc, &point_ids, buffer, None, None, None) {
                                return Err(DracoError::DracoError("Failed to decode integer attribute".to_string()));
                            }
                        }
                        2 => {
                            let original = pc.attribute(att_id);
                            let mut portable = PointAttribute::default();
                            portable.init(
                                original.attribute_type(),
                                original.num_components(),
                                DataType::Uint32,
                                false,
                                num_points,
                            );
                            let mut att_decoder = SequentialIntegerAttributeDecoder::new();
                            att_decoder.init(self, att_id);
                            if !att_decoder.decode_values(pc, &point_ids, buffer, None, None, Some(&mut portable)) {
                                return Err(DracoError::DracoError("Failed to decode quantized portable values".to_string()));
                            }
                            pending_quant.push(PendingQuant {
                                att_id,
                                portable,
                                transform: AttributeQuantizationTransform::new(),
                            });
                        }
                        3 => {
                            let mut portable = PointAttribute::default();
                            portable.init(GeometryAttributeType::Generic, 2, DataType::Uint32, false, num_points);
                            let mut att_decoder = SequentialIntegerAttributeDecoder::new();
                            att_decoder.init(self, att_id);
                            if !att_decoder.decode_values(pc, &point_ids, buffer, None, None, Some(&mut portable)) {
                                return Err(DracoError::DracoError("Failed to decode normal portable values".to_string()));
                            }
                            pending_normals.push(PendingNormal { att_id, portable, quantization_bits: 0 });
                        }
                        0 => {
                            // Generic sequential values (raw), matching C++
                            // SequentialAttributeDecoder::DecodeValues().
                            let entry_size = pc.attribute(att_id).byte_stride() as usize;
                            if entry_size == 0 {
                                return Err(DracoError::DracoError(
                                    "Invalid point cloud attribute entry size".to_string(),
                                ));
                            }

                            let dst = pc.attribute_mut(att_id).buffer_mut().data_mut();
                            if dst.len() < entry_size * num_points {
                                return Err(DracoError::DracoError(
                                    "Point cloud attribute buffer too small".to_string(),
                                ));
                            }

                            for i in 0..num_points {
                                let start = i * entry_size;
                                let end = start + entry_size;
                                buffer.decode_bytes(&mut dst[start..end]).map_err(|_| {
                                    DracoError::DracoError(
                                        "Failed to decode raw point cloud attribute values".to_string(),
                                    )
                                })?;
                            }
                        }
                        _ => {
                            return Err(DracoError::DracoError(format!("Unsupported sequential decoder type: {}", decoder_type)));
                        }
                    }
                }

                for (local_i, &att_id) in att_ids.iter().enumerate() {
                    match decoder_types[local_i] {
                        2 => {
                            let idx = pending_quant.iter().position(|p| p.att_id == att_id).unwrap();
                            let original = pc.attribute(att_id);
                            if !pending_quant[idx].transform.decode_parameters(original, buffer) {
                                return Err(DracoError::DracoError("Failed to decode quantization parameters".to_string()));
                            }
                        }
                        3 => {
                            let idx = pending_normals.iter().position(|p| p.att_id == att_id).unwrap();
                            pending_normals[idx].quantization_bits = buffer.decode_u8()?;
                        }
                        _ => {}
                    }
                }

                for q in pending_quant {
                    let dst = pc.attribute_mut(q.att_id);
                    if !q.transform.inverse_transform_attribute(&q.portable, dst) {
                        return Err(DracoError::DracoError("Failed to dequantize attribute".to_string()));
                    }
                }
                for n in pending_normals {
                    let mut oct = AttributeOctahedronTransform::new(-1);
                    oct.set_parameters(n.quantization_bits as i32);
                    let dst = pc.attribute_mut(n.att_id);
                    if !oct.inverse_transform_attribute(&n.portable, dst) {
                        return Err(DracoError::DracoError("Failed to decode normals".to_string()));
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn get_geometry_type(&self) -> EncodedGeometryType {
        self.geometry_type
    }
}
