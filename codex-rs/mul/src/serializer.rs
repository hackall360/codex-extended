use crate::{MulProgram, error::Result};

pub fn serialize_program(program: &MulProgram) -> Result<String> {
    Ok(serde_json::to_string(program)?)
}
