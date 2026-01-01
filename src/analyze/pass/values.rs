//! Value validation pass.
//!
//! This pass validates encoding parameter values.

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::FilePass;
use crate::analyze::ResolvedType;
use crate::analyze::TypeKind;
use crate::analyze::VariantKind;
use crate::ast::Encoding;
use crate::ast::ScalarType;
use crate::core::SchemaImport;
use crate::lex::Span;

/// Maximum bits for Bits(n) encoding.
const MAX_BITS: u64 = 64;

/// Maximum bits for FixedPoint(i, f) encoding.
const MAX_FIXED_POINT_BITS: u64 = 64;

/// Maximum value for BitsVariable(max).
const MAX_BITS_VARIABLE: u64 = 64;

/* -------------------------------------------------------------------------- */
/*                               Struct: Values                               */
/* -------------------------------------------------------------------------- */

/// Value validation pass.
///
/// Validates:
/// - Bits(n): n <= type width and n <= 64
/// - BitsVariable(max): max <= 64
/// - FixedPoint(i, f): i + f <= 64
pub struct Values;

/* ----------------------------- Impl: FilePass ----------------------------- */

impl FilePass for Values {
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
                        if let Some(encodings) = &field.encoding {
                            for encoding in encodings {
                                validate_encoding(
                                    ctx,
                                    file,
                                    &field.resolved_type,
                                    encoding,
                                    field.span,
                                );
                            }
                        }
                    }
                }
                TypeKind::Enum { variants } => {
                    for variant in variants {
                        if let VariantKind::Field(field) = &variant.kind {
                            if let Some(encodings) = &field.encoding {
                                for encoding in encodings {
                                    validate_encoding(
                                        ctx,
                                        file,
                                        &field.resolved_type,
                                        encoding,
                                        field.span,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/* -------------------------- Fn: validate_encoding ------------------------- */

fn validate_encoding(
    ctx: &mut Context,
    file: &SchemaImport,
    typ: &ResolvedType,
    encoding: &Encoding,
    span: Span,
) {
    match encoding {
        Encoding::Bits(n) => {
            if *n > MAX_BITS {
                ctx.add_error(Error {
                    file: file.clone(),
                    span,
                    kind: ErrorKind::BitsExceedsLimit(*n),
                });
            }

            if let Some(type_width) = get_type_width(typ) {
                if *n > type_width {
                    ctx.add_error(Error {
                        file: file.clone(),
                        span,
                        kind: ErrorKind::BitsExceedsTypeWidth(*n, type_width),
                    });
                }
            }
        }

        Encoding::BitsVariable(max) => {
            if *max > MAX_BITS_VARIABLE {
                ctx.add_error(Error {
                    file: file.clone(),
                    span,
                    kind: ErrorKind::BitsVariableExceedsLimit(*max),
                });
            }
        }

        Encoding::FixedPoint(i, f) => {
            let total = i + f;
            if total > MAX_FIXED_POINT_BITS {
                ctx.add_error(Error {
                    file: file.clone(),
                    span,
                    kind: ErrorKind::FixedPointExceedsLimit(*i, *f, total),
                });
            }
        }

        Encoding::Delta | Encoding::ZigZag | Encoding::Pad(_) => {}
    }
}

/* --------------------------- Fn: get_type_width --------------------------- */

fn get_type_width(typ: &ResolvedType) -> Option<u64> {
    match typ {
        ResolvedType::Scalar(scalar) => match scalar {
            ScalarType::Bit => Some(1),
            ScalarType::Bool => Some(1),
            ScalarType::Byte | ScalarType::Uint8 | ScalarType::Int8 => Some(8),
            ScalarType::Uint16 | ScalarType::Int16 => Some(16),
            ScalarType::Uint32 | ScalarType::Int32 | ScalarType::Float32 => Some(32),
            ScalarType::Uint64 | ScalarType::Int64 | ScalarType::Float64 => Some(64),
            ScalarType::String => None, // Variable length
        },
        _ => None,
    }
}