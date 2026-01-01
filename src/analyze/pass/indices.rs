//! Index validation pass.
//!
//! This pass validates field and variant indices.

use std::collections::HashSet;

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::FilePass;
use crate::analyze::TypeKind;
use crate::core::SchemaImport;

/// Maximum field/variant index (2^29 - 1, matching protobuf).
const MAX_INDEX: u64 = (1 << 29) - 1;

/* -------------------------------------------------------------------------- */
/*                              Struct: Indices                               */
/* -------------------------------------------------------------------------- */

/// Index validation pass.
///
/// Validates:
/// - No duplicate field indices within a message
/// - No duplicate variant indices within an enum
/// - Indices are within range
pub struct Indices;

/* ----------------------------- Impl: FilePass ----------------------------- */

impl FilePass for Indices {
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
                    let mut seen_indices = HashSet::new();

                    for field in fields {
                        if !seen_indices.insert(field.index) {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: field.span,
                                kind: ErrorKind::DuplicateFieldIndex(field.index),
                            });
                        }

                        if field.index > MAX_INDEX {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: field.span,
                                kind: ErrorKind::IndexOutOfRange(field.index),
                            });
                        }
                    }
                }
                TypeKind::Enum { variants } => {
                    let mut seen_indices = HashSet::new();

                    for variant in variants {
                        if !seen_indices.insert(variant.index) {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: variant.span,
                                kind: ErrorKind::DuplicateVariantIndex(variant.index),
                            });
                        }

                        if variant.index > MAX_INDEX {
                            ctx.add_error(Error {
                                file: file.clone(),
                                span: variant.span,
                                kind: ErrorKind::IndexOutOfRange(variant.index),
                            });
                        }
                    }
                }
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::FieldEntry;
    use crate::analyze::ResolvedType;
    use crate::analyze::TypeEntry;
    use crate::analyze::VariantEntry;
    use crate::analyze::VariantKind;
    use crate::ast::ScalarType;
    use crate::core::Descriptor;
    use crate::core::DescriptorBuilder;
    use crate::core::PackageName;
    use crate::lex::Span;

    /* ------------------- Tests: Message field indices ------------------- */

    #[test]
    fn test_no_error_for_unique_field_indices() {
        // Given: A message with unique field indices.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage");

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![
                    make_field("field1", 1),
                    make_field("field2", 2),
                    make_field("field3", 3),
                ],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_duplicate_field_index() {
        // Given: A message with duplicate field indices.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage");

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![
                    make_field("field1", 1),
                    make_field("field2", 2),
                    make_field("field3", 2), // Duplicate index 2
                ],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: A duplicate field index error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::DuplicateFieldIndex(2)
        ));
    }

    #[test]
    fn test_detects_field_index_out_of_range() {
        // Given: A message with a field index exceeding the maximum.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage");

        let out_of_range_index = MAX_INDEX + 1;
        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![
                    make_field("field1", 1),
                    make_field("field2", out_of_range_index),
                ],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: An index out of range error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::IndexOutOfRange(_)
        ));
    }

    /* ------------------- Tests: Enum variant indices ------------------- */

    #[test]
    fn test_no_error_for_unique_variant_indices() {
        // Given: An enum with unique variant indices.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestEnum");

        let enm = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Enum {
                variants: vec![
                    make_variant("Variant1", 1),
                    make_variant("Variant2", 2),
                    make_variant("Variant3", 3),
                ],
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(enm);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_duplicate_variant_index() {
        // Given: An enum with duplicate variant indices.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestEnum");

        let enm = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Enum {
                variants: vec![
                    make_variant("Variant1", 1),
                    make_variant("Variant2", 2),
                    make_variant("Variant3", 2), // Duplicate index 2
                ],
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(enm);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: A duplicate variant index error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::DuplicateVariantIndex(2)
        ));
    }

    #[test]
    fn test_detects_variant_index_out_of_range() {
        // Given: An enum with a variant index exceeding the maximum.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestEnum");

        let out_of_range_index = MAX_INDEX + 1;
        let enm = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Enum {
                variants: vec![
                    make_variant("Variant1", 1),
                    make_variant("Variant2", out_of_range_index),
                ],
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(enm);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: An index out of range error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::IndexOutOfRange(_)
        ));
    }

    #[test]
    fn test_allows_maximum_valid_index() {
        // Given: A message with a field at the maximum valid index.
        let mut ctx = Context::default();
        let file = make_import();
        let descriptor = make_descriptor("TestMessage");

        let message = TypeEntry {
            descriptor: descriptor.clone(),
            kind: TypeKind::Message {
                fields: vec![make_field("field1", MAX_INDEX)],
                nested: Vec::new(),
            },
            span: Span::from(0..10),
            source: file.clone(),
        };

        ctx.symbols.register_type(message);

        // When: Running the indices validation pass.
        Indices.run(&mut ctx, &file);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    /* --------------------------- Helper functions --------------------------- */

    fn make_import() -> SchemaImport {
        let temp = tempfile::Builder::new()
            .suffix(".baproto")
            .tempfile()
            .unwrap();
        SchemaImport::try_from(temp.path().to_path_buf()).unwrap()
    }

    fn make_descriptor(name: &str) -> Descriptor {
        DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["test".to_string()]).unwrap())
            .path(Vec::new())
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

    fn make_variant(name: &str, index: u64) -> VariantEntry {
        VariantEntry {
            name: name.to_string(),
            index,
            kind: VariantKind::Unit,
            span: Span::from(0..10),
        }
    }
}