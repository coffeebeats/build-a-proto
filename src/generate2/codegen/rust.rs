//! Rust code generator implementation.
//!
//! Implements the [`CodeGen`] trait to generate Rust structs, enums,
//! and stub encode/decode methods from IR schemas.

use std::path::PathBuf;

use anyhow::Result;

use super::CodeGen;
use crate::generate2::{CodeWriter, Writer};
use crate::ir::{self, NativeType, Variant};

/* -------------------------------------------------------------------------- */
/*                            Struct: RustCodeGen                             */
/* -------------------------------------------------------------------------- */

/// Rust code generator using the CodeGen visitor pattern.
pub struct RustCodeGen {
    cw: CodeWriter,
}

impl RustCodeGen {
    /// Creates a new Rust code generator.
    pub fn new() -> Self {
        Self {
            cw: CodeWriter::rust(),
        }
    }

    /// Converts a package path to a file path.
    ///
    /// Example: "foo.bar" → "foo/bar.rs"
    fn package_to_path(&self, package_path: &str) -> PathBuf {
        let parts: Vec<&str> = package_path.split('.').collect();
        let mut path = PathBuf::new();

        for part in &parts[..parts.len().saturating_sub(1)] {
            path.push(part);
        }

        if let Some(last) = parts.last() {
            path.push(format!("{}.rs", last));
        }

        path
    }

    /// Converts a descriptor string to a Rust type name.
    ///
    /// Resolves cross-package references using `crate::` paths.
    /// Same-package references use simple names.
    fn descriptor_to_rust_type(&self, descriptor: &str, current_package: &str) -> String {
        let parts: Vec<&str> = descriptor.split('.').collect();
        let current_pkg_parts: Vec<&str> = current_package.split('.').collect();

        let is_same_package = parts.len() > current_pkg_parts.len()
            && parts[..current_pkg_parts.len()] == current_pkg_parts[..];

        if is_same_package {
            // Same package - use simple name (nested types are inlined)
            parts.last().copied().unwrap_or(descriptor).to_string()
        } else {
            // Cross-package reference - use crate:: qualified path
            format!("crate::{}", parts.join("::"))
        }
    }
}

impl RustCodeGen {
    /// Converts an IR NativeType to a Rust type string.
    ///
    /// This is a helper method used by both field and variant generation.
    pub fn type_name(&self, native: &NativeType, current_package: &str) -> String {
        match native {
            NativeType::Bool => "bool".to_string(),
            NativeType::Int { bits, signed } => {
                let prefix = if *signed { "i" } else { "u" };
                format!("{}{}", prefix, bits)
            }
            NativeType::Float { bits } => format!("f{}", bits),
            NativeType::String => "String".to_string(),
            NativeType::Bytes => "Vec<u8>".to_string(),
            NativeType::Array { element } => {
                let inner = self.type_name(&element.native, current_package);
                format!("Vec<{}>", inner)
            }
            NativeType::Map { key, value } => {
                let key_type = self.type_name(&key.native, current_package);
                let value_type = self.type_name(&value.native, current_package);
                format!("HashMap<{}, {}>", key_type, value_type)
            }
            NativeType::Message { descriptor } => {
                self.descriptor_to_rust_type(descriptor, current_package)
            }
            NativeType::Enum { descriptor } => {
                self.descriptor_to_rust_type(descriptor, current_package)
            }
        }
    }

    /// Returns a default value expression for a NativeType.
    ///
    /// Used in constructors and initialization.
    pub fn default_value(&self, native: &NativeType) -> String {
        match native {
            NativeType::Bool => "false".to_string(),
            NativeType::Int { .. } => "0".to_string(),
            NativeType::Float { .. } => "0.0".to_string(),
            NativeType::String => "String::new()".to_string(),
            NativeType::Bytes => "Vec::new()".to_string(),
            NativeType::Array { .. } => "Vec::new()".to_string(),
            NativeType::Map { .. } => "HashMap::new()".to_string(),
            NativeType::Message { descriptor } => {
                let type_name = descriptor.split('.').last().unwrap_or(descriptor);
                format!("{}::new()", type_name)
            }
            NativeType::Enum { descriptor } => {
                let type_name = descriptor.split('.').last().unwrap_or(descriptor);
                format!("{}::default()", type_name)
            }
        }
    }
}

impl Default for RustCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: Writer> CodeGen<W> for RustCodeGen {
    fn code_writer(&self) -> &CodeWriter {
        &self.cw
    }

    fn code_writer_mut(&mut self) -> &mut CodeWriter {
        &mut self.cw
    }

    fn file_path(&self, pkg: &ir::Package) -> PathBuf {
        self.package_to_path(&pkg.path)
    }

