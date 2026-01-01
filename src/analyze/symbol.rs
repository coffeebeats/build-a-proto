//! Symbol table for semantic analysis.
//!
//! The symbol table tracks all type definitions and their metadata during
//! analysis. It provides resolution of type references within scopes.

use std::collections::HashMap;

use crate::ast::Encoding;
use crate::ast::ScalarType;
use crate::core::Descriptor;
use crate::core::PackageName;
use crate::core::Reference;
use crate::core::SchemaImport;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                             Struct: SymbolTable                            */
/* -------------------------------------------------------------------------- */

/// `SymbolTable` is the central registry of all type definitions.
///
/// The symbol table stores type entries keyed by their fully-qualified name
/// (FQN) string, and provides resolution of both absolute and relative type
/// references.
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// `types` maps a type's FQN (`String`) to [`TypeEntry`].
    types: HashMap<String, TypeEntry>,
    /// `modules` maps a module path to its module metadata [`ModuleEntry`].
    modules: HashMap<SchemaImport, ModuleEntry>,
    /// `descriptors` maps a type's FQN string to a [`Descriptor`].
    descriptors: HashMap<String, Descriptor>,
}

/* ---------------------------- Impl: SymbolTable --------------------------- */

impl SymbolTable {
    /// `register_type` registers a type in the symbol table.
    pub fn register_type(&mut self, entry: TypeEntry) -> Option<TypeEntry> {
        let fqn = entry.descriptor.to_string();
        self.descriptors
            .insert(fqn.clone(), entry.descriptor.clone());
        self.types.insert(fqn, entry)
    }

    /// `register_module` registers a module in the symbol table.
    pub fn register_module(&mut self, import: SchemaImport, entry: ModuleEntry) {
        self.modules.insert(import, entry);
    }

    /// `get_type` looks up a type by its [`Descriptor`].
    pub fn get_type(&self, desc: &Descriptor) -> Option<&TypeEntry> {
        let fqn = desc.to_string();
        self.types.get(&fqn)
    }

    /// `get_type_by_fqn` looks up a [`TypeEntry`] by its FQN string.
    pub fn get_type_by_fqn(&self, fqn: &str) -> Option<&TypeEntry> {
        self.types.get(fqn)
    }

    /// `get_type_mut` gets a mutable reference to a [`TypeEntry`].
    pub fn get_type_mut(&mut self, desc: &Descriptor) -> Option<&mut TypeEntry> {
        let fqn = desc.to_string();
        self.types.get_mut(&fqn)
    }

    /// `contains` checks if a [`TypeEntry`] exists for the [`Descriptor`].
    pub fn contains(&self, desc: &Descriptor) -> bool {
        self.types.contains_key(&desc.to_string())
    }

    /// `get_module` gets a module entry by its [`SchemaImport`].
    pub fn get_module(&self, import: &SchemaImport) -> Option<&ModuleEntry> {
        self.modules.get(import)
    }

    /// `iter_modules` iterates over all module entries.
    pub fn iter_modules(&self) -> impl Iterator<Item = (&SchemaImport, &ModuleEntry)> {
        self.modules.iter()
    }

    /// `iter_types` iterates over all [`TypeEntry`] types in the table.
    pub fn iter_types(&self) -> impl Iterator<Item = (&String, &TypeEntry)> {
        self.types.iter()
    }

    /// `find` resolves a reference to a [`TypeEntry`] within a given scope.
    ///
    /// Resolution strategy:
    /// - Absolute references (starting with `.`) are resolved directly.
    /// - Relative references search outward from the current scope.
    pub fn find(&self, scope: &Descriptor, reference: &Reference) -> Option<&TypeEntry> {
        if reference.is_absolute() {
            return self.resolve_absolute(reference);
        }

        self.resolve_relative(scope, reference)
    }

