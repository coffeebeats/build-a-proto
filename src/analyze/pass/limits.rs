//! Resource limits pass.
//!
//! This pass validates structural resource limits.

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::FilePass;
use crate::analyze::ResolvedType;
use crate::analyze::TypeKind;
use crate::core::SchemaImport;

/// Maximum fields per message.
const MAX_FIELDS: usize = 512;

/// Maximum variants per enum.
const MAX_VARIANTS: usize = 256;

/// Maximum nesting depth.
const MAX_DEPTH: usize = 32;

/// Maximum declared array size.
const MAX_ARRAY_SIZE: u64 = 65536;

/* -------------------------------------------------------------------------- */
/*                               Struct: Limits                               */
/* -------------------------------------------------------------------------- */

/// Resource limits pass.
///
/// Validates:
/// - Message field count <= 512
/// - Enum variant count <= 256
/// - Nesting depth <= 32
/// - Array declared size <= 65536
pub struct Limits;

/* ----------------------------- Impl: FilePass ----------------------------- */

impl FilePass for Limits {
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
                    if fields.len() > MAX_FIELDS {
                        ctx.add_error(Error {
                            file: file.clone(),
                            span: entry.span,
                            kind: ErrorKind::TooManyFields(fields.len(), MAX_FIELDS),
                        });
                    }

                    for field in fields {
                        validate_array_size(ctx, file, &field.resolved_type, field.span);
                    }

                    let depth = entry.descriptor.path.len() + 1;
                    if depth > MAX_DEPTH {
                        ctx.add_error(Error {
                            file: file.clone(),
                            span: entry.span,
                            kind: ErrorKind::NestingTooDeep(depth, MAX_DEPTH),
                        });
                    }
                }
                TypeKind::Enum { variants } => {
                    if variants.len() > MAX_VARIANTS {
                        ctx.add_error(Error {
                            file: file.clone(),
                            span: entry.span,
                            kind: ErrorKind::TooManyVariants(variants.len(), MAX_VARIANTS),
                        });
                    }
                }
            }
        }
    }
}

/* ------------------------- Fn: validate_array_size ------------------------ */

