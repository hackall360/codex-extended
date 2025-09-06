use super::ToolAdapter;
use crate::error::Result;

pub struct Adapter;

impl ToolAdapter for Adapter {
    fn build() -> Result<Vec<&'static str>> {
        Ok(vec!["gradle", "build"])
    }
    fn test() -> Result<Vec<&'static str>> {
        Ok(vec!["gradle", "test"])
    }
    fn lint() -> Result<Vec<&'static str>> {
        Ok(vec!["codenarc"])
    }
    fn run() -> Result<Vec<&'static str>> {
        Ok(vec!["groovy"])
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
