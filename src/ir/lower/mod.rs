mod comment;
mod encoding;
mod enumeration;
mod field;
mod message;
mod schema;
mod types;

use crate::ast;
use crate::core::Descriptor;

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

/* -------------------------- Struct: LowerContext -------------------------- */

/// `LowerContext` provides the environment needed for lowering AST to IR.
#[derive(Clone)]
pub struct LowerContext<'a, R: TypeResolver<TypeKind>> {
    /// Type resolver for looking up referenced types.
    pub resolver: &'a R,
    /// `scope` uniquely identifies the current scope.
    pub scope: Descriptor,
}

/* --------------------------- Impl: LowerContext --------------------------- */

impl<'a, R: TypeResolver<TypeKind>> LowerContext<'a, R> {
    /// `new` creates a new root context for lowering a top-level schema.
    pub fn new(resolver: &'a R, scope: Descriptor) -> Self {
        Self { resolver, scope }
    }

    /// `with` creates a child context by extending the parent descriptor.
    pub fn with<T>(&self, name: T) -> Self
    where
        T: AsRef<str>,
    {
        let mut path = self.scope.path.clone();
        path.push(name.as_ref().to_owned());

        Self {
            resolver: self.resolver,
            scope: Descriptor {
                package: self.scope.package.clone(),
                path,
            },
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                            Trait: TypeResolver                             */
/* -------------------------------------------------------------------------- */

/// `TypeResolver` provides symbol resolution for lowering references to IR.
///
/// Implementors are responsible for validating that references exist and
/// returning their resolved descriptor and associated data.
pub trait TypeResolver<T> {
    /// `resolve` looks up a type reference and returns its descriptor and data.
    ///
    /// - `scope`: The current scope (package + parent type path)
    /// - `reference`: The reference components as strings
    fn resolve(
        &self,
        scope: &Descriptor,
        reference: &ast::Reference,
    ) -> Option<(Descriptor, T)>;
}

/* ----------------------------- Enum: TypeKind ----------------------------- */

/// `TypeKind` represents the kind of a type definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Message,
    Enum,
    Package,
}

/* -------------------------------------------------------------------------- */
/*                            Struct: MockResolver                            */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
pub(super) struct MockResolver {
    pub result: Option<(Descriptor, TypeKind)>,
}

/* --------------------------- Impl: TypeResolver --------------------------- */

#[cfg(test)]
impl MockResolver {
    /// Creates a `MockResolver` that returns `None` for all resolutions.
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Creates a `MockResolver` that returns the specified result.
    pub fn with_result(result: (Descriptor, TypeKind)) -> Self {
        Self {
            result: Some(result),
        }
    }
}

#[cfg(test)]
impl TypeResolver<TypeKind> for MockResolver {
    fn resolve(
        &self,
        _scope: &Descriptor,
        _reference: &ast::Reference,
    ) -> Option<(Descriptor, TypeKind)> {
        self.result.clone()
    }
}

/* -------------------------------------------------------------------------- */
/*                              Fn: make_context                              */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
pub(super) fn make_context<'a>(resolver: &'a MockResolver) -> LowerContext<'a, MockResolver> {
    use crate::core::DescriptorBuilder;
    use crate::core::PackageName;

    let scope = DescriptorBuilder::default()
        .package(PackageName::try_from(vec!["test".to_string()]).unwrap())
        .build()
        .unwrap();

    LowerContext { resolver, scope }
}
