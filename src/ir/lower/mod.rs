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

use crate::compile::Symbols;
use crate::core::Descriptor;

/* -------------------------------------------------------------------------- */
/*                                Trait: Lower                                */
/* -------------------------------------------------------------------------- */

/// `Lower` converts an AST node to its corresponding IR representation.
///
/// The trait is generic over the context type, allowing each AST node to
/// specify what context it needs. Most types use `LowerContext` (the default).
pub trait Lower<'a, T, Ctx = LowerContext<'a>> {
    /// Lowers this AST node to an IR node using the provided context.
    fn lower(&'a self, ctx: &'a Ctx) -> Option<T>;
}

/* -------------------------------------------------------------------------- */
/*                            Struct: LowerContext                            */
/* -------------------------------------------------------------------------- */

/// `LowerContext` provides the environment needed for lowering AST to IR.
#[derive(Clone)]
pub struct LowerContext<'a> {
    /// Symbol table with type information.
    pub symbols: &'a Symbols,
    /// Parent descriptor (defines the current scope).
    pub parent: Descriptor,
}

impl<'a> LowerContext<'a> {
    /// Creates a new root context for lowering a top-level schema.
    pub fn new(symbols: &'a Symbols, parent: Descriptor) -> Self {
        Self { symbols, parent }
    }

    /// Creates a child context by extending the parent descriptor.
    pub fn with_child(&self, name: String) -> Self {
        let mut path = self.parent.path.clone();
        if let Some(parent_name) = &self.parent.name {
            path.push(parent_name.clone());
        }

        Self {
            symbols: self.symbols,
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
