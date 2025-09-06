use crate::{MulProgram, error::Result, parser, serializer};

/// Adapter for converting to and from [`MulProgram`] JSON representation.
pub struct MulAdapter;

impl MulAdapter {
    /// Serialize a [`MulProgram`] into a JSON string.
    pub fn to_mul(program: &MulProgram) -> Result<String> {
        serializer::serialize_program(program)
    }

    /// Parse a JSON string into a [`MulProgram`].
    pub fn from_mul(input: &str) -> Result<MulProgram> {
        parser::parse_program(input)
    }
}
