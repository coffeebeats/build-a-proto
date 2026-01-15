use std::collections::HashMap;

use crate::ast;
use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::PackageName;
use crate::ir::lower::TypeResolver;

/* -------------------------------------------------------------------------- */
/*                               Struct: Symbols                              */
/* -------------------------------------------------------------------------- */

/// `Symbols` is a symbol table tracking type data during compilation.
#[derive(Clone)]
pub struct Symbols<T: Clone> {
    types: HashMap<Descriptor, T>,
}

/* ----------------------------- Impl: Symbols ------------------------------ */

impl<T: Clone> Symbols<T> {
    #[allow(unused)]
    /// `contains` checks if a type [`Descriptor`] exists in the symbol table.
    pub fn contains(&self, desc: &Descriptor) -> bool {
        self.types.contains_key(desc)
    }

    #[allow(unused)]
    /// `get` looks up stored data for the specified [`Descriptor`].
    pub fn get(&self, desc: &Descriptor) -> Option<T> {
        self.types.get(desc).cloned()
    }

    /// `insert` records data by its [`Descriptor`], returning any existing
    /// value previously registered for `descriptor`.
    pub fn insert(&mut self, descriptor: Descriptor, value: T) -> Option<T> {
        self.types.insert(descriptor, value)
    }

    /// `resolve` resolves a type `reference` from the given `scope`.
    ///
    /// For absolute references, tries different package/path splits to find the
    /// referenced type.
    ///
    /// For relative references, walks up the scope hierarchy from most specific
    /// to least specific (nested → sibling → package level).
    pub fn resolve(
        &self,
        scope: &Descriptor,
        reference: &ast::Reference,
    ) -> Option<(Descriptor, T)> {
        debug_assert!(
            !reference.components.is_empty(),
            "Reference must have at least one component"
        );

        let parts: Vec<String> = reference
            .components
            .iter()
            .map(|c| c.name.clone())
            .collect();

        if reference.is_absolute {
            // Try different package/path splits for absolute references.
            for pkg_len in 1..=parts.len() {
                if let Ok(package) = PackageName::try_from(parts[..pkg_len].to_vec()) {
                    let path = parts[pkg_len..].to_vec();

                    let candidate = DescriptorBuilder::default()
                        .package(package)
                        .path(path)
                        .build()
                        .unwrap();

                    if let Some(value) = self.types.get(&candidate).cloned() {
                        return Some((candidate, value));
                    }
                }
            }

            return None;
        }

        debug_assert!(
            !reference.is_absolute,
            "This branch handles relative references only"
        );

        // For relative refs, package is always the same as scope's package.
        let package = scope.package.clone();
        let scope_path = &scope.path;

        // Try from most specific scope to package level. This handles self-
        // lookup (recursive types) and nested/sibling resolution.
        for depth in (0..=scope_path.len()).rev() {
            let mut candidate_path = scope_path[..depth].to_vec();
            candidate_path.extend(parts.clone());

            let candidate = DescriptorBuilder::default()
                .package(package.clone())
                .path(candidate_path)
                .build()
                .unwrap();

            if let Some(value) = self.types.get(&candidate).cloned() {
                return Some((candidate, value));
            }
        }

        None
    }
}

/* ------------------------------ Impl: Default ----------------------------- */

impl<T: Clone> Default for Symbols<T> {
    fn default() -> Self {
        Self {
            types: Default::default(),
        }
    }
}

/* --------------------------- Impl: TypeResolver --------------------------- */

impl<T: Clone> TypeResolver<T> for Symbols<T> {
    fn resolve(&self, scope: &Descriptor, reference: &ast::Reference) -> Option<(Descriptor, T)> {
        self.resolve(scope, reference)
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::span::Span as SpanTrait;

    /* --------------------------- Tests: absolute -------------------------- */

    #[test]
    fn test_resolve_absolute_package_level_type() {
        // Given: Symbol table with package-level type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Message".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["other", "package"]).unwrap())
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["com", "example", "Message"], true);

        // When: Resolve absolute reference.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to the registered type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Message");
    }

    #[test]
    fn test_resolve_absolute_nested_type() {
        // Given: Symbol table with nested type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Outer".to_string(), "Inner".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["other", "package"]).unwrap())
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["com", "example", "Outer", "Inner"], true);

