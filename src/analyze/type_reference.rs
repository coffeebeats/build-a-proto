use thiserror::Error;

use crate::analyze::{Analyzer, Diagnostic};
use crate::ast;
use crate::compile::Symbols;
use crate::core::Descriptor;
use crate::ir::lower::TypeKind;
use crate::visit::{Visitor, walk};

/* -------------------------------------------------------------------------- */
/*                       Struct: TypeReferenceResolver                        */
/* -------------------------------------------------------------------------- */

/// Analyzer that validates all type references resolve to existing types.
pub struct TypeReferenceResolver<'a> {
    symbols: &'a Symbols<TypeKind>,
    scope: Descriptor,
    diagnostics: Vec<Diagnostic>,
}

/* ------------------------------- Enum: Error ------------------------------ */

#[derive(Debug, Error)]
pub enum ReferenceError {
    #[error("could not resolve reference: {0}")]
    Unresolved(ast::Reference),
    #[error("invalid reference type: {0} is {1:?}, expected one of {2:?}")]
    InvalidType(ast::Reference, TypeKind, Vec<TypeKind>),
}

/* ----------------------- Impl: TypeReferenceResolver ---------------------- */

impl<'a> TypeReferenceResolver<'a> {
    /// `new` creates a new [`TypeReferenceResolver`] for the given package
    /// scope.
    pub fn new(symbols: &'a Symbols<TypeKind>, scope: Descriptor) -> Self {
        Self {
            diagnostics: Vec::new(),
            scope,
            symbols,
        }
    }

    /// `resolve` attempts to resolve the provided type reference.
    ///
    /// Validates that:
    /// 1. The reference exists in the symbol table
    /// 2. The reference is in scope (handled by `Symbols::resolve`)
    /// 3. The resolved type is a valid `TypeKind` (Message or Enum, not Package)
    fn resolve(
        &self,
        reference: &ast::Reference,
    ) -> Result<(Descriptor, TypeKind), ReferenceError> {
        match self.symbols.resolve(&self.scope, reference) {
            Some((descriptor, kind)) => {
                // Validate that the type is a valid reference target.
                // Package-level descriptors cannot be referenced as types.
                if kind == TypeKind::Package {
                    return Err(ReferenceError::InvalidType(
                        reference.clone(),
                        kind,
                        vec![TypeKind::Message, TypeKind::Enum],
                    ));
                }
                Ok((descriptor, kind))
            }
            None => Err(ReferenceError::Unresolved(reference.clone())),
        }
    }
}

/* ------------------------- Impl: Analyzer --------------------------------- */

impl Analyzer for TypeReferenceResolver<'_> {
    fn drain_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

/* ------------------- Impl: Visitor<TypeReferenceResolver> ---------------- */

