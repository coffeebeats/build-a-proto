//! Name validation pass.
//!
//! This pass validates that names are unique within their scope.

use std::collections::HashSet;

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::FilePass;
use crate::analyze::TypeKind;
use crate::core::SchemaImport;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                               Struct: Names                                */
/* -------------------------------------------------------------------------- */

/// Name validation pass.
///
/// Validates:
/// - No duplicate field names within a message
/// - No duplicate variant names within an enum
/// - No duplicate nested type names within a message
/// - No conflicts between field names and nested type names
pub struct Names;

/* ----------------------------- Impl: FilePass ----------------------------- */

impl FilePass for Names {
    fn run(&self, ctx: &mut Context, file: &SchemaImport) {
        let types_in_file: Vec<_> = ctx
            .symbols
            .iter_types()
            .filter(|(_, entry)| &entry.source == file)
            .map(|(_, entry)| entry.clone())
            .collect();

        for entry in types_in_file {
            match &entry.kind {
                TypeKind::Message { fields, nested } => {
                    let mut seen_field_names = HashSet::new();
                    let mut seen_nested_names = HashSet::new();

                    for field in fields {
                        if !seen_field_names.insert(field.name.clone()) {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: field.span,
                                kind: ErrorKind::DuplicateFieldName(field.name.clone()),
                            });
                        }
                    }

                    for nested_desc in nested {
                        let name = nested_desc.name.clone().unwrap_or_default();

                        if !seen_nested_names.insert(name.clone()) {
                            let span = ctx
                                .symbols
                                .get_type(nested_desc)
                                .map(|e| e.span)
                                .unwrap_or_else(|| Span::from(0..0));

                            ctx.add_error(Error {
                                file: file.clone(),
                                span,
                                kind: ErrorKind::DuplicateNestedTypeName(name.clone()),
                            });
                        }

                        if seen_field_names.contains(&name) {
                            let span = ctx
                                .symbols
                                .get_type(nested_desc)
                                .map(|e| e.span)
                                .unwrap_or_else(|| Span::from(0..0));

                            ctx.add_error(Error {
                                file: file.clone(),
                                span,
                                kind: ErrorKind::FieldTypeNameConflict(name),
                            });
                        }
                    }
                }
                TypeKind::Enum { variants } => {
                    let mut seen_names = HashSet::new();

                    for variant in variants {
                        if !seen_names.insert(variant.name.clone()) {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: variant.span,
                                kind: ErrorKind::DuplicateVariantName(variant.name.clone()),
                            });
                        }
                    }
                }
            }
        }
    }
}