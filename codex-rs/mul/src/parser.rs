use crate::{MulProgram, error::Result};

pub fn parse_program(input: &str) -> Result<MulProgram> {
    Ok(serde_json::from_str(input)?)
}
