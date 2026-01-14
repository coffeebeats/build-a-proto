use std::collections::HashMap;

use crate::ast;
use crate::core::Descriptor;
use crate::core::PackageName;
use crate::core::SchemaImport;
use crate::ir::lower;

/* -------------------------------------------------------------------------- */
/*                               Struct: Symbols                              */
/* -------------------------------------------------------------------------- */

/// `Symbols` is a symbol table tracking type existence and module metadata
/// during compilation.
#[allow(dead_code)]
#[derive(Default)]
pub struct Symbols {
    descriptors: HashMap<String, Descriptor>,
    modules: HashMap<SchemaImport, ModuleMetadata>,
    types: HashMap<Descriptor, TypeKind>,
}

/* ----------------------------- Impl: Symbols ------------------------------ */

impl Symbols {
    /// `contains` checks if a type descriptor exists in the symbol table.
    #[allow(dead_code)]
    pub fn contains(&self, desc: &Descriptor) -> bool {
        self.types.contains_key(desc)
    }

    /// `get_type` looks up the type data for the specified descriptor.
    #[allow(dead_code)]
    pub fn get_type(&self, desc: &Descriptor) -> Option<TypeKind> {
        self.types.get(desc).copied()
    }

    /// `insert_type` registers type data by its descriptor.
    #[allow(dead_code)]
    pub fn insert_type(&mut self, desc: Descriptor, kind: TypeKind) {
        // Build the fully qualified name for fast lookup
        let fqn = desc.to_string();
        self.types.insert(desc.clone(), kind);
        self.descriptors.insert(fqn, desc);
    }

    /// `insert_module` registers module metadata by its import path.
    #[allow(dead_code)]
    pub fn insert_module(&mut self, import: SchemaImport, meta: ModuleMetadata) {
        self.modules.insert(import, meta);
    }

    /// `find` resolves a reference to a registered type, if one exists.
    #[allow(dead_code)]
    pub fn find(
        &self,
        _scope: &Descriptor,
        _reference: &ast::Reference,
    ) -> Option<(Descriptor, TypeKind)> {
        todo!()
    }
}

/* --------------------------- Impl: TypeResolver --------------------------- */

impl lower::TypeResolver for Symbols {
    fn resolve(
        &self,
        _scope: &[String],
        _reference: &[String],
        _is_absolute: bool,
    ) -> Option<(String, lower::TypeKind)> {
        todo!()
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
#[allow(dead_code)]
pub struct ModuleMetadata {
    /// Package namespace
    pub package: PackageName,
    /// Schema file dependencies (include statements)
    pub deps: Vec<SchemaImport>,
    /// Type descriptors defined in this module
    pub types: Vec<Descriptor>,
}

/* -------------------------------------------------------------------------- */
/*                              Enum: TypeKind                                */
/* -------------------------------------------------------------------------- */

/// The kind of a registered type (`Message` or `Enum`).
///
/// This is a lightweight representation used during validation before full type
/// information is available.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Message,
    Enum,
}