fn validate_array_size(
    ctx: &mut Context,
    file: &SchemaImport,
    typ: &ResolvedType,
    span: crate::lex::Span,
) {
    if let ResolvedType::Array { size: Some(size), element } = typ {
        if *size > MAX_ARRAY_SIZE {
            ctx.add_error(Error {
                file: file.clone(),
                span,
                kind: ErrorKind::ArraySizeTooLarge(*size, MAX_ARRAY_SIZE),
            });
        }

        validate_array_size(ctx, file, element, span);
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::FieldEntry;
    use crate::analyze::TypeEntry;
    use crate::analyze::VariantEntry;
    use crate::analyze::VariantKind;
    use crate::ast::ScalarType;
    use crate::core::Descriptor;
    use crate::core::DescriptorBuilder;
    use crate::core::PackageName;
    use crate::lex::Span;

    /* ------------------- Tests: Message field count limit ------------------- */

    #[test]
    fn test_no_error_for_message_within_field_limit() {
        // Given: A message with fields at the maximum limit.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage", vec![]);

        let fields: Vec<_> = (0..MAX_FIELDS)
            .map(|i| make_field(&format!("field{}", i), i as u64))
            .collect();

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields,
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_too_many_fields() {
        // Given: A message exceeding the maximum field count.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage", vec![]);

        let fields: Vec<_> = (0..=MAX_FIELDS)
            .map(|i| make_field(&format!("field{}", i), i as u64))
            .collect();

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields,
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: A too many fields error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::TooManyFields(_, MAX_FIELDS)
        ));
    }

    /* ------------------ Tests: Enum variant count limit ------------------ */

    #[test]
    fn test_no_error_for_enum_within_variant_limit() {
        // Given: An enum with variants at the maximum limit.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestEnum", vec![]);

        let variants: Vec<_> = (0..MAX_VARIANTS)
            .map(|i| make_variant(&format!("Variant{}", i), i as u64))
            .collect();

        let enm = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Enum { variants },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(enm);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_too_many_variants() {
        // Given: An enum exceeding the maximum variant count.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestEnum", vec![]);

        let variants: Vec<_> = (0..=MAX_VARIANTS)
            .map(|i| make_variant(&format!("Variant{}", i), i as u64))
            .collect();

        let enm = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Enum { variants },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(enm);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: A too many variants error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::TooManyVariants(_, MAX_VARIANTS)
        ));
    }

    /* -------------------- Tests: Nesting depth limit -------------------- */

    #[test]
    fn test_no_error_for_nesting_within_limit() {
        // Given: A message at maximum nesting depth.
        let mut ctx = Context::default();
        let file = make_import();

        // Create path with MAX_DEPTH - 1 elements, depth will be len + 1.
        let path: Vec<String> = (0..MAX_DEPTH - 1).map(|i| format!("Outer{}", i)).collect();
        let descriptor = make_descriptor("DeepMessage", path);

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![make_field("field1", 1)],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_nesting_too_deep() {
        // Given: A message exceeding maximum nesting depth.
        let mut ctx = Context::default();
        let file = make_import();

        // Create path with MAX_DEPTH elements, depth will be len + 1 = MAX_DEPTH + 1.
        let path: Vec<String> = (0..MAX_DEPTH).map(|i| format!("Outer{}", i)).collect();
        let descriptor = make_descriptor("TooDeepMessage", path);

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![make_field("field1", 1)],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: A nesting too deep error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::NestingTooDeep(_, MAX_DEPTH)
        ));
    }

    /* -------------------- Tests: Array size limit -------------------- */

    #[test]
    fn test_no_error_for_array_within_size_limit() {
        // Given: A message with an array at the maximum size.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage", vec![]);

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![make_array_field("arr", 1, MAX_ARRAY_SIZE)],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_array_size_too_large() {
        // Given: A message with an array exceeding the maximum size.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage", vec![]);

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![make_array_field("arr", 1, MAX_ARRAY_SIZE + 1)],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: An array size too large error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::ArraySizeTooLarge(_, MAX_ARRAY_SIZE)
        ));
    }

    #[test]
    fn test_detects_nested_array_size_too_large() {
        // Given: A message with a nested array exceeding the maximum size.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage", vec![]);

        let inner_array = ResolvedType::Array {
            element: Box::new(ResolvedType::Scalar(ScalarType::Int32)),
            size: Some(MAX_ARRAY_SIZE + 1),
        };
        let outer_array = ResolvedType::Array {
            element: Box::new(inner_array),
            size: Some(10),
        };

        let field = FieldEntry {
            name: "nested_arr".to_string(),
            index: 1,
            resolved_type: outer_array,
            encoding: None,
            span: Span::from(0..10),
        };

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![field],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the limits validation pass.
        Limits.run(&mut ctx, &file);

        // Then: An array size too large error should be reported for the nested array.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::ArraySizeTooLarge(_, MAX_ARRAY_SIZE)
        ));
    }

    /* --------------------------- Helper functions --------------------------- */

    fn make_import() -> SchemaImport {
        let temp = tempfile::Builder::new()
            .suffix(".baproto")
            .tempfile()
            .unwrap();
        SchemaImport::try_from(temp.path().to_path_buf()).unwrap()
    }

    fn make_descriptor(name: &str, path: Vec<String>) -> Descriptor {
        DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["test".to_string()]).unwrap())
            .path(path)
            .name(name.to_string())
            .build()
            .unwrap()
    }

    fn make_field(name: &str, index: u64) -> FieldEntry {
        FieldEntry {
            name: name.to_string(),
            index,
            resolved_type: ResolvedType::Scalar(ScalarType::Int32),
            encoding: None,
            span: Span::from(0..10),
        }
    }

    fn make_array_field(name: &str, index: u64, size: u64) -> FieldEntry {
        FieldEntry {
            name: name.to_string(),
            index,
            resolved_type: ResolvedType::Array {
                element: Box::new(ResolvedType::Scalar(ScalarType::Int32)),
                size: Some(size),
            },
            encoding: None,
            span: Span::from(0..10),
        }
    }

    fn make_variant(name: &str, index: u64) -> VariantEntry {
        VariantEntry {
            name: name.to_string(),
            index,
            kind: VariantKind::Unit,
            span: Span::from(0..10),
        }
    }
}