impl<'ast> Visitor<'ast> for TypeReferenceResolver<'_> {
    fn visit_message(&mut self, msg: &'ast ast::Message) {
        // Push message name onto scope
        self.scope.push(msg.name.name.clone());

        // Continue walking
        walk::walk_message(self, msg);

        // Pop from scope
        self.scope.pop();
    }

    fn visit_enum(&mut self, enum_: &'ast ast::Enum) {
        // Push enum name onto scope
        self.scope.push(enum_.name.name.clone());

        // Continue walking
        walk::walk_enum(self, enum_);

        // Pop from scope
        self.scope.pop();
    }

    fn visit_reference(&mut self, reference: &'ast ast::Reference) {
        if let Err(err) = self.resolve(reference) {
            self.diagnostics.push(Diagnostic::error(
                reference.span.clone(),
                format!("{}: '{}'", err, reference),
            ));
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DescriptorBuilder, PackageName};
    use chumsky::span::Span as SpanTrait;

    /* --------------------------- Tests: resolve --------------------------- */

    #[test]
    fn test_resolve_unresolved_reference() {
        // Given: Empty symbols table and resolver.
        let symbols = Symbols::<TypeKind>::default();

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["NonExistent"], false);

        // When: Resolve reference to non-existent type.
        let result = resolver.resolve(&reference);

        // Then: Should return Unresolved error.
        assert!(result.is_err());
        match result.unwrap_err() {
            ReferenceError::Unresolved(_) => {}
            _ => panic!("Expected Unresolved error"),
        }
    }

    #[test]
    fn test_resolve_package_type_reference() {
        // Given: Symbols with package type (invalid for references).
        let mut symbols = Symbols::<TypeKind>::default();
        let package_desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();
        symbols.insert(package_desc, TypeKind::Package);

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["com", "example"], true);

        // When: Resolve reference to package type.
        let result = resolver.resolve(&reference);

        // Then: Should return InvalidType error.
        assert!(result.is_err());
        match result.unwrap_err() {
            ReferenceError::InvalidType(_, kind, allowed) => {
                assert_eq!(kind, TypeKind::Package);
                assert_eq!(allowed, vec![TypeKind::Message, TypeKind::Enum]);
            }
            _ => panic!("Expected InvalidType error"),
        }
    }

    #[test]
    fn test_resolve_valid_message_reference() {
        // Given: Symbols with a message type and resolver with scope.
        let mut symbols = Symbols::<TypeKind>::default();
        let message_desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Message".to_string()])
            .build()
            .unwrap();
        symbols.insert(message_desc.clone(), TypeKind::Message);

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["Message"], false);

        // When: Resolve the reference.
        let result = resolver.resolve(&reference);

        // Then: Should successfully resolve to Message type.
        assert!(result.is_ok());
        let (desc, kind) = result.unwrap();
        assert_eq!(desc, message_desc);
        assert_eq!(kind, TypeKind::Message);
    }

    #[test]
    fn test_resolve_valid_enum_reference() {
        // Given: Symbols with an enum type and resolver with scope.
        let mut symbols = Symbols::<TypeKind>::default();
        let enum_desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Status".to_string()])
            .build()
            .unwrap();
        symbols.insert(enum_desc.clone(), TypeKind::Enum);

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["Status"], false);

        // When: Resolve the reference.
        let result = resolver.resolve(&reference);

        // Then: Should successfully resolve to Enum type.
        assert!(result.is_ok());
        let (desc, kind) = result.unwrap();
        assert_eq!(desc, enum_desc);
        assert_eq!(kind, TypeKind::Enum);
    }

    #[test]
    fn test_resolve_nested_message_reference() {
        // Given: Symbols with nested message type.
        let mut symbols = Symbols::<TypeKind>::default();
        let nested_desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string(), "Inner".to_string()])
            .build()
            .unwrap();
        symbols.insert(nested_desc.clone(), TypeKind::Message);

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string()])
            .build()
            .unwrap();

        let resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["Inner"], false);

        // When: Resolve nested type reference.
        let result = resolver.resolve(&reference);

        // Then: Should successfully resolve nested message.
        assert!(result.is_ok());
        let (desc, kind) = result.unwrap();
        assert_eq!(desc, nested_desc);
        assert_eq!(kind, TypeKind::Message);
    }

    #[test]
    fn test_resolve_absolute_reference() {
        // Given: Symbols with message type and resolver.
        let mut symbols = Symbols::<TypeKind>::default();
        let message_desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Message".to_string()])
            .build()
            .unwrap();
        symbols.insert(message_desc.clone(), TypeKind::Message);

        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["other", "package"]).unwrap())
            .build()
            .unwrap();

        let resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["com", "example", "Message"], true);

        // When: Resolve absolute reference from different package.
        let result = resolver.resolve(&reference);

        // Then: Should successfully resolve across packages.
        assert!(result.is_ok());
        let (desc, kind) = result.unwrap();
        assert_eq!(desc, message_desc);
        assert_eq!(kind, TypeKind::Message);
    }

    /* --------------------------- Tests: visitor --------------------------- */

    #[test]
    fn test_visitor_tracks_scope_for_messages() {
        // Given: Resolver with initial scope and mock message.
        let symbols = Symbols::<TypeKind>::default();
        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let mut resolver = TypeReferenceResolver::new(&symbols, scope.clone());

        let message = ast::Message {
            comment: None,
            items: vec![],
            name: make_test_ident("TestMessage"),
            span: make_test_span(),
        };

        // When: Visit message.
        resolver.visit_message(&message);

        // Then: Scope should be restored after visiting.
        assert_eq!(resolver.scope, scope);
    }

    #[test]
    fn test_visitor_tracks_scope_for_enums() {
        // Given: Resolver with initial scope and mock enum.
        let symbols = Symbols::<TypeKind>::default();
        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let mut resolver = TypeReferenceResolver::new(&symbols, scope.clone());

        let enum_ = ast::Enum {
            comment: None,
            items: vec![],
            name: make_test_ident("TestEnum"),
            span: make_test_span(),
        };

        // When: Visit enum.
        resolver.visit_enum(&enum_);

        // Then: Scope should be restored after visiting.
        assert_eq!(resolver.scope, scope);
    }

    #[test]
    fn test_visitor_collects_diagnostic_for_invalid_reference() {
        // Given: Empty symbols and resolver with reference.
        let symbols = Symbols::<TypeKind>::default();
        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        let mut resolver = TypeReferenceResolver::new(&symbols, scope);
        let reference = make_test_reference(vec!["InvalidType"], false);

        // When: Visit invalid reference.
        resolver.visit_reference(&reference);

        // Then: Should have collected diagnostic error.
        let diagnostics = resolver.drain_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .message
                .contains("could not resolve reference")
        );
    }

    /* ----------------------- Fn: make_test_reference ---------------------- */

    fn make_test_reference(components: Vec<&str>, is_absolute: bool) -> ast::Reference {
        ast::Reference {
            components: components
                .into_iter()
                .map(|name| make_test_ident(name))
                .collect(),
            is_absolute,
            span: make_test_span(),
        }
    }

    /* ------------------------- Fn: make_test_ident ------------------------ */

    fn make_test_ident(name: &str) -> ast::Ident {
        ast::Ident {
            name: name.to_string(),
            span: make_test_span(),
        }
    }

    /* ------------------------- Fn: make_test_span ------------------------- */

    fn make_test_span() -> crate::lex::Span {
        SpanTrait::new(Default::default(), 0..0)
    }
}
