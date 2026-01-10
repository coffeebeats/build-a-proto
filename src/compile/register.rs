use chumsky::error::Rich;
use std::collections::HashMap;
use std::path::Path;

use crate::ast;
use crate::ast::Schema;
use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::ImportRoot;
use crate::core::PackageName;
use crate::core::SchemaImport;
use crate::lex::Span;
use crate::parse::ParseError;

use super::symbol::Symbols;

/* -------------------------------------------------------------------------- */
/*                               Struct: Context                              */
/* -------------------------------------------------------------------------- */

/// `Context` contains compilation context that preserves parsed AST
/// with spans for semantic validation while tracking type definitions in a
/// symbol table.
#[allow(unused)]
#[derive(Default)]
pub struct Context {
    /// `symbols` is a symbol table tracking all type definitions.
    pub symbols: Symbols,
    /// `source_files` are parsed ASTs per schema, preserving spans for
    /// later validation.
    pub source_files: HashMap<SchemaImport, Schema>,
}

/* -------------------------------------------------------------------------- */
/*                                Fn: register                                */
/* -------------------------------------------------------------------------- */

/// `register` registers types from a parsed AST into the compilation context.
///
/// This function:
/// 1. Extracts package name, includes, and type definitions from the AST
/// 2. Registers all type names into the symbol table ([`Symbols`])
/// 3. Stores the original parsed AST for later validation
#[allow(unused)]
pub fn register<'src>(
    _: &mut Context,
    _: &[ImportRoot],
    _: &SchemaImport,
    _: Schema,
) -> Result<(), ParseError<'src>> {
    todo!() // TODO: Replace this with new parsing implementation.
}

/* ----------------------- Fn: register_nested_types ------------------------ */

/// Recursively registers nested messages and enums within a message.
fn register_nested_types(_: &mut Symbols, _: &PackageName, _: &[&str], _: &ast::Message) {
    todo!() // TODO: Replace this with new parsing implementation.
}

/* ------------------------- Fn: build_descriptor --------------------------- */

/// Builds a Descriptor from package, path, and name components.
fn build_descriptor(package: &PackageName, path: &[&str], name: &str) -> Descriptor {
    DescriptorBuilder::default()
        .package(package.clone())
        .path(path.iter().map(|s| s.to_string()).collect())
        .name(name.to_owned())
        .build()
        .expect("descriptor should be valid")
}

/* ------------------------ Fn: resolve_include_path ------------------------ */

/// Resolves an include path by searching through import roots in order.
///
/// Returns a validated SchemaImport for the first matching .baproto file.
fn resolve_include_path<'a>(
    path: &Path,
    import_roots: &[ImportRoot],
    span: Span,
) -> Result<SchemaImport, ParseError<'a>> {
    import_roots
        .iter()
        .find_map(|root| root.resolve_schema_import(path).ok())
        .ok_or_else(|| {
            Rich::custom(
                span,
                format!(
                    "include path '{}' not found in any import root",
                    path.display(),
                ),
            )
        })
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {}
