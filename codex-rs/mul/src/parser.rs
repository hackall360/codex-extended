use crate::MulProgram;
use serde_json::Error;

pub fn parse_program(input: &str) -> Result<MulProgram, Error> {
    serde_json::from_str(input)
}
