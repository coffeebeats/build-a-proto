//! AST-to-IR lowering logic using a trait-based approach.
//!
//! Each AST type implements the `Lower` trait, which converts it to the
//! corresponding IR type given a lowering context.

mod comment;
mod encoding;
mod enumeration;
mod field;
mod message;
mod schema;
mod types;

use crate::core::Descriptor;

/* -------------------------------------------------------------------------- */
/*                               Enum: TypeKind                               */
/* -------------------------------------------------------------------------- */

/// `TypeKind` represents the kind of a type definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Message,
    Enum,
}

/* -------------------------------------------------------------------------- */
/*                            Trait: TypeResolver                             */
/* -------------------------------------------------------------------------- */

/// `TypeResolver` provides symbol resolution for lowering references to IR.
///
/// Implementors are responsible for validating that references exist and
/// returning their resolved descriptor and kind.
pub trait TypeResolver {
    /// `resolve` looks up a type reference and returns its descriptor and kind.
    ///
    /// - `scope`: The current scope (package + parent type path)
    /// - `reference`: The reference components as strings
    /// - `is_absolute`: Whether the reference starts with `.`
    fn resolve(
        &self,
        scope: &[String],
        reference: &[String],
        is_absolute: bool,
    ) -> Option<(String, TypeKind)>;
}

/* -------------------------------------------------------------------------- */
/*                                Trait: Lower                                */
/* -------------------------------------------------------------------------- */

/// `Lower` converts an AST node to its corresponding IR representation.
///
/// The trait is generic over the context type, allowing each AST node to
/// specify what context it needs. Most types use `LowerContext` (the default).
pub trait Lower<'a, T, Ctx> {
    /// `lower` converts this AST node to an IR node using the provided context.
    fn lower(&'a self, ctx: &'a Ctx) -> Option<T>;
}

/* -------------------------------------------------------------------------- */
/*                            Struct: LowerContext                            */
/* -------------------------------------------------------------------------- */

/// `LowerContext` provides the environment needed for lowering AST to IR.
#[derive(Clone)]
pub struct LowerContext<'a, R: TypeResolver> {
    /// Type resolver for looking up referenced types.
    pub resolver: &'a R,
    /// Parent descriptor (defines the current scope).
    pub parent: Descriptor,
}

/* --------------------------- Impl: LowerContext --------------------------- */

impl<'a, R: TypeResolver> LowerContext<'a, R> {
    /// `new` creates a new root context for lowering a top-level schema.
    pub fn new(resolver: &'a R, parent: Descriptor) -> Self {
        Self { resolver, parent }
    }

    /// `with_child` creates a child context by extending the parent descriptor.
    pub fn with_child(&self, name: String) -> Self {
        let mut path = self.parent.path.clone();
        if let Some(parent_name) = &self.parent.name {
            path.push(parent_name.clone());
        }

        Self {
            resolver: self.resolver,
            parent: Descriptor {
                package: self.parent.package.clone(),
                path,
                name: Some(name),
            },
        }
    }

    /// Returns the current scope as a slice of strings.
    pub fn scope(&self) -> Vec<String> {
        let mut scope = self.parent.path.clone();
        if let Some(name) = &self.parent.name {
            scope.push(name.clone());
        }
        scope
    }

    /// Builds a descriptor for a child type.
    pub fn build_child_descriptor(&self, name: &str) -> String {
        self.with_child(name.to_string()).parent.to_string()
    }
}

/* -------------------------------------------------------------------------- */
/*                            Struct: MockResolver                            */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
pub(self) struct MockResolver;

/* ---------------------------- Impl: TypeResolver -------------------------- */

#[cfg(test)]
impl TypeResolver for MockResolver {
    fn resolve(
        &self,
        _scope: &[String],
        _reference: &[String],
        _is_absolute: bool,
    ) -> Option<(String, TypeKind)> {
        None
    }
}

/* -------------------------------------------------------------------------- */
/*                              Fn: make_context                              */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
pub(self) fn make_context() -> LowerContext<'static, MockResolver> {
    use crate::core::DescriptorBuilder;
    use crate::core::PackageName;

    let resolver = Box::leak(Box::new(MockResolver));
    let desc = DescriptorBuilder::default()
        .package(PackageName::try_from(vec!["test".to_string()]).unwrap())
        .build()
        .unwrap();

    LowerContext {
        resolver,
        parent: desc,
    }
}
