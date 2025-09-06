use crate::error::Result;

/// Trait defining tooling operations for a given language.
pub trait ToolAdapter {
    /// Build the generated project or source code.
    fn build() -> Result<Vec<&'static str>>;

    /// Run tests for the generated project.
    fn test() -> Result<Vec<&'static str>>;

    /// Lint the generated source code.
    fn lint() -> Result<Vec<&'static str>>;

    /// Execute the generated program.
    fn run() -> Result<Vec<&'static str>>;
}

pub mod ada;
pub mod bash;
pub mod c;
pub mod clojure;
pub mod cpp;
pub mod csharp;
pub mod dart;
pub mod default;
pub mod elixir;
pub mod erlang;
pub mod fortran;
pub mod fsharp;
pub mod go;
pub mod groovy;
pub mod haskell;
pub mod java;
pub mod javascript;
pub mod julia;
pub mod kotlin;
pub mod lua;
pub mod matlab;
pub mod objectivec;
pub mod ocaml;
pub mod perl;
pub mod php;
pub mod powershell;
pub mod python;
pub mod r;
pub mod ruby;
pub mod rust;
pub mod scala;
pub mod sql;
pub mod swift;
pub mod typescript;
