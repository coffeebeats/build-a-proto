//! Type validation pass.
//!
//! This pass validates type compatibility (e.g., map keys must be scalar).

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::FilePass;
use crate::analyze::ResolvedType;
use crate::analyze::TypeKind;
use crate::analyze::VariantKind;
use crate::core::SchemaImport;

/* -------------------------------------------------------------------------- */
/*                               Struct: Types                                */
/* -------------------------------------------------------------------------- */

/// Type validation pass.
///
/// Validates:
/// - Map keys are scalar types
/// - Array sizes are positive (if specified)
pub struct Types;

/* ----------------------------- Impl: FilePass ----------------------------- */

impl FilePass for Types {
    fn run(&self, ctx: &mut Context, file: &SchemaImport) {
        let types_in_file: Vec<_> = ctx
            .symbols
            .iter_types()
            .filter(|(_, entry)| &entry.source == file)
            .map(|(_, entry)| entry.clone())
            .collect();

        for entry in types_in_file {
            match &entry.kind {
                TypeKind::Message { fields, .. } => {
                    for field in fields {
                        if let ResolvedType::Map { key, .. } = &field.resolved_type {
                            if !is_scalar_type(key) {
                                ctx.add_error(Error {
                                    file: file.clone(),
                                    span: field.span,
                                    kind: ErrorKind::InvalidMapKeyType(format_type(key)),
                                });
                            }
                        }

                        if let ResolvedType::Array { size: Some(0), .. } = &field.resolved_type {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: field.span,
                                kind: ErrorKind::InvalidArraySize,
                            });
                        }
                    }
                }
                TypeKind::Enum { variants } => {
                    for variant in variants {
                        if let VariantKind::Field(field) = &variant.kind {
                            if let ResolvedType::Map { key, .. } = &field.resolved_type {
                                if !is_scalar_type(key) {
                                    ctx.add_error(Error {
                                        file: file.clone(),
                                        span: field.span,
                                        kind: ErrorKind::InvalidMapKeyType(format_type(key)),
                                    });
                                }
                            }

                            if let ResolvedType::Array { size: Some(0), .. } = &field.resolved_type
                            {
                                ctx.add_error(Error {
                                    file: file.clone(),
                                    span: field.span,
                                    kind: ErrorKind::InvalidArraySize,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

/* ---------------------------- Fn: is_scalar_type -------------------------- */

fn is_scalar_type(typ: &ResolvedType) -> bool {
    matches!(typ, ResolvedType::Scalar(_))
}

/* ------------------------------ Fn: format_type --------------------------- */

fn format_type(typ: &ResolvedType) -> String {
    match typ {
        ResolvedType::Scalar(s) => format!("{:?}", s),
        ResolvedType::Array { element, size } => match size {
            Some(n) => format!("[{}]{}", n, format_type(element)),
            None => format!("[]{}", format_type(element)),
        },
        ResolvedType::Map { key, value } => {
            format!("map[{}]{}", format_type(key), format_type(value))
        }
        ResolvedType::Named(desc) => desc.to_string(),
        ResolvedType::Unresolved(name) => format!("unresolved({})", name),
    }
}