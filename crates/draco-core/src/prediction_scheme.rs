use crate::geometry_attribute::{GeometryAttributeType, PointAttribute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionSchemeMethod {
    Undefined = -2,
    None = -1,
    Difference = 0,
    MeshPredictionParallelogram = 1,
    MeshPredictionMultiParallelogram = 2,
    MeshPredictionTexCoordsDeprecated = 3,
    MeshPredictionConstrainedMultiParallelogram = 4,
    MeshPredictionTexCoordsPortable = 5,
    MeshPredictionGeometricNormal = 6,
}

impl TryFrom<u8> for PredictionSchemeMethod {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PredictionSchemeMethod::Difference),
            1 => Ok(PredictionSchemeMethod::MeshPredictionParallelogram),
            2 => Ok(PredictionSchemeMethod::MeshPredictionMultiParallelogram),
            3 => Ok(PredictionSchemeMethod::MeshPredictionTexCoordsDeprecated),
            4 => Ok(PredictionSchemeMethod::MeshPredictionConstrainedMultiParallelogram),
            5 => Ok(PredictionSchemeMethod::MeshPredictionTexCoordsPortable),
            6 => Ok(PredictionSchemeMethod::MeshPredictionGeometricNormal),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionSchemeTransformType {
    None = -1,
    Delta = 0,
    Wrap = 1,
    NormalOctahedron = 2,
    NormalOctahedronCanonicalized = 3,
    Parallelogram = 4,
    TexCoordsPortable = 5,
    GeometricNormal = 6,
    MultiParallelogram = 7,
    ConstrainedMultiParallelogram = 8,
}

impl TryFrom<u8> for PredictionSchemeTransformType {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PredictionSchemeTransformType::Delta),
            1 => Ok(PredictionSchemeTransformType::Wrap),
            2 => Ok(PredictionSchemeTransformType::NormalOctahedron),
            3 => Ok(PredictionSchemeTransformType::NormalOctahedronCanonicalized),
            4 => Ok(PredictionSchemeTransformType::Parallelogram),
            5 => Ok(PredictionSchemeTransformType::TexCoordsPortable),
            6 => Ok(PredictionSchemeTransformType::GeometricNormal),
            7 => Ok(PredictionSchemeTransformType::MultiParallelogram),
            8 => Ok(PredictionSchemeTransformType::ConstrainedMultiParallelogram),
            _ => Err(()),
        }
    }
}

pub trait PredictionScheme {
    fn get_prediction_method(&self) -> PredictionSchemeMethod;
    fn is_initialized(&self) -> bool;
    fn get_num_parent_attributes(&self) -> i32;
    fn get_parent_attribute_type(&self, i: i32) -> GeometryAttributeType;
    fn set_parent_attribute(&mut self, att: &PointAttribute) -> bool;
    fn get_transform_type(&self) -> PredictionSchemeTransformType;
    
    /// Returns true if the correction values are always positive (non-negative).
    /// This is used to determine whether to apply ZigZag encoding to corrections.
    /// For normal octahedron transforms, corrections are already in [0, max_value],
    /// so no ZigZag encoding is needed.
    fn are_corrections_positive(&self) -> bool {
        false
    }
}

pub trait PredictionSchemeEncodingTransform<DataType, CorrType> {
    fn init(&mut self, orig_data: &[DataType], size: usize, num_components: usize);
    fn compute_correction(
        &self,
        original_vals: &[DataType],
        predicted_vals: &[DataType],
        out_corr_vals: &mut [CorrType],
    );
    fn encode_transform_data(&mut self, buffer: &mut Vec<u8>) -> bool;
    fn get_type(&self) -> PredictionSchemeTransformType;
    
    /// Returns true if the corrections produced by this transform are always positive.
    fn are_corrections_positive(&self) -> bool {
        false
    }
}

pub trait PredictionSchemeDecodingTransform<DataType, CorrType> {
    fn init(&mut self, num_components: usize);
    fn compute_original_value(
        &self,
        predicted_vals: &[DataType],
        corr_vals: &[CorrType],
        out_original_vals: &mut [DataType],
    );
    fn decode_transform_data(&mut self, buffer: &mut crate::decoder_buffer::DecoderBuffer) -> bool;
    fn get_type(&self) -> PredictionSchemeTransformType;
    
    /// Returns true if the corrections are always positive (no ZigZag encoding needed).
    fn are_corrections_positive(&self) -> bool {
        false
    }
}

pub trait PredictionSchemeEncoder<DataType, CorrType>: PredictionScheme {
    fn compute_correction_values(
        &mut self,
        in_data: &[DataType],
        out_corr: &mut [CorrType],
        size: usize,
        num_components: usize,
        entry_to_point_id_map: Option<&[u32]>,
    ) -> bool;

    fn encode_prediction_data(&mut self, buffer: &mut Vec<u8>) -> bool;
}

pub trait PredictionSchemeDecoder<DataType, CorrType>: PredictionScheme {
    fn compute_original_values(
        &mut self,
        in_corr: &[CorrType],
        out_data: &mut [DataType],
        size: usize,
        num_components: usize,
        entry_to_point_id_map: Option<&[u32]>,
    ) -> bool;

    fn decode_prediction_data(&mut self, buffer: &mut crate::decoder_buffer::DecoderBuffer) -> bool;
}
