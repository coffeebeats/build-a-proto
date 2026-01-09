//! Rust code generator wrapper for the public API.
//!
//! This wraps the visitor-pattern Rust generator to provide a simple
//! Generator trait implementation for use by the CLI.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::generate::api::{Generator, GeneratorError, GeneratorOutput};
use crate::generate::StringWriter;
use crate::ir;

use super::rust;

/* -------------------------------------------------------------------------- */
/*                            Struct: RustGenerator                           */
/* -------------------------------------------------------------------------- */

/// Generates Rust code from IR schemas.
///
/// This is a wrapper around the visitor-pattern generator that provides
/// the simple Generator trait API expected by the CLI.
pub struct RustGenerator;

impl RustGenerator {
    /// Creates a new Rust generator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for RustGenerator {
    fn name(&self) -> &str {
        "rust"
    }

    fn generate(&self, schema: &ir::Schema) -> Result<GeneratorOutput, GeneratorError> {
        use crate::generate::{generator::Generator as GenTrait, Writer};

        let mut rust_gen = rust::rust::<StringWriter>();
        let mut writers = HashMap::<PathBuf, StringWriter>::new();

        // Create writers for each package
        for pkg in &schema.packages {
            let path = rust_gen.configure_writer(std::path::Path::new("."), pkg)?;
            let mut w = StringWriter::default();
            w.open(&path)?;
            writers.insert(path, w);
        }

        rust_gen.gen_begin(schema, writers.iter_mut().collect())?;

        for pkg in &schema.packages {
            let path = rust_gen.configure_writer(std::path::Path::new("."), pkg)?;
            let w = writers
                .get_mut(&path)
                .ok_or_else(|| GeneratorError::Generation(format!("missing writer for package: {}", pkg.path)))?;

            rust_gen.gen_pkg(schema, pkg, w)?;
        }

        rust_gen.gen_end(schema, writers.iter_mut().collect())?;

        // Collect content from writers
        let mut result = GeneratorOutput::new();
        for (path, w) in writers {
            // Strip leading "./" if present
            let relative_path = path.strip_prefix(".").unwrap_or(&path);
            result.add(relative_path.to_path_buf(), w.into_content());
        }

        Ok(result)
    }
}

