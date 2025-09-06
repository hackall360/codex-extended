pub mod parser;
pub mod serializer;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MulProgram {
    pub statements: Vec<MulStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MulStatement {
    /// Assign a value to a variable.
    Let { name: String, value: MulType },
    /// Multiply two values and store the result in a variable.
    Mul {
        name: String,
        left: MulType,
        right: MulType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MulType {
    Number(i64),
    Variable(String),
}