    /// `resolve_absolute` resolves an absolute reference (e.g., `.foo.Bar`).
    fn resolve_absolute(&self, reference: &Reference) -> Option<&TypeEntry> {
        // Strip leading dot and look up directly
        let key = reference
            .to_string()
            .strip_prefix('.')
            .unwrap_or_default()
            .to_string();
        self.types.get(&key)
    }

    /// `resolve_relative` resolves a relative reference by searching outward
    /// from the provided [`Descriptor`] scope.
    fn resolve_relative(&self, scope: &Descriptor, reference: &Reference) -> Option<&TypeEntry> {
        let scope_name = scope.to_string();
        let mut parts: Vec<&str> = scope_name.split('.').filter(|s| !s.is_empty()).collect();

        // Try each scope level, from innermost to outermost
        loop {
            let candidate_name = if parts.is_empty() {
                reference.to_string()
            } else {
                format!("{}.{}", parts.join("."), reference)
            };

            if let Some(entry) = self.types.get(&candidate_name) {
                return Some(entry);
            }

            if parts.is_empty() {
                break;
            }
            parts.pop();
        }

        None
    }
}

/* -------------------------------------------------------------------------- */
/*                              Struct: TypeEntry                             */
/* -------------------------------------------------------------------------- */

/// `TypeEntry` contains full metadata about a registered type.
#[derive(Debug, Clone)]
pub struct TypeEntry {
    /// The fully qualified descriptor for this type.
    pub descriptor: Descriptor,
    /// The kind of type (Message or Enum) with details.
    pub kind: TypeKind,
    /// The span where this type is defined.
    pub span: Span,
    /// The source file that defined this type.
    pub source: SchemaImport,
}

/* -------------------------------------------------------------------------- */
/*                               Enum: TypeKind                               */
/* -------------------------------------------------------------------------- */

/// `TypeKind` defines the kind of type, along with its members.
#[derive(Debug, Clone)]
pub enum TypeKind {
    /// `Message` is message type with fields and nested types.
    Message {
        fields: Vec<FieldEntry>,
        nested: Vec<Descriptor>,
    },
    /// `Enum` is an enum type with variants.
    Enum { variants: Vec<VariantEntry> },
}

/* ----------------------------- Impl: TypeKind ----------------------------- */

impl TypeKind {
    /// `is_message` returns true if this is a [`Message`] type.
    pub fn is_message(&self) -> bool {
        matches!(self, TypeKind::Message { .. })
    }

    /// `is_enum` returns true if this is an [`Enum`] type.
    pub fn is_enum(&self) -> bool {
        matches!(self, TypeKind::Enum { .. })
    }
}

/* -------------------------------------------------------------------------- */
/*                             Struct: FieldEntry                             */
/* -------------------------------------------------------------------------- */

