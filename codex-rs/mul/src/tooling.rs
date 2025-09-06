use crate::error::Result;

/// Trait defining tooling operations for a given language.
pub trait ToolAdapter {
    /// Build the generated project or source code.
    fn build() -> Result<()>;

    /// Run tests for the generated project.
    fn test() -> Result<()>;

    /// Lint the generated source code.
    fn lint() -> Result<()>;

    /// Execute the generated program.
    fn run() -> Result<()>;
}

pub mod default;
