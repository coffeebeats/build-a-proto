//! Rust code generator.
//!
//! Generates Rust structs and enums from IR schemas with stub serialization methods.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::generate::codegen::rust::RustCodeGen;
use crate::generate::{Generator, GeneratorError, GeneratorOutput, StringWriter, generate_schema};
use crate::ir;

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

    /// Generates mod.rs files for the directory structure.
    ///
    /// Creates mod.rs files that re-export submodules based on the file structure.
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
                output.add(mod_path, format!("{}", mod_content));
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

            output.add("mod.rs", format!("{}", mod_content));
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
        // Use the new RustCodeGen visitor pattern internally
        let mut codegen = RustCodeGen::new();
        let mut output = generate_schema::<StringWriter, _>(schema, &mut codegen)
            .map_err(|e| GeneratorError::Generation(e.to_string()))?;

        // Generate mod.rs files for directory structure
        self.generate_mod_files(&mut output);

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::codegen::rust::RustCodeGen;
    use crate::ir::{Encoding, Field, Message, NativeType, Package, Schema, WireFormat};

    fn make_encoding(native: NativeType) -> Encoding {
        Encoding {
            wire: WireFormat::Bits { count: 32 },
            native,
            transforms: vec![],
            padding_bits: None,
        }
    }

    #[test]
    fn test_generate_simple_message() {
        let generator = RustGenerator::new();

        // File header
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

    #[test]
    fn test_type_name_conversion() {
        let codegen = RustCodeGen::new();
        let pkg = "test";

        assert_eq!(codegen.type_name(&NativeType::Bool, pkg), "bool");
        assert_eq!(
            codegen.type_name(
                &NativeType::Int {
                    bits: 32,
                    signed: true
                },
                pkg
            ),
            "i32"
        );
        assert_eq!(
            codegen.type_name(
                &NativeType::Int {
                    bits: 64,
                    signed: false
                },
                pkg
            ),
            "u64"
        );
        assert_eq!(
            codegen.type_name(&NativeType::Float { bits: 32 }, pkg),
            "f32"
        );
        assert_eq!(codegen.type_name(&NativeType::String, pkg), "String");
        assert_eq!(codegen.type_name(&NativeType::Bytes, pkg), "Vec<u8>");
    }
}
