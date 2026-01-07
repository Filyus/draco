use crate::encoder_options::EncoderOptions;
use crate::point_cloud_encoder::GeometryEncoder;
use crate::prediction_scheme::PredictionSchemeMethod;
use crate::geometry_attribute::GeometryAttributeType;
use crate::compression_config::EncodedGeometryType;

pub fn select_prediction_method(
    att_id: i32,
    options: &EncoderOptions,
    encoder: &dyn GeometryEncoder,
) -> PredictionSchemeMethod {
    if options.get_encoding_speed() >= 10 {
        return PredictionSchemeMethod::Difference;
    }

    if encoder.get_geometry_type() == EncodedGeometryType::TriangularMesh {
        // CRITICAL: In C++, MeshSequentialEncoder does NOT override GetCornerTable(),
        // so it returns nullptr. This means mesh prediction schemes (Parallelogram, etc.)
        // are never created for sequential encoding - the factory falls back to Delta.
        // We must match this behavior for C++ decoder compatibility.
        let encoding_method = encoder.get_encoding_method();
        if encoding_method == Some(0) {
            // Sequential encoding (method = 0) - use Delta prediction only
            return PredictionSchemeMethod::Difference;
        }
        let att_quant = options.get_attribute_int(att_id, "quantization_bits", -1);
        let pc = encoder.point_cloud().unwrap(); // Should be safe if called from encoder
        let att = pc.attribute(att_id);
        
        if att_quant != -1 && att.attribute_type() == GeometryAttributeType::TexCoord && att.num_components() == 2 {
            let pos_att = pc.named_attribute(GeometryAttributeType::Position);
            let mut is_pos_att_valid = false;
            
            if let Some(pos_att) = pos_att {
                if pos_att.data_type().is_integral() {
                    is_pos_att_valid = true;
                } else {
                    let pos_att_id = pc.named_attribute_id(GeometryAttributeType::Position);
                    let pos_quant = options.get_attribute_int(pos_att_id, "quantization_bits", -1);
                    if pos_quant > 0 && pos_quant <= 21 && 2 * pos_quant + att_quant < 64 {
                        is_pos_att_valid = true;
                    }
                }
            }
            
            if is_pos_att_valid && options.get_encoding_speed() < 4 {
                return PredictionSchemeMethod::MeshPredictionTexCoordsPortable;
            }
        }

        // Note: Do not apply any non-Draco fallbacks here. Prediction selection must
        // follow the Draco C++ rules; any correctness issues should be fixed in the
        // corresponding prediction scheme implementation.
        
        if att.attribute_type() == GeometryAttributeType::Normal {
            // TODO: Normal prediction support
            return PredictionSchemeMethod::Difference;
        }
        
        if options.get_encoding_speed() >= 8 {
            return PredictionSchemeMethod::Difference;
        }
        
        if options.get_encoding_speed() >= 2 || pc.num_points() < 40 {
            return PredictionSchemeMethod::MeshPredictionParallelogram;
        }
        
        return PredictionSchemeMethod::MeshPredictionConstrainedMultiParallelogram;
    }
    
    // Point Cloud prediction
    PredictionSchemeMethod::Difference
}
