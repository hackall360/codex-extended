use super::ToolAdapter;
use crate::error::Result;

/// No-op implementation of [`ToolAdapter`].
pub struct DefaultToolAdapter;

impl ToolAdapter for DefaultToolAdapter {
    fn build() -> Result<()> {
        Ok(())
    }

    fn test() -> Result<()> {
        Ok(())
    }

    fn lint() -> Result<()> {
        Ok(())
    }

    fn run() -> Result<()> {
        Ok(())
    }
}

impl DefaultToolAdapter {
    /// Convenience wrapper around [`ToolAdapter::build`].
    pub fn build() -> Result<()> {
        <Self as ToolAdapter>::build()
    }

    /// Convenience wrapper around [`ToolAdapter::test`].
    pub fn test() -> Result<()> {
        <Self as ToolAdapter>::test()
    }

    /// Convenience wrapper around [`ToolAdapter::lint`].
    pub fn lint() -> Result<()> {
        <Self as ToolAdapter>::lint()
    }

    /// Convenience wrapper around [`ToolAdapter::run`].
    pub fn run() -> Result<()> {
        <Self as ToolAdapter>::run()
    }
}