    fn gen_package_begin(&mut self, pkg: &ir::Package, w: &mut W) -> Result<()> {
        let cw = &self.cw;

        // File header
        cw.writeln(
            w,
            &format!("//! Generated code for package `{}`.", pkg.path),
        )?;
        cw.writeln(w, "//!")?;
        cw.writeln(w, "//! This file was automatically generated by baproto.")?;
        cw.writeln(w, "//! Do not edit manually.")?;
        cw.blank_line(w)?;

        // Imports
        cw.writeln(w, "use std::collections::HashMap;")?;
        cw.writeln(w, "use std::io::{Read, Write};")?;
        cw.blank_line(w)?;

        Ok(())
    }

    fn gen_package_end(&mut self, _pkg: &ir::Package, _w: &mut W) -> Result<()> {
        // No footer needed for Rust files
        Ok(())
    }

    fn gen_message_begin(&mut self, msg: &ir::Message, w: &mut W) -> Result<()> {
        let cw = &mut self.cw;

        // Doc comment
        cw.comment_opt(w, msg.doc.as_deref())?;

        // Struct definition
        cw.writeln(w, "#[derive(Debug, Clone, PartialEq)]")?;
        cw.writeln(w, &format!("pub struct {} {{", msg.name))?;
        cw.indent();

        Ok(())
    }

    fn gen_message_end(&mut self, msg: &ir::Message, w: &mut W) -> Result<()> {
        // Compute defaults before borrowing cw to avoid borrow conflicts
        let defaults: Vec<_> = msg
            .fields
            .iter()
            .map(|f| (f.name.clone(), self.default_value(&f.encoding.native)))
            .collect();

        let cw = &mut self.cw;

        // Close struct
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.blank_line(w)?;

        // Impl block
        cw.writeln(w, &format!("impl {} {{", msg.name))?;
        cw.indent();

        // Constructor
        cw.comment(w, "Creates a new instance with default values.")?;
        cw.writeln(w, "pub fn new() -> Self {")?;
        cw.indent();
        cw.writeln(w, "Self {")?;
        cw.indent();

        for (name, default) in defaults {
            cw.writeln(w, &format!("{}: {},", name, default))?;
        }

        cw.outdent();
        cw.writeln(w, "}")?;
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.blank_line(w)?;

        // Encode stub
        cw.comment(w, "Encodes this message to a writer.")?;
        cw.writeln(
            w,
            "pub fn encode(&self, _writer: &mut [u8]) -> std::io::Result<()> {",
        )?;
        cw.indent();
        cw.writeln(w, "todo!(\"serialization not yet implemented\")")?;
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.blank_line(w)?;

        // Decode stub
        cw.comment(w, "Decodes a message from a reader.")?;
        cw.writeln(
            w,
            "pub fn decode(_reader: &[u8]) -> std::io::Result<Self> {",
        )?;
        cw.indent();
        cw.writeln(w, "todo!(\"deserialization not yet implemented\")")?;
        cw.outdent();
        cw.writeln(w, "}")?;

        // Close impl block
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.blank_line(w)?;

        // Default impl
        cw.writeln(w, &format!("impl Default for {} {{", msg.name))?;
        cw.indent();
        cw.writeln(w, "fn default() -> Self {")?;
        cw.indent();
        cw.writeln(w, "Self::new()")?;
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.blank_line(w)?;

        Ok(())
    }

    fn gen_enum_begin(&mut self, e: &ir::Enum, w: &mut W) -> Result<()> {
        let cw = &mut self.cw;

        // Doc comment
        cw.comment_opt(w, e.doc.as_deref())?;

        // Enum definition
        cw.writeln(w, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]")?;
        cw.writeln(w, &format!("pub enum {} {{", e.name))?;
        cw.indent();

        Ok(())
    }

    fn gen_enum_end(&mut self, _e: &ir::Enum, w: &mut W) -> Result<()> {
        let cw = &mut self.cw;

        // Close enum
        cw.outdent();
        cw.writeln(w, "}")?;
        cw.blank_line(w)?;

        Ok(())
    }

    fn gen_field(&mut self, field: &ir::Field, current_package: &str, w: &mut W) -> Result<()> {
        let cw = &self.cw;

        // Doc comment
        cw.comment_opt(w, field.doc.as_deref())?;

        // Field declaration
        let rust_type = self.type_name(&field.encoding.native, current_package);
        cw.writeln(w, &format!("pub {}: {},", field.name, rust_type))?;

        Ok(())
    }

    fn gen_variant(&mut self, variant: &Variant, current_package: &str, w: &mut W) -> Result<()> {
        let cw = &self.cw;

        match variant {
            Variant::Unit { name, doc, .. } => {
                cw.comment_opt(w, doc.as_deref())?;
                cw.writeln(w, &format!("{},", name))?;
            }
            Variant::Field {
                name, field, doc, ..
            } => {
                cw.comment_opt(w, doc.as_deref())?;
                let rust_type = self.type_name(&field.encoding.native, current_package);
                cw.writeln(w, &format!("{}({}),", name, rust_type))?;
            }
        }

        Ok(())
    }
}