        // When: Resolve absolute reference to nested type.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to the nested type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Outer.Inner");
    }

    #[test]
    fn test_resolve_absolute_invalid_package() {
        // Given: Symbol table without the referenced package.
        let symbols = Symbols::<()>::default();

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["invalid", "package", "Type"], true);

        // When: Resolve absolute reference with invalid package.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should return None.
        assert!(result.is_none());
    }

    /* --------------------------- Tests: relative -------------------------- */

    #[test]
    fn test_resolve_relative_type_not_found() {
        // Given: Empty symbol table.
        let symbols = Symbols::<()>::default();

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["NonExistent"], false);

        // When: Resolve reference to non-existent type.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should return None.
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_relative_from_empty_path_scope() {
        // Given: Symbol table with package-level type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Message".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Message"], false);

        // When: Resolve from scope with empty path and no name.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to package-level type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Message");
    }

    #[test]
    fn test_resolve_relative_multi_component_reference() {
        // Given: Symbol table with multi-component nested type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec![
                    "Outer".to_string(),
                    "Inner".to_string(),
                    "Field".to_string(),
                ])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Inner", "Field"], false);

        // When: Resolve multi-component relative reference.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to multi-component nested type.
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0.to_string(),
            "com.example.Outer.Inner.Field"
        );
    }

    /* -------------------- Tests: relative (nested type) ------------------- */

    #[test]
    fn test_resolve_relative_nested_in_current_message() {
        // Given: Symbol table with nested type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Outer".to_string(), "Inner".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Inner"], false);

        // When: Resolve relative reference to nested type.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to nested type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Outer.Inner");
    }

    #[test]
    fn test_resolve_relative_deeply_nested_type() {
        // Given: Symbol table with deeply nested type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec![
                    "Outer".to_string(),
                    "Middle".to_string(),
                    "Inner".to_string(),
                ])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string(), "Middle".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Inner"], false);

        // When: Resolve relative reference from deeply nested scope.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to deeply nested type.
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0.to_string(),
            "com.example.Outer.Middle.Inner"
        );
    }

    /* ---------------------- Tests: relative (sibling) --------------------- */

    #[test]
    fn test_resolve_relative_sibling_in_same_message() {
        // Given: Symbol table with sibling types.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Outer".to_string(), "Second".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string(), "First".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Second"], false);

        // When: Resolve relative reference to sibling.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to sibling type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Outer.Second");
    }

    #[test]
    fn test_resolve_relative_sibling_at_parent_level() {
        // Given: Symbol table with sibling at parent level.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec![
                    "Outer".to_string(),
                    "Middle".to_string(),
                    "Sibling".to_string(),
                ])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec![
                "Outer".to_string(),
                "Middle".to_string(),
                "Inner".to_string(),
            ])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Sibling"], false);

        // When: Resolve relative reference to parent-level sibling.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to sibling at parent level.
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0.to_string(),
            "com.example.Outer.Middle.Sibling"
        );
    }

    /* ---------------------- Tests: relative (package) --------------------- */

    #[test]
    fn test_resolve_relative_package_level_from_nested() {
        // Given: Symbol table with package-level type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Message".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string(), "Inner".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Message"], false);

        // When: Resolve relative reference to package-level type from nested scope.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to package-level type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Message");
    }

    #[test]
    fn test_resolve_relative_multi_component_package_level() {
        // Given: Symbol table with multi-component path in package.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["other".to_string(), "Message".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["other", "Message"], false);

        // When: Resolve multi-component relative reference.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to multi-component type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.other.Message");
    }

    /* -------------------- Tests: relative (self-lookup) ------------------- */

    #[test]
    fn test_resolve_relative_message_referencing_itself() {
        // Given: Symbol table with a message type.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Message".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Message".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Message"], false);

        // When: Resolve self-reference (recursive type like Node { Node next; }).
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to itself.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Message");
    }

    #[test]
    fn test_resolve_relative_nested_referencing_parent() {
        // Given: Symbol table with parent and nested types.
        let mut symbols = Symbols::<()>::default();
        symbols.insert(
            DescriptorBuilder::default()
                .package(PackageName::try_from(vec!["com", "example"]).unwrap())
                .path(vec!["Outer".to_string()])
                .build()
                .unwrap(),
            (),
        );

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string(), "Inner".to_string()])
            .build()
            .unwrap();

        let reference = make_test_reference(vec!["Outer"], false);

        // When: Resolve reference from nested type to parent.
        let result = symbols.resolve(&scope, &reference);

        // Then: Should resolve to parent type.
        assert!(result.is_some());
        assert_eq!(result.unwrap().0.to_string(), "com.example.Outer");
    }

    /* ----------------------- Fn: make_test_reference ---------------------- */

    fn make_test_reference(components: Vec<&str>, is_absolute: bool) -> ast::Reference {
        ast::Reference {
            components: components
                .into_iter()
                .map(|name| ast::Ident {
                    name: name.to_string(),
                    span: SpanTrait::new(Default::default(), 0..0),
                })
                .collect(),
            is_absolute,
            span: SpanTrait::new(Default::default(), 0..0),
        }
    }
}
