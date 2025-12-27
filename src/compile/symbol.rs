use std::collections::HashMap;

use crate::core::{Descriptor, SchemaImport};

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
    pub fn find(&self, scope: &Descriptor, reference: &str) -> Option<(Descriptor, TypeKind)> {
        if let Some(reference_abs) = reference.strip_prefix('.') {
            return self.resolve_absolute(reference_abs);
        }

        self.resolve_relative(scope, reference)
    }

    /// `resolve_absolute` resolves an absolute reference (a reference beginning
    /// with a `.`).
    fn resolve_absolute(&self, reference: &str) -> Option<(Descriptor, TypeKind)> {
        let descriptor = self.descriptors.get(reference)?;
        let kind = self.types.get(descriptor)?;
        Some((descriptor.clone(), *kind))
    }

    /// `resolve_relative` resolves a relative reference by searching outward
    /// from the provided scope.
    fn resolve_relative(
        &self,
        scope: &Descriptor,
        reference: &str,
    ) -> Option<(Descriptor, TypeKind)> {
        if reference.is_empty() {
            return None;
        }

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
    /// Package namespace (e.g., ["game", "entities"])
    pub package: Vec<String>,
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

    #[test]
    fn test_symbols_new_is_empty() {
        // Given: A new symbol table.
        let symbols = Symbols::default();

        // When: Checking for any descriptor.
        let descriptor = desc(&["pkg"], &[], "Type");

        // Then: The symbol table should not contain it.
        assert!(!symbols.contains(&descriptor));
        assert_eq!(symbols.get_type(&descriptor), None);
    }

    #[test]
    fn test_symbols_insert_and_contains() {
        // Given: A symbol table.
        let mut symbols = Symbols::default();
        let descriptor = desc(&["pkg"], &[], "Message");

        // When: Inserting a type.
        symbols.insert_type(descriptor.clone(), TypeKind::Message);

        // Then: The type should be found.
        assert!(symbols.contains(&descriptor));
        assert_eq!(symbols.get_type(&descriptor), Some(TypeKind::Message));
    }

    #[test]
    fn test_symbols_type_kinds() {
        // Given: A symbol table with both message and enum.
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
    fn test_resolve_absolute_package_level_type() {
        // Given: A symbol table with a package-level type.
        let mut symbols = Symbols::default();
        let player = desc(&["game"], &[], "Player");
        symbols.insert_type(player.clone(), TypeKind::Message);

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, ".game.Player");

        // Then: The type should be resolved correctly.
        assert_eq!(result, Some((player, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_absolute_nested_type() {
        // Given: A symbol table with a nested type.
        let mut symbols = Symbols::default();
        let inner = desc(&["game"], &["Outer"], "Inner");
        symbols.insert_type(inner.clone(), TypeKind::Message);

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, ".game.Outer.Inner");

        // Then: The nested type should be resolved.
        assert_eq!(result, Some((inner, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_absolute_multi_part_package() {
        // Given: A symbol table with a multi-part package.
        let mut symbols = Symbols::default();
        let entity = desc(&["game", "entities"], &[], "Player");
        symbols.insert_type(entity.clone(), TypeKind::Message);

        // When: Resolving an absolute reference.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, ".game.entities.Player");

        // Then: The type should be resolved.
        assert_eq!(result, Some((entity, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_absolute_unknown_type() {
        // Given: A symbol table with some types.
        let mut symbols = Symbols::default();
        symbols.insert_type(desc(&["game"], &[], "Player"), TypeKind::Message);

        // When: Resolving a reference to an unknown type.
        let scope = desc(&["other"], &[], "Other");
        let result = symbols.find(&scope, ".game.Unknown");

        // Then: Resolution should fail.
        assert_eq!(result, None);
    }

    /* --------------------- Tests: Relative references --------------------- */

    #[test]
    fn test_resolve_relative_sibling_type() {
        // Given: Two types in the same package.
        let mut symbols = Symbols::default();
        let player = desc(&["game"], &[], "Player");
        let enemy = desc(&["game"], &[], "Enemy");
        symbols.insert_type(player, TypeKind::Message);
        symbols.insert_type(enemy.clone(), TypeKind::Message);

        // When: Resolving a sibling reference from Player's scope.
        let scope = desc(&["game"], &[], "Player");
        let result = symbols.find(&scope, "Enemy");

        // Then: The sibling type should be found.
        assert_eq!(result, Some((enemy, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_relative_nested_child() {
        // Given: A parent message with a nested child.
        let mut symbols = Symbols::default();
        let outer = desc(&["game"], &[], "Outer");
        let inner = desc(&["game"], &["Outer"], "Inner");
        symbols.insert_type(outer, TypeKind::Message);
        symbols.insert_type(inner.clone(), TypeKind::Message);

        // When: Resolving a child reference from the parent.
        let scope = desc(&["game"], &[], "Outer");
        let result = symbols.find(&scope, "Inner");

        // Then: The child should be found.
        assert_eq!(result, Some((inner, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_relative_parent_scope() {
        // Given: Nested messages where an inner references an outer sibling.
        let mut symbols = Symbols::default();
        let outer = desc(&["game"], &[], "Outer");
        let sibling = desc(&["game"], &[], "Sibling");
        let inner = desc(&["game"], &["Outer"], "Inner");
        symbols.insert_type(outer, TypeKind::Message);
        symbols.insert_type(sibling.clone(), TypeKind::Message);
        symbols.insert_type(inner, TypeKind::Message);

        // When: Resolving from inner scope to outer scope sibling.
        let scope = desc(&["game"], &["Outer"], "Inner");
        let result = symbols.find(&scope, "Sibling");

        // Then: The type should be found in parent scope.
        assert_eq!(result, Some((sibling, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_relative_shadowing() {
        // Given: A type name exists at both nested and package level.
        let mut symbols = Symbols::default();
        let package_level = desc(&["game"], &[], "Config");
        let nested = desc(&["game"], &["Outer"], "Config");
        symbols.insert_type(package_level, TypeKind::Message);
        symbols.insert_type(nested.clone(), TypeKind::Message);

        // When: Resolving from within Outer.
        let scope = desc(&["game"], &["Outer"], "Inner");
        let result = symbols.find(&scope, "Config");

        // Then: The nested type should shadow the package-level type.
        assert_eq!(result, Some((nested, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_relative_multi_part_reference() {
        // Given: A deeply nested type.
        let mut symbols = Symbols::default();
        let deep = desc(&["game"], &["Outer", "Middle"], "Inner");
        symbols.insert_type(deep.clone(), TypeKind::Message);

        // When: Resolving a multi-part reference from package root.
        let scope = desc(&["game"], &[], "Other");
        let result = symbols.find(&scope, "Outer.Middle.Inner");

        // Then: The type should be found.
        assert_eq!(result, Some((deep, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_relative_imported_package() {
        // Given: Types in different packages.
        let mut symbols = Symbols::default();
        let player = desc(&["game"], &[], "Player");
        let enemy = desc(&["entities"], &[], "Enemy");
        symbols.insert_type(player, TypeKind::Message);
        symbols.insert_type(enemy.clone(), TypeKind::Message);

        // When: Resolving a cross-package reference.
        let scope = desc(&["game"], &[], "Combat");
        let result = symbols.find(&scope, "entities.Enemy");

        // Then: The imported package type should be found.
        assert_eq!(result, Some((enemy, TypeKind::Message)));
    }

    #[test]
    fn test_resolve_relative_unknown_type() {
        // Given: A symbol table with some types.
        let mut symbols = Symbols::default();
        symbols.insert_type(desc(&["game"], &[], "Player"), TypeKind::Message);

        // When: Resolving a reference to an unknown type.
        let scope = desc(&["game"], &[], "Player");
        let result = symbols.find(&scope, "Unknown");

        // Then: Resolution should fail.
        assert_eq!(result, None);
    }

    /* -------------------- Tests: Edge Cases ------------------- */

    #[test]
    fn test_resolve_empty_reference() {
        // Given: A symbol table
        let symbols = Symbols::default();
        let scope = desc(&["game"], &[], "Player");

        // When: Attempting to resolve an empty reference
        let result = symbols.find(&scope, "");

        // Then: Resolution should fail
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_deeply_nested_scope() {
        // Given: A type at package level
        let mut symbols = Symbols::default();
        let target = desc(&["game"], &[], "Target");
        symbols.insert_type(target.clone(), TypeKind::Message);

        // When: Resolving from a deeply nested scope
        let scope = desc(&["game"], &["A", "B", "C", "D"], "Deep");
        let result = symbols.find(&scope, "Target");

        // Then: The type should still be found (walks up to package root)
        assert_eq!(result, Some((target, TypeKind::Message)));
    }

    /* ------------------------------ Fn: desc ------------------------------ */

    fn desc(package: &[&str], path: &[&str], name: &str) -> Descriptor {
        DescriptorBuilder::default()
            .package(package.iter().map(|s| s.to_string()).collect())
            .path(path.iter().map(|s| s.to_string()).collect())
            .name(name.to_string())
            .build()
            .unwrap()
    }
}