/// `FieldEntry` contains resolved field information.
#[derive(Debug, Clone)]
pub struct FieldEntry {
    /// `encoding` is an optional encoding specification.
    pub encoding: Option<Vec<Encoding>>,
    /// `index` is the field's index.
    pub index: u64,
    /// `name` is the field's name.
    pub name: String,
    /// `resolved_type` is the field's resolved type.
    pub resolved_type: ResolvedType,
    /// `span` is the source code span of the field definition.
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                            Struct: VariantEntry                            */
/* -------------------------------------------------------------------------- */

/// `VariantEntry` contains resolved variant information.
#[derive(Debug, Clone)]
pub struct VariantEntry {
    /// `index` is the variant's index.
    pub index: u64,
    /// `kind` is the variant's kind.
    pub kind: VariantKind,
    /// `name` is the variant's name.
    pub name: String,
    /// `span` is the source code span of the variant definition.
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                             Enum: VariantKind                              */
/* -------------------------------------------------------------------------- */

/// `VariantKind` is the kind of [`Enum`] variant.
#[derive(Debug, Clone)]
pub enum VariantKind {
    /// `Unit` is a unit variant with no associated data.
    Unit,
    /// `Field` is a variant with an associated [`FieldEntry`].
    Field(FieldEntry),
}

/* -------------------------------------------------------------------------- */
/*                            Enum: ResolvedType                              */
/* -------------------------------------------------------------------------- */

/// `ResolvedType` is a type whose name has been resolved.
#[derive(Debug, Clone)]
pub enum ResolvedType {
    /// `Array` is an array with an element type and optional fixed size.
    Array {
        element: Box<ResolvedType>,
        size: Option<u64>,
    },
    /// `Scalar` is a scalar type.
    Scalar(ScalarType),
    /// `Map` is a map with key and value types.
    Map {
        key: Box<ResolvedType>,
        value: Box<ResolvedType>,
    },
    /// `Named` is a reference to another type.
    Named(Descriptor),
    /// `Unresolved` is a placeholder for unresolved references; this is useful
    /// for error recovery in the parser/compiler.
    Unresolved(String),
}

/* -------------------------------------------------------------------------- */
/*                            Struct: ModuleEntry                             */
/* -------------------------------------------------------------------------- */

/// `ModuleEntry` defines metadata about a compiled module.
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    /// The package namespace.
    pub package: PackageName,
    /// Dependencies (include statements).
    pub deps: Vec<SchemaImport>,
    /// Types defined in this module.
    pub types: Vec<Descriptor>,
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DescriptorBuilder;

    /* --------------------- Tests: Absolute references --------------------- */

    #[test]
    fn test_resolve_absolute_package_level_type() {
        // Given: A symbol table with a package-level type.
        let mut symbols = SymbolTable::default();
        let descriptor = desc(&["foo"], &[], "Bar");
        symbols.register_type(make_type_entry(descriptor.clone()));

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo"], "Bar"));

        // Then: The type should be resolved correctly.
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, descriptor);
    }

