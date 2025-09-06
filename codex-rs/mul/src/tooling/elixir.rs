use super::ToolAdapter;
use crate::error::Result;

pub struct Adapter;

impl ToolAdapter for Adapter {
    fn build() -> Result<Vec<&'static str>> {
        Ok(vec!["mix", "compile"])
    }
    fn test() -> Result<Vec<&'static str>> {
        Ok(vec!["mix", "test"])
    }
    fn lint() -> Result<Vec<&'static str>> {
        Ok(vec!["mix", "format"])
    }
    fn run() -> Result<Vec<&'static str>> {
        Ok(vec!["mix", "run"])
    }
}

impl Adapter {
    pub fn build() -> Result<Vec<&'static str>> {
        <Self as ToolAdapter>::build()
    }
    pub fn test() -> Result<Vec<&'static str>> {
        <Self as ToolAdapter>::test()
    }
    pub fn lint() -> Result<Vec<&'static str>> {
        <Self as ToolAdapter>::lint()
    }
    pub fn run() -> Result<Vec<&'static str>> {
        <Self as ToolAdapter>::run()
    }
}
