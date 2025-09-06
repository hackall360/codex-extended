pub mod adapter;
pub mod error;
pub mod langs;
pub mod parser;
pub mod serializer;
pub mod tooling;

pub use adapter::{JsonAdapter, MulAdapter};
pub use error::MulError;
pub use tooling::{ToolAdapter, default::DefaultToolAdapter};

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
