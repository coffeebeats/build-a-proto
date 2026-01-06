//! Generator trait for code generation plugins.

use crate::ir;

use super::{GeneratorError, GeneratorOutput};

/* -------------------------------------------------------------------------- */
/*                              Trait: Generator                              */
/* -------------------------------------------------------------------------- */

/// Trait for code generators that transform IR into source code.
///
/// Implementations of this trait can be either native (built-in Rust code)
/// or external (wrappers around binary plugins).
///
/// # Thread Safety
///
/// Generators are `Send + Sync` to allow for future parallel generation.
pub trait Generator: Send + Sync {
    /// Returns the unique identifier for this generator.
    ///
    /// This is used for logging and error messages.
    /// Examples: "rust", "gdscript", "cpp"
    fn name(&self) -> &'static str;

    /// Returns the file extension for generated files.
    ///
    /// Examples: "rs", "gd", "cpp"
    ///
    /// External generators may return an empty string since they control
    /// their own file naming.
    fn extension(&self) -> &'static str;

    /// Generates code from the IR schema.
    ///
    /// Returns a map of relative file paths to their contents.
    /// The caller is responsible for writing these files to disk.
    fn generate(&self, schema: &ir::Schema) -> Result<GeneratorOutput, GeneratorError>;
}