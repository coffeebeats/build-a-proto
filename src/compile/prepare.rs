//! DEPRECATED: This module will be replaced by `register.rs` as part of the
//! semantic validation refactoring.
//!
//! The new `register` phase:
//! - Preserves spans for better error reporting
//! - Separates type registration from type lowering
//! - Uses the `Symbols` table instead of directly building the `Registry`
//!
//! This module is kept temporarily until the command pipeline is updated (Step 6).

use chumsky::error::Rich;
use std::path::Path;

use crate::ast::Schema;
use crate::core::ImportRoot;
use crate::core::Registry;
use crate::core::SchemaImport;
use crate::lex::Span;
use crate::parse::ParseError;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Prepare                                */
/* -------------------------------------------------------------------------- */

pub fn prepare<'a>(
    _: &'a SchemaImport,
    _: &[ImportRoot],
    _: &'a mut Registry,
    _: &'a Schema,
) -> Result<(), ParseError<'a>> {
    todo!() // TODO: Replace this with new parsing implementation.
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
