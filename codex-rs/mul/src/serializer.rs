use crate::MulProgram;
use serde_json::Error;

pub fn serialize_program(program: &MulProgram) -> Result<String, Error> {
    serde_json::to_string(program)
}
