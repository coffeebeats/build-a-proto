use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use crate::ir;

/* ----------------------------- Mod: External ------------------------------ */

mod external;
pub use external::*;

/* -------------------------------- Mod: Rust ------------------------------- */

mod rust;
pub use rust::*;

/* -------------------------------------------------------------------------- */
/*                            Struct: GeneratorOutput                         */
/* -------------------------------------------------------------------------- */

/// Output from any generator (native or external).
///
/// Contains a map of relative file paths to their generated contents.
/// The paths are relative to the output directory specified by the user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneratorOutput {
    /// `files` is a map of relative file paths to their generated contents.
    pub files: HashMap<PathBuf, String>,
}

/* -------------------------- Impl: GeneratorOutput ------------------------- */

impl GeneratorOutput {
    /// Adds a file to the output.
    pub fn add(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }
}

/* -------------------------------------------------------------------------- */
/*                            Enum: GeneratorError                            */
/* -------------------------------------------------------------------------- */

/// Errors that can occur during code generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Failed to serialize IR to JSON for external generator.
    #[error("serialization failed: {0}")]
    Serialization(String),

    /// External generator process failed.
    #[error("external process failed: {0}")]
    Process(String),

    /// External generator produced invalid output.
    #[error("invalid output from generator: {0}")]
    InvalidOutput(String),

    /// General generation failure.
    #[error("generation failed: {0}")]
    Generation(String),

    /// I/O error during generation.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error from anyhow.
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/* -------------------------------------------------------------------------- */
/*                              Trait: Generator                              */
/* -------------------------------------------------------------------------- */

/// Trait for code generators that transform IR into source code.
///
/// This is the public-facing API used by the CLI. It differs from the internal
/// visitor-pattern generator which provides more fine-grained control.
///
/// Implementations can be either native (built-in Rust code) or external
/// (wrappers around binary plugins).
pub trait Generator: Send + Sync {
    /// Returns the unique identifier for this generator.
    ///
    /// This is used for logging and error messages.
    /// Examples: "rust", "gdscript", "cpp"
    #[allow(unused)]
    fn name(&self) -> &str;

    /// Generates code from the IR schema.
    ///
    /// Returns a map of relative file paths to their contents.
    /// The caller is responsible for writing these files to disk.
    fn generate(&self, schema: &ir::Schema) -> Result<GeneratorOutput, GeneratorError>;
}