    #[test]
    fn test_resolve_absolute_nested_type() {
        // Given: A symbol table with a nested type.
        let mut symbols = SymbolTable::default();
        let inner = desc(&["foo"], &["Outer"], "Inner");
        symbols.register_type(make_type_entry(inner.clone()));

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo", "Outer"], "Inner"));

        // Then: The nested type should be resolved.
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, inner);
    }

    #[test]
    fn test_resolve_absolute_unknown_type() {
        // Given: A symbol table with some types.
        let mut symbols = SymbolTable::default();
        symbols.register_type(make_type_entry(desc(&["foo"], &[], "Bar")));

        // When: Resolving a reference to an unknown type.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo"], "Unknown"));

        // Then: Resolution should fail.
        assert!(result.is_none());
    }

    /* --------------------- Tests: Relative references --------------------- */

    #[test]
    fn test_resolve_relative_sibling_type() {
        // Given: Two types in the same package.
        let mut symbols = SymbolTable::default();
        symbols.register_type(make_type_entry(desc(&["foo"], &[], "Bar")));
        let baz = desc(&["foo"], &[], "Baz");
        symbols.register_type(make_type_entry(baz.clone()));

        // When: Resolving a sibling reference from Bar's scope.
        let scope = desc(&["foo"], &[], "Other");
        let result = symbols.find(&scope, &type_ref(&[], "Baz"));

        // Then: The sibling type should be found.
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, baz);
    }

    #[test]
    fn test_resolve_relative_nested_child() {
        // Given: A parent message with a nested child.
        let mut symbols = SymbolTable::default();
        symbols.register_type(make_type_entry(desc(&["foo"], &[], "Outer")));
        let inner = desc(&["foo"], &["Outer"], "Inner");
        symbols.register_type(make_type_entry(inner.clone()));

        // When: Resolving a child reference from the parent.
        let scope = desc(&["foo"], &[], "Outer");
        let result = symbols.find(&scope, &type_ref(&[], "Inner"));

        // Then: The child should be found.
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, inner);
    }

    #[test]
    fn test_resolve_relative_parent_scope() {
        // Given: Nested messages where an inner references an outer sibling.
        let mut symbols = SymbolTable::default();
        symbols.register_type(make_type_entry(desc(&["foo"], &[], "Outer")));
        let sibling = desc(&["foo"], &[], "Sibling");
        symbols.register_type(make_type_entry(sibling.clone()));
        symbols.register_type(make_type_entry(desc(&["foo"], &["Outer"], "Inner")));

        // When: Resolving from inner scope to outer scope sibling.
        let scope = desc(&["foo"], &["Outer"], "Inner");
        let result = symbols.find(&scope, &type_ref(&[], "Sibling"));

        // Then: The type should be found in parent scope.
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, sibling);
    }

    #[test]
    fn test_resolve_relative_shadowing() {
        // Given: A type name exists at both nested and package level.
        let mut symbols = SymbolTable::default();
        symbols.register_type(make_type_entry(desc(&["foo"], &[], "Config")));
        let nested = desc(&["foo"], &["Outer"], "Config");
        symbols.register_type(make_type_entry(nested.clone()));

        // When: Resolving from within Outer.
        let scope = desc(&["foo"], &["Outer"], "Inner");
        let result = symbols.find(&scope, &type_ref(&[], "Config"));

        // Then: The nested type should shadow the package-level type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, nested);
    }

    #[test]
    fn test_resolve_relative_unknown_type() {
        // Given: A symbol table with some types.
        let mut symbols = SymbolTable::default();
        symbols.register_type(make_type_entry(desc(&["foo"], &[], "Bar")));

        // When: Resolving a reference to an unknown type.
        let scope = desc(&["foo"], &[], "Bar");
        let result = symbols.find(&scope, &type_ref(&[], "Unknown"));

        // Then: Resolution should fail.
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_relative_deeply_nested_scope() {
        // Given: A type at package level.
        let mut symbols = SymbolTable::default();
        let target = desc(&["foo"], &[], "Target");
        symbols.register_type(make_type_entry(target.clone()));

        // When: Resolving from a deeply nested scope.
        let scope = desc(&["foo"], &["A", "B", "C", "D"], "Deep");
        let result = symbols.find(&scope, &type_ref(&[], "Target"));

        // Then: The type should still be found (walks up to package root).
        assert!(result.is_some());
        assert_eq!(result.unwrap().descriptor, target);
    }

    /* ------------------------------ Fn: desc ------------------------------ */

    fn desc(package: &[&str], path: &[&str], name: &str) -> Descriptor {
        DescriptorBuilder::default()
            .package(PackageName::try_from(package.to_vec()).unwrap())
            .path(path.iter().map(|s| s.to_string()).collect())
            .name(name.to_string())
            .build()
            .unwrap()
    }

    /* ---------------------------- Fn: type_ref ---------------------------- */

    fn type_ref(path: &[&str], name: &str) -> Reference {
        Reference::try_new_relative(path.iter().map(|&s| s.to_owned()).collect(), name).unwrap()
    }

    /* -------------------------- Fn: type_ref_abs -------------------------- */

    fn type_ref_abs(path: &[&str], name: &str) -> Reference {
        Reference::try_new_absolute(path.iter().map(|&s| s.to_owned()).collect(), name).unwrap()
    }

    /* ------------------------- Fn: make_type_entry ------------------------ */

    fn make_type_entry(descriptor: Descriptor) -> TypeEntry {
        TypeEntry {
            descriptor,
            kind: TypeKind::Message {
                fields: Vec::new(),
                nested: Vec::new(),
            },
            span: Span::from(0..0),
            source: make_test_import(),
        }
    }

    fn make_test_import() -> SchemaImport {
        // Create a temp file with .baproto extension for valid SchemaImport
        let temp = tempfile::Builder::new()
            .suffix(".baproto")
            .tempfile()
            .unwrap();
        SchemaImport::try_from(temp.path().to_path_buf()).unwrap()
    }
}