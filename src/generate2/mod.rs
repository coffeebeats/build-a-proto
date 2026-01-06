//! Code generation framework for baproto schemas.
//!
//! This module provides a plugin architecture for code generators, supporting both:
//! - **Native generators** - Built-in Rust implementations of the `Generator` trait
//! - **External generators** - Binary plugins invoked via stdin/stdout JSON protocol
//!
//! ## Protocol for External Generators
//!
//! External generators receive JSON-serialized `ir::Schema` on stdin and must output
//! a JSON object with a "files" key mapping relative paths to file contents:
//!
//! ```json
//! {"files": {"foo/bar.py": "# generated code..."}}
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod code;
pub mod codegen;
mod external;
mod generator;
pub mod lang;
mod writer;

pub use code::{CodeWriter, StringWriter, Writer};
pub use codegen::{generate_schema, CodeGen};
pub use external::ExternalGenerator;
pub use generator::Generator;
pub use writer::FileWriter;

/* -------------------------------------------------------------------------- */
/*                            Struct: GeneratorOutput                         */
/* -------------------------------------------------------------------------- */

/// Output from any generator (native or external).
///
/// Contains a map of relative file paths to their generated contents.
/// The paths are relative to the output directory specified by the user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneratorOutput {
    /// Map of relative file paths to their generated contents.
    pub files: HashMap<PathBuf, String>,
}

impl GeneratorOutput {
    /// Creates a new empty generator output.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to the output.
    pub fn add(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }
}

/* -------------------------------------------------------------------------- */
/*                             Enum: GeneratorError                           */
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
}