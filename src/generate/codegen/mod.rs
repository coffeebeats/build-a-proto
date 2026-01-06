//! Code generation framework using the visitor pattern.
//!
//! This module provides:
//! - [`CodeGen`] trait - Visitor-style hooks for generating code from IR
//! - [`generate_schema`] - Orchestrator function that drives the visitor
//!
//! The CodeGen trait allows language-specific generators to implement
//! only the methods they need, with sensible defaults for others.

pub mod rust;

use std::path::PathBuf;

use anyhow::Result;

use crate::generate::{CodeWriter, GeneratorOutput, Writer};
use crate::ir;

/* -------------------------------------------------------------------------- */
/*                              Trait: CodeGen                                */
/* -------------------------------------------------------------------------- */

/// Visitor-style code generator that transforms IR into source code.
///
/// Implementors provide language-specific code generation logic by implementing
/// the visitor hooks. The [`generate_schema`] orchestrator drives the traversal.
///
/// # Design Pattern
///
/// The CodeGen trait follows the visitor pattern with begin/end hooks for each
/// IR construct. This enables:
/// - **Structured traversal**: Guaranteed consistent field/variant ordering
/// - **Composability**: Shared type conversion logic across messages and enums
/// - **Ser/de support**: `gen_field_encode/decode` hooks for serialization
///
/// # Example
///
/// ```rust,ignore
/// struct RustCodeGen {
///     cw: CodeWriter,
/// }
///
/// impl<W: Writer> CodeGen<W> for RustCodeGen {
///     fn code_writer(&self) -> &CodeWriter {
///         &self.cw
///     }
///
///     fn gen_message_begin(&mut self, msg: &ir::Message, w: &mut W) -> Result<()> {
///         let cw = self.code_writer();
///         cw.comment_opt(w, msg.doc.as_deref())?;
///         cw.writeln(w, "#[derive(Debug, Clone, PartialEq)]")?;
///         cw.writeln(w, &format!("pub struct {} {{", msg.name))?;
///         Ok(())
///     }
///     // ... other methods
/// }
/// ```
pub trait CodeGen<W: Writer> {
    // ========================= Configuration =========================

    /// Returns a reference to the CodeWriter for this generator.
    fn code_writer(&self) -> &CodeWriter;

    /// Returns a mutable reference to the CodeWriter for this generator.
    fn code_writer_mut(&mut self) -> &mut CodeWriter;

    /// Returns the file path for a given package.
    ///
    /// # Example
    ///
    /// For Rust: "foo.bar" → "foo/bar.rs"
    /// For GDScript: "foo.bar" → "foo/bar.gd"
    fn file_path(&self, pkg: &ir::Package) -> PathBuf;

    // ========================= Package Generation =========================

    /// Called at the start of package generation.
    ///
    /// Use this for file headers, imports, etc.
    fn gen_package_begin(&mut self, pkg: &ir::Package, w: &mut W) -> Result<()> {
        let _ = (pkg, w);
        Ok(())
    }

    /// Called at the end of package generation.
    ///
    /// Use this for file footers, closing braces, etc.
    fn gen_package_end(&mut self, pkg: &ir::Package, w: &mut W) -> Result<()> {
        let _ = (pkg, w);
        Ok(())
    }

    // ========================= Message Generation =========================

    /// Called at the start of message generation.
    ///
    /// Generate the struct/class header, opening brace, etc.
    fn gen_message_begin(&mut self, msg: &ir::Message, w: &mut W) -> Result<()>;

    /// Called at the end of message generation (after fields and nested types).
    ///
    /// Generate the closing brace, impl blocks, encode/decode methods, etc.
    fn gen_message_end(&mut self, msg: &ir::Message, w: &mut W) -> Result<()>;

    // ========================= Enum Generation =========================

    /// Called at the start of enum generation.
    ///
    /// Generate the enum header, opening brace, etc.
    fn gen_enum_begin(&mut self, e: &ir::Enum, w: &mut W) -> Result<()>;

    /// Called at the end of enum generation (after variants).
    ///
    /// Generate the closing brace, impl blocks, etc.
    fn gen_enum_end(&mut self, e: &ir::Enum, w: &mut W) -> Result<()>;

    // ========================= Field/Variant Generation =========================

    /// Generates a message field declaration.
    ///
    /// Called for each field in a message. Uses `type_name` for type conversion.
    fn gen_field(&mut self, field: &ir::Field, current_package: &str, w: &mut W) -> Result<()>;

    /// Generates an enum variant.
    ///
    /// Called for each variant in an enum. Handles both Unit and Field variants.
    fn gen_variant(&mut self, variant: &ir::Variant, current_package: &str, w: &mut W)
        -> Result<()>;

    // ========================= Encoding/Decoding (Optional) =========================

    /// Generates code to encode a field.
    ///
    /// Default implementation is a no-op. Override to generate serialization code.
    fn gen_field_encode(&mut self, _field: &ir::Field, _w: &mut W) -> Result<()> {
        Ok(())
    }

    /// Generates code to decode a field.
    ///
    /// Default implementation is a no-op. Override to generate deserialization code.
    fn gen_field_decode(&mut self, _field: &ir::Field, _w: &mut W) -> Result<()> {
        Ok(())
    }
}

/* -------------------------------------------------------------------------- */
/*                           Function: generate_schema                        */
/* -------------------------------------------------------------------------- */

/// Orchestrates code generation by driving a CodeGen visitor over an IR schema.
///
/// This function:
/// 1. Iterates through all packages in the schema
/// 2. For each package, creates a Writer and calls visitor hooks
/// 3. Recursively handles nested messages and enums
/// 4. Returns a GeneratorOutput with all generated files
///
/// # Example
///
/// ```rust,ignore
/// let mut gen = RustCodeGen::new();
/// let output = generate_schema::<StringWriter, _>(&schema, &mut gen)?;
/// ```
pub fn generate_schema<W, G>(schema: &ir::Schema, generator: &mut G) -> Result<GeneratorOutput>
where
    W: Writer,
    G: CodeGen<W>,
{
    let mut output = GeneratorOutput::new();

    for pkg in &schema.packages {
        let mut w = W::default();

        generator.gen_package_begin(pkg, &mut w)?;

        // Generate top-level enums first (may be referenced by messages)
        for e in &pkg.enums {
            gen_enum(e, &pkg.path, generator, &mut w)?;
        }

        // Generate top-level messages
        for msg in &pkg.messages {
            gen_message(msg, &pkg.path, generator, &mut w)?;
        }

        generator.gen_package_end(pkg, &mut w)?;

        let path = generator.file_path(pkg);
        output.add(path, w.finish()?);
    }

    Ok(output)
}

/* -------------------------------------------------------------------------- */
/*                          Helper Functions                                  */
/* -------------------------------------------------------------------------- */

/// Recursively generates code for a message and its nested types.
fn gen_message<W, G>(msg: &ir::Message, current_package: &str, generator: &mut G, w: &mut W) -> Result<()>
where
    W: Writer,
    G: CodeGen<W>,
{
    // Generate nested enums first (before the struct)
    for e in &msg.enums {
        gen_enum(e, current_package, generator, w)?;
    }

    // Generate nested messages (before the struct)
    for nested in &msg.messages {
        gen_message(nested, current_package, generator, w)?;
    }

    // Generate the struct definition
    generator.gen_message_begin(msg, w)?;

    // Generate fields
    for field in &msg.fields {
        generator.gen_field(field, current_package, w)?;
    }

    generator.gen_message_end(msg, w)?;

    Ok(())
}

/// Generates code for an enum.
fn gen_enum<W, G>(e: &ir::Enum, current_package: &str, generator: &mut G, w: &mut W) -> Result<()>
where
    W: Writer,
    G: CodeGen<W>,
{
    generator.gen_enum_begin(e, w)?;

    // Generate variants
    for variant in &e.variants {
        generator.gen_variant(variant, current_package, w)?;
    }

    generator.gen_enum_end(e, w)?;

    Ok(())
}
