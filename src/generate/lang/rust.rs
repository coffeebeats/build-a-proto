//! Rust code generator.
//!
//! Generates Rust structs and enums from IR schemas with stub serialization methods.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::generate::{Generator, GeneratorError, GeneratorOutput};
use crate::ir::{self, NativeType, Variant};

/* -------------------------------------------------------------------------- */
/*                            Struct: RustGenerator                           */
/* -------------------------------------------------------------------------- */

/// Generates Rust code from IR schemas.
///
/// ## Output Structure
///
/// - One `.rs` file per package (e.g., `foo/bar.rs` for `foo.bar`)
/// - `mod.rs` files for directory structure
/// - Nested types are generated inline within their parent
///
/// ## Generated Code
///
/// - Structs for messages with public fields
/// - Enums for baproto enums
/// - `new()` constructor with default values
/// - Stub `encode()`/`decode()` methods (to be implemented)
pub struct RustGenerator {
    // Future: config options like derive macros, visibility, etc.
}

impl RustGenerator {
    /// Creates a new Rust generator.
    pub fn new() -> Self {
        Self {}
    }

    /// Converts a package path to a file path.
    ///
    /// Example: "foo.bar" -> "foo/bar.rs"
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

    /// Generates mod.rs files for the directory structure.
    fn generate_mod_files(&self, output: &mut GeneratorOutput) {
        // Collect all unique directory paths
        let mut dirs: HashSet<PathBuf> = HashSet::new();

        for path in output.files.keys() {
            let mut current = PathBuf::new();
            if let Some(parent) = path.parent() {
                for component in parent.components() {
                    current.push(component);
                    dirs.insert(current.clone());
                }
            }
        }

        // For each directory, collect child modules
        for dir in dirs {
            let mut children: Vec<String> = Vec::new();

            for path in output.files.keys() {
                if let Some(parent) = path.parent() {
                    if parent == dir {
                        if let Some(stem) = path.file_stem() {
                            if let Some(name) = stem.to_str() {
                                if name != "mod" {
                                    children.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }

            // Also check for subdirectories that have mod.rs
            for other_dir in output.files.keys() {
                if let Some(parent) = other_dir.parent() {
                    if let Some(grandparent) = parent.parent() {
                        if grandparent == dir {
                            if let Some(name) = parent.file_name() {
                                if let Some(name_str) = name.to_str() {
                                    if !children.contains(&name_str.to_string()) {
                                        children.push(name_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            children.sort();

            if !children.is_empty() {
                let mod_content = children
                    .iter()
                    .map(|name| format!("pub mod {};", name))
                    .collect::<Vec<_>>()
                    .join("\n");

                let mod_path = dir.join("mod.rs");
                output.add(mod_path, format!("{}\n", mod_content));
            }
        }

        // Generate root mod.rs if there are top-level modules
        let mut root_children: Vec<String> = Vec::new();
        for path in output.files.keys() {
            if path.components().count() == 1 {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        if name != "mod" {
                            root_children.push(name.to_string());
                        }
                    }
                }
            } else if path.components().count() == 2 {
                // Check for directories at root level
                if let Some(first) = path.components().next() {
                    if let Some(name) = first.as_os_str().to_str() {
                        if !root_children.contains(&name.to_string()) {
                            root_children.push(name.to_string());
                        }
                    }
                }
            }
        }

        root_children.sort();

        if !root_children.is_empty() {
            let mod_content = root_children
                .iter()
                .map(|name| format!("pub mod {};", name))
                .collect::<Vec<_>>()
                .join("\n");

            output.add("mod.rs", format!("{}\n", mod_content));
        }
    }

    /// Generates code for a package.
    fn generate_package(&self, package: &ir::Package) -> String {
        let mut code = String::new();

        // File header
        code.push_str(&format!(
            "//! Generated code for package `{}`.\n",
            package.path
        ));
        code.push_str("//!\n");
        code.push_str("//! This file was automatically generated by baproto.\n");
        code.push_str("//! Do not edit manually.\n\n");

        // Imports
        code.push_str("use std::collections::HashMap;\n");
        code.push_str("use std::io::{Read, Write};\n\n");

        // Generate enums first (they may be referenced by messages)
        for e in &package.enums {
            code.push_str(&self.generate_enum(e, 0, &package.path));
            code.push('\n');
        }

        // Generate messages
        for msg in &package.messages {
            code.push_str(&self.generate_message(msg, 0, &package.path));
            code.push('\n');
        }

        code
    }

    /// Generates code for a message.
    fn generate_message(&self, msg: &ir::Message, indent: usize, current_package: &str) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        // Doc comment
        if let Some(doc) = &msg.doc {
            for line in doc.lines() {
                code.push_str(&format!("{}/// {}\n", indent_str, line));
            }
        }

        // Struct definition
        code.push_str(&format!(
            "{}#[derive(Debug, Clone, PartialEq)]\n",
            indent_str
        ));
        code.push_str(&format!("{}pub struct {} {{\n", indent_str, msg.name));

        // Fields
        for field in &msg.fields {
            code.push_str(&self.generate_field(field, indent + 1, current_package));
        }

        code.push_str(&format!("{}}}\n\n", indent_str));

        // Impl block
        code.push_str(&format!("{}impl {} {{\n", indent_str, msg.name));

        // Constructor
        code.push_str(&format!("{}    /// Creates a new instance with default values.\n", indent_str));
        code.push_str(&format!("{}    pub fn new() -> Self {{\n", indent_str));
        code.push_str(&format!("{}        Self {{\n", indent_str));
        for field in &msg.fields {
            let default = self.default_value(&field.encoding.native);
            code.push_str(&format!("{}            {}: {},\n", indent_str, field.name, default));
        }
        code.push_str(&format!("{}        }}\n", indent_str));
        code.push_str(&format!("{}    }}\n\n", indent_str));

        // Encode stub
        code.push_str(&format!(
            "{}    /// Encodes this message to a writer.\n",
            indent_str
        ));
        code.push_str(&format!(
            "{}    pub fn encode(&self, _writer: &mut impl Write) -> std::io::Result<()> {{\n",
            indent_str
        ));
        code.push_str(&format!(
            "{}        todo!(\"serialization not yet implemented\")\n",
            indent_str
        ));
        code.push_str(&format!("{}    }}\n\n", indent_str));

        // Decode stub
        code.push_str(&format!(
            "{}    /// Decodes a message from a reader.\n",
            indent_str
        ));
        code.push_str(&format!(
            "{}    pub fn decode(_reader: &mut impl Read) -> std::io::Result<Self> {{\n",
            indent_str
        ));
        code.push_str(&format!(
            "{}        todo!(\"deserialization not yet implemented\")\n",
            indent_str
        ));
        code.push_str(&format!("{}    }}\n", indent_str));

        code.push_str(&format!("{}}}\n\n", indent_str));

        // Default impl
        code.push_str(&format!("{}impl Default for {} {{\n", indent_str, msg.name));
        code.push_str(&format!("{}    fn default() -> Self {{\n", indent_str));
        code.push_str(&format!("{}        Self::new()\n", indent_str));
        code.push_str(&format!("{}    }}\n", indent_str));
        code.push_str(&format!("{}}}\n", indent_str));

        // Generate nested enums
        for e in &msg.enums {
            code.push('\n');
            code.push_str(&self.generate_enum(e, indent, current_package));
        }

        // Generate nested messages
        for nested in &msg.messages {
            code.push('\n');
            code.push_str(&self.generate_message(nested, indent, current_package));
        }

        code
    }

    /// Generates code for a field.
    fn generate_field(&self, field: &ir::Field, indent: usize, current_package: &str) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        // Doc comment
        if let Some(doc) = &field.doc {
            for line in doc.lines() {
                code.push_str(&format!("{}/// {}\n", indent_str, line));
            }
        }

        let rust_type = self.native_type_to_rust(&field.encoding.native, current_package);
        code.push_str(&format!(
            "{}pub {}: {},\n",
            indent_str, field.name, rust_type
        ));

        code
    }

    /// Generates code for an enum.
    fn generate_enum(&self, e: &ir::Enum, indent: usize, current_package: &str) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        // Doc comment
        if let Some(doc) = &e.doc {
            for line in doc.lines() {
                code.push_str(&format!("{}/// {}\n", indent_str, line));
            }
        }

        // Enum definition
        code.push_str(&format!(
            "{}#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n",
            indent_str
        ));
        code.push_str(&format!("{}pub enum {} {{\n", indent_str, e.name));

        // Variants
        for variant in &e.variants {
            match variant {
                Variant::Unit { name, doc, .. } => {
                    if let Some(doc) = doc {
                        for line in doc.lines() {
                            code.push_str(&format!("{}    /// {}\n", indent_str, line));
                        }
                    }
                    code.push_str(&format!("{}    {},\n", indent_str, name));
                }
                Variant::Field {
                    name, field, doc, ..
                } => {
                    if let Some(doc) = doc {
                        for line in doc.lines() {
                            code.push_str(&format!("{}    /// {}\n", indent_str, line));
                        }
                    }
                    let rust_type = self.native_type_to_rust(&field.encoding.native, current_package);
                    code.push_str(&format!("{}    {}({}),\n", indent_str, name, rust_type));
                }
            }
        }

        code.push_str(&format!("{}}}\n", indent_str));

        code
    }

    /// Converts a NativeType to a Rust type string.
    fn native_type_to_rust(&self, native: &NativeType, current_package: &str) -> String {
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
                let inner = self.native_type_to_rust(&element.native, current_package);
                format!("Vec<{}>", inner)
            }
            NativeType::Map { key, value } => {
                let key_type = self.native_type_to_rust(&key.native, current_package);
                let value_type = self.native_type_to_rust(&value.native, current_package);
                format!("HashMap<{}, {}>", key_type, value_type)
            }
            NativeType::Message { descriptor } => self.descriptor_to_rust_type(descriptor, current_package),
            NativeType::Enum { descriptor } => self.descriptor_to_rust_type(descriptor, current_package),
        }
    }

    /// Converts a descriptor string to a Rust type name.
    ///
    /// Resolves cross-package references using `crate::` paths.
    /// Same-package references use simple names.
    fn descriptor_to_rust_type(&self, descriptor: &str, current_package: &str) -> String {
        // Parse descriptor: "com.example.Outer.Inner"
        // Package is everything up to the first uppercase letter (heuristic)
        // or we compare against current_package length
        let parts: Vec<&str> = descriptor.split('.').collect();

        // Check if this descriptor starts with the current package
        let current_pkg_parts: Vec<&str> = current_package.split('.').collect();
        let is_same_package = parts.len() > current_pkg_parts.len()
            && parts[..current_pkg_parts.len()] == current_pkg_parts[..];

        if is_same_package {
            // Same package - use simple name (nested types are inlined)
            parts.last().copied().unwrap_or(descriptor).to_string()
        } else {
            // Cross-package reference - use crate:: qualified path
            // Convert "com.example.User" to "crate::com::example::User"
            format!("crate::{}", parts.join("::"))
        }
    }

    /// Returns the default value for a native type.
    fn default_value(&self, native: &NativeType) -> String {
        match native {
            NativeType::Bool => "false".to_string(),
            NativeType::Int { .. } => "0".to_string(),
            NativeType::Float { .. } => "0.0".to_string(),
            NativeType::String => "String::new()".to_string(),
            NativeType::Bytes => "Vec::new()".to_string(),
            NativeType::Array { .. } => "Vec::new()".to_string(),
            NativeType::Map { .. } => "HashMap::new()".to_string(),
            NativeType::Message { descriptor } => {
                // For defaults, just use simple name (they're in the same file)
                let type_name = descriptor.split('.').last().unwrap_or(descriptor);
                format!("{}::new()", type_name)
            }
            NativeType::Enum { descriptor } => {
                // For defaults, just use simple name (they're in the same file)
                let type_name = descriptor.split('.').last().unwrap_or(descriptor);
                format!("{}::default()", type_name)
            }
        }
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for RustGenerator {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn extension(&self) -> &'static str {
        "rs"
    }

    fn generate(&self, schema: &ir::Schema) -> Result<GeneratorOutput, GeneratorError> {
        let mut output = GeneratorOutput::new();

        for package in &schema.packages {
            let path = self.package_to_path(&package.path);
            let content = self.generate_package(package);
            output.add(path, content);
        }

        // Generate mod.rs files for directory structure
        self.generate_mod_files(&mut output);

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Encoding, Field, Message, Package, Schema, WireFormat};

    fn make_encoding(native: NativeType) -> Encoding {
        Encoding {
            wire: WireFormat::Bits { count: 32 },
            native,
            transforms: vec![],
            padding_bits: None,
        }
    }

    #[test]
    fn test_package_to_path() {
        let generator = RustGenerator::new();
        assert_eq!(generator.package_to_path("foo"), PathBuf::from("foo.rs"));
        assert_eq!(generator.package_to_path("foo.bar"), PathBuf::from("foo/bar.rs"));
        assert_eq!(
            generator.package_to_path("foo.bar.baz"),
            PathBuf::from("foo/bar/baz.rs")
        );
    }

    #[test]
    fn test_native_type_to_rust() {
        let generator = RustGenerator::new();
        let pkg = "test";

        assert_eq!(generator.native_type_to_rust(&NativeType::Bool, pkg), "bool");
        assert_eq!(
            generator.native_type_to_rust(&NativeType::Int {
                bits: 32,
                signed: true
            }, pkg),
            "i32"
        );
        assert_eq!(
            generator.native_type_to_rust(&NativeType::Int {
                bits: 64,
                signed: false
            }, pkg),
            "u64"
        );
        assert_eq!(
            generator.native_type_to_rust(&NativeType::Float { bits: 32 }, pkg),
            "f32"
        );
        assert_eq!(generator.native_type_to_rust(&NativeType::String, pkg), "String");
        assert_eq!(generator.native_type_to_rust(&NativeType::Bytes, pkg), "Vec<u8>");
    }

    #[test]
    fn test_generate_simple_message() {
        let generator = RustGenerator::new();

        let schema = Schema {
            packages: vec![Package {
                path: "test".to_string(),
                messages: vec![Message {
                    descriptor: "test.Person".to_string(),
                    name: "Person".to_string(),
                    fields: vec![
                        Field {
                            name: "name".to_string(),
                            index: 1,
                            encoding: make_encoding(NativeType::String),
                            doc: None,
                        },
                        Field {
                            name: "age".to_string(),
                            index: 2,
                            encoding: make_encoding(NativeType::Int {
                                bits: 32,
                                signed: false,
                            }),
                            doc: None,
                        },
                    ],
                    messages: vec![],
                    enums: vec![],
                    doc: Some("A person.".to_string()),
                }],
                enums: vec![],
            }],
        };

        let output = generator.generate(&schema).unwrap();

        assert!(output.files.contains_key(&PathBuf::from("test.rs")));

        let content = &output.files[&PathBuf::from("test.rs")];
        assert!(content.contains("pub struct Person"));
        assert!(content.contains("pub name: String"));
        assert!(content.contains("pub age: u32"));
        assert!(content.contains("/// A person."));
        assert!(content.contains("fn new()"));
        assert!(content.contains("fn encode("));
        assert!(content.contains("fn decode("));
    }
}