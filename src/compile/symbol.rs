use std::collections::HashMap;

use crate::core::Descriptor;
use crate::core::SchemaImport;
use crate::core::PackageName;
use crate::core::Reference;

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
        let fqn = String::from(&desc);
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
        scope: &Descriptor,
        reference: &Reference,
    ) -> Option<(Descriptor, TypeKind)> {
        if reference.is_absolute() {
            return self.resolve_absolute(reference);
        }

        self.resolve_relative(scope, reference)
    }

    /// `resolve_absolute` resolves an absolute reference (a reference beginning
    /// with a `.`).
    fn resolve_absolute(&self, reference: &Reference) -> Option<(Descriptor, TypeKind)> {
        let key = reference.to_string().strip_prefix('.').unwrap().to_string();

        let descriptor = self.descriptors.get(&key)?;
        let kind = self.types.get(descriptor)?;
        Some((descriptor.clone(), *kind))
    }

    /// `resolve_relative` resolves a relative reference by searching outward
    /// from the provided scope.
    fn resolve_relative(
        &self,
        scope: &Descriptor,
        reference: &Reference,
    ) -> Option<(Descriptor, TypeKind)> {
        let scope_name = String::from(scope);
        debug_assert!(!scope_name.is_empty());

        let mut parts: Vec<&str> = scope_name.split('.').filter(|s| !s.is_empty()).collect();

        // Try each scope level, from innermost to outermost
        loop {
            let candidate_name = if parts.is_empty() {
                reference.to_string()
            } else {
                format!("{}.{}", parts.join("."), reference)
            };

            if let Some(desc) = self.descriptors.get(&candidate_name) {
                let kind = self.types.get(desc);
                debug_assert!(kind.is_some());

                return kind.map(|k| (desc.clone(), *k));
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
    Variant,
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DescriptorBuilder;
    use crate::core::PackageName;

    #[test]
    fn test_symbols_contains() {
        // Given: A symbol table.
        let mut symbols = Symbols::default();

        // Given: A test descriptor.
        let descriptor = desc(&["pkg"], &[], "Message");

        // When: Inserting a type for that descriptor.
        symbols.insert_type(descriptor.clone(), TypeKind::Message);

        // Then: The type should exist.
        assert!(symbols.contains(&descriptor));
    }

    #[test]
    fn test_symbols_insert() {
        // Given: A symbol table.
        let mut symbols = Symbols::default();

        // Given: A test descriptor.
        let descriptor = desc(&["pkg"], &[], "Message");

        // When: Inserting a type for that descriptor.
        symbols.insert_type(descriptor.clone(), TypeKind::Message);

        // Then: The type should be found.
        assert_eq!(symbols.get_type(&descriptor), Some(TypeKind::Message));
    }

    #[test]
    fn test_symbols_get_type() {
        // Given: A symbol table with both message and variant.
        let mut symbols = Symbols::default();
        let message = desc(&["pkg"], &[], "Message");
        let variant = desc(&["pkg"], &[], "Variant");

        // When: Inserting different types.
        symbols.insert_type(message.clone(), TypeKind::Message);
        symbols.insert_type(variant.clone(), TypeKind::Variant);

        // Then: Each should have the correct type.
        assert_eq!(symbols.get_type(&message), Some(TypeKind::Message));
        assert_eq!(symbols.get_type(&variant), Some(TypeKind::Variant));
    }

    /* --------------------- Tests: Absolute references --------------------- */

    #[test]
    fn test_symbols_resolve_absolute_package_level_type() {
        // Given: A symbol table with a package-level type.
        let mut symbols = Symbols::default();
        let descriptor = desc(&["foo"], &[], "Bar");
        symbols.insert_type(descriptor.clone(), TypeKind::Message);

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo"], "Bar"));

        // Then: The type should be resolved correctly.
        assert_eq!(result, Some((descriptor, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_absolute_nested_type() {
        // Given: A symbol table with a nested type.
        let mut symbols = Symbols::default();
        let inner = desc(&["foo"], &["Outer"], "Inner");
        symbols.insert_type(inner.clone(), TypeKind::Message);

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo", "Outer"], "Inner"));

        // Then: The nested type should be resolved.
        assert_eq!(result, Some((inner, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_absolute_multi_part_package() {
        // Given: A symbol table with a multi-part package.
        let mut symbols = Symbols::default();
        let descriptor = desc(&["foo", "bar"], &[], "Baz");
        symbols.insert_type(descriptor.clone(), TypeKind::Message);

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo", "bar"], "Baz"));

        // Then: The type should be resolved.
        assert_eq!(result, Some((descriptor, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_absolute_unknown_type() {
        // Given: A symbol table with some types.
        let mut symbols = Symbols::default();
        symbols.insert_type(desc(&["foor"], &[], "Bar"), TypeKind::Message);

        // When: Resolving a reference to an unknown type.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, &type_ref_abs(&["foo"], "Unknown"));

        // Then: Resolution should fail.
        assert_eq!(result, None);
    }

    /* --------------------- Tests: Relative references --------------------- */

    #[test]
    fn test_symbols_resolve_relative_sibling_type() {
        // Given: Two types in the same package.
        let mut symbols = Symbols::default();
        let bar = desc(&["foo"], &[], "Bar");
        let baz = desc(&["foo"], &[], "Baz");
        symbols.insert_type(bar, TypeKind::Message);
        symbols.insert_type(baz.clone(), TypeKind::Message);

        // When: Resolving a sibling reference from Bar's scope.
        let scope = desc(&["foo"], &[], "Other");
        let result = symbols.find(&scope, &type_ref(&[], "Baz"));

        // Then: The sibling type should be found.
        assert_eq!(result, Some((baz, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_relative_nested_child() {
        // Given: A parent message with a nested child.
        let mut symbols = Symbols::default();
        let outer = desc(&["foo"], &[], "Outer");
        let inner = desc(&["foo"], &["Outer"], "Inner");
        symbols.insert_type(outer, TypeKind::Message);
        symbols.insert_type(inner.clone(), TypeKind::Message);

        // When: Resolving a child reference from the parent.
        let scope = desc(&["foo"], &[], "Outer");
        let result = symbols.find(&scope, &type_ref(&[], "Inner"));

        // Then: The child should be found.
        assert_eq!(result, Some((inner, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_relative_parent_scope() {
        // Given: Nested messages where an inner references an outer sibling.
        let mut symbols = Symbols::default();
        let outer = desc(&["foo"], &[], "Outer");
        let sibling = desc(&["foo"], &[], "Sibling");
        let inner = desc(&["foo"], &["Outer"], "Inner");
        symbols.insert_type(outer, TypeKind::Message);
        symbols.insert_type(sibling.clone(), TypeKind::Message);
        symbols.insert_type(inner, TypeKind::Message);

        // When: Resolving from inner scope to outer scope sibling.
        let scope = desc(&["foo"], &["Outer"], "Inner");
        let result = symbols.find(&scope, &type_ref(&[], "Sibling"));

        // Then: The type should be found in parent scope.
        assert_eq!(result, Some((sibling, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_relative_shadowing() {
        // Given: A type name exists at both nested and package level.
        let mut symbols = Symbols::default();
        let package_level = desc(&["foo"], &[], "Config");
        let nested = desc(&["foo"], &["Outer"], "Config");
        symbols.insert_type(package_level, TypeKind::Message);
        symbols.insert_type(nested.clone(), TypeKind::Message);

        // When: Resolving from within Outer.
        let scope = desc(&["foo"], &["Outer"], "Inner");
        let result = symbols.find(&scope, &type_ref(&[], "Config"));

        // Then: The nested type should shadow the package-level type.
        assert_eq!(result, Some((nested, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_relative_multi_part_reference() {
        // Given: A deeply nested type.
        let mut symbols = Symbols::default();
        let deep = desc(&["foo"], &["Outer", "Middle"], "Inner");
        symbols.insert_type(deep.clone(), TypeKind::Message);

        // When: Resolving a multi-part reference from package root.
        let scope = desc(&["foo"], &[], "Other");
        let result = symbols.find(&scope, &type_ref(&["Outer", "Middle"], "Inner"));

        // Then: The type should be found.
        assert_eq!(result, Some((deep, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_relative_imported_package() {
        // Given: Types in different packages.
        let mut symbols = Symbols::default();
        let bar = desc(&["foo1"], &[], "Bar");
        let baz = desc(&["foo2"], &[], "Baz");
        symbols.insert_type(bar, TypeKind::Message);
        symbols.insert_type(baz.clone(), TypeKind::Message);

        // When: Resolving a cross-package reference.
        let scope = desc(&["foo1"], &[], "Other");
        let result = symbols.find(&scope, &type_ref(&["foo2"], "Baz"));

        // Then: The imported package type should be found.
        assert_eq!(result, Some((baz, TypeKind::Message)));
    }

    #[test]
    fn test_symbols_resolve_relative_unknown_type() {
        // Given: A symbol table with some types.
        let mut symbols = Symbols::default();
        symbols.insert_type(desc(&["food"], &[], "Bar"), TypeKind::Message);

        // When: Resolving a reference to an unknown type.
        let scope = desc(&["food"], &[], "Bar");
        let result = symbols.find(&scope, &type_ref(&[], "Unknown"));

        // Then: Resolution should fail.
        assert_eq!(result, None);
    }

    #[test]
    fn test_symbols_resolve_relative_deeply_nested_scope() {
        // Given: A type at package level.
        let mut symbols = Symbols::default();
        let target = desc(&["foo"], &[], "Target");
        symbols.insert_type(target.clone(), TypeKind::Message);

        // When: Resolving from a deeply nested scope.
        let scope = desc(&["foo"], &["A", "B", "C", "D"], "Deep");
        let result = symbols.find(&scope, &type_ref(&[], "Target"));

        // Then: The type should still be found (walks up to package root).
        assert_eq!(result, Some((target, TypeKind::Message)));
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
}
