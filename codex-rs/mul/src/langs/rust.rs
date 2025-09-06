use crate::{
    MulProgram,
    adapter::{JsonAdapter, MulAdapter},
    error::Result,
};

pub struct Adapter;

impl MulAdapter for Adapter {
    fn to_source(program: &MulProgram) -> Result<String> {
        JsonAdapter::to_source(program)
    }

    fn from_source(source: &str) -> Result<MulProgram> {
        JsonAdapter::from_source(source)
    }
}

impl Adapter {
    pub fn to_source(program: &MulProgram) -> Result<String> {
        <Self as MulAdapter>::to_source(program)
    }

    pub fn from_source(source: &str) -> Result<MulProgram> {
        <Self as MulAdapter>::from_source(source)
    }
}
