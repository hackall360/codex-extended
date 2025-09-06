use super::ToolAdapter;
use crate::error::Result;

pub struct Adapter;

impl ToolAdapter for Adapter {
    fn build() -> Result<Vec<&'static str>> {
        Ok(vec!["go", "build"])
    }
    fn test() -> Result<Vec<&'static str>> {
        Ok(vec!["go", "test"])
    }
    fn lint() -> Result<Vec<&'static str>> {
        Ok(vec!["golangci-lint", "run"])
    }
    fn run() -> Result<Vec<&'static str>> {
        Ok(vec!["go", "run"])
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
