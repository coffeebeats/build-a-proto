use std::collections::HashMap;

use crate::core::{Descriptor, DescriptorBuilder};
use crate::core::PackageName;
use crate::core::SchemaImport;

/* -------------------------------------------------------------------------- */
/*                               Struct: Symbols                              */
/* -------------------------------------------------------------------------- */

/// `Symbols` is a symbol table tracking type existence and module metadata
/// during compilation.
#[derive(Clone, Default)]
pub struct Symbols {
    descriptors: HashMap<String, Descriptor>,
    modules: HashMap<SchemaImport, ModuleMetadata>,
    types: HashMap<Descriptor, TypeKind>,
}

/* ----------------------------- Impl: Symbols ------------------------------ */

impl Symbols {
    /// Checks if a type descriptor exists in the symbol table.
    pub fn contains(&self, desc: &Descriptor) -> bool {
        self.types.contains_key(desc)
    }

    /// Looks up the type data for the specified descriptor.
    pub fn get_type(&self, desc: &Descriptor) -> Option<TypeKind> {
        self.types.get(desc).copied()
    }

    /// Registers type data by its descriptor.
    pub fn insert_type(&mut self, desc: Descriptor, kind: TypeKind) {
        // Build the fully qualified name for fast lookup
        let fqn = desc.to_string();
        self.types.insert(desc.clone(), kind);
        self.descriptors.insert(fqn, desc);
    }

    /// Registers module metadata by its import path.
    pub fn insert_module(&mut self, import: SchemaImport, meta: ModuleMetadata) {
        self.modules.insert(import, meta);
    }

    /// Resolves a type reference from the given scope.
    ///
    /// For absolute references (starting with `.`), searches from the root.
    /// For relative references, searches outward from the current scope.
    pub fn resolve(
        &self,
        package: &PackageName,
        scope: &[String],
        reference: &str,
    ) -> Option<Descriptor> {
        // Parse reference into components
        let ref_parts: Vec<&str> = reference.split('.').collect();

        // Handle absolute references (starting with .)
        if ref_parts.first().map(|s| s.is_empty()).unwrap_or(false) {
            // Absolute reference - build descriptor from package root
            let path = ref_parts[1..ref_parts.len().saturating_sub(1)]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let name = ref_parts.last().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });

            let mut builder = DescriptorBuilder::default();
            builder.package(package.clone()).path(path);

            if let Some(n) = name {
                builder.name(n);
            }

            let descriptor = builder.build().ok()?;

            return if self.contains(&descriptor) {
                Some(descriptor)
            } else {
                None
            };
        }

        // Relative reference - search outward from current scope
        // Try current scope first, then progressively remove nesting
        for depth in (0..=scope.len()).rev() {
            let mut path = scope[..depth].to_vec();
            path.extend(
                ref_parts[..ref_parts.len().saturating_sub(1)]
                    .iter()
                    .map(|s| s.to_string()),
            );

            let name = ref_parts.last().map(|s| s.to_string());

            let descriptor = DescriptorBuilder::default()
                .package(package.clone())
                .path(path)
                .name(name.unwrap_or_default())
                .build()
                .ok()?;

            if self.contains(&descriptor) {
                return Some(descriptor);
            }
        }

        None
    }
}

/* -------------------------------------------------------------------------- */
/*                          Struct: ModuleMetadata                            */
/* -------------------------------------------------------------------------- */

/// Metadata about a compiled module (single .baproto file).
///
/// This tracks:
/// - The package namespace this module contributes to
/// - Schema imports (dependencies)
/// - Types defined in this module
#[derive(Clone, Debug)]
pub struct ModuleMetadata {
    pub package: PackageName,
    pub deps: Vec<SchemaImport>,
    pub types: Vec<Descriptor>,
}

/* -------------------------------------------------------------------------- */
/*                              Enum: TypeKind                                */
/* -------------------------------------------------------------------------- */

/// The kind of a registered type (`Message` or `Enum`).
///
/// This is a lightweight representation used during validation before full type
/// information is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Message,
    Enum,
}
