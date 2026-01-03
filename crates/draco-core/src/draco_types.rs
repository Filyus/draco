#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    Invalid = 0,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Float32,
    Float64,
    Bool,
}

impl DataType {
    pub fn byte_length(&self) -> usize {
        match self {
            DataType::Invalid => 0,
            DataType::Int8 | DataType::Uint8 | DataType::Bool => 1,
            DataType::Int16 | DataType::Uint16 => 2,
            DataType::Int32 | DataType::Uint32 | DataType::Float32 => 4,
            DataType::Int64 | DataType::Uint64 | DataType::Float64 => 8,
        }
    }

    pub fn is_integral(&self) -> bool {
        match self {
            DataType::Float32 | DataType::Float64 | DataType::Invalid => false,
            _ => true,
        }
    }
}
