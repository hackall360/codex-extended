use crate::{MulProgram, error::Result, parser, serializer};

/// Trait for converting between language-specific source code and the [`MulProgram`] representation.
pub trait MulAdapter {
    /// Emit source code from a [`MulProgram`].
    fn to_source(program: &MulProgram) -> Result<String>;

    /// Parse source code into a [`MulProgram`].
    fn from_source(source: &str) -> Result<MulProgram>;
}

/// Adapter for the canonical JSON representation of MUL programs.
pub struct JsonAdapter;

impl MulAdapter for JsonAdapter {
    fn to_source(program: &MulProgram) -> Result<String> {
        serializer::serialize_program(program)
    }

    fn from_source(source: &str) -> Result<MulProgram> {
        parser::parse_program(source)
    }
}

impl JsonAdapter {
    /// Convenience wrapper around [`MulAdapter::to_source`].
    pub fn to_source(program: &MulProgram) -> Result<String> {
        <Self as MulAdapter>::to_source(program)
    }

    /// Convenience wrapper around [`MulAdapter::from_source`].
    pub fn from_source(source: &str) -> Result<MulProgram> {
        <Self as MulAdapter>::from_source(source)
    }
}
