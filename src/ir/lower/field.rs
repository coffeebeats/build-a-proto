use crate::ast;
use crate::ir::Field;

use super::{Lower, LowerContext, TypeResolver};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* --------------------------- Struct: ast::Field --------------------------- */

impl<'a, R: TypeResolver> Lower<'a, Field, LowerContext<'a, R>> for ast::Field {
    fn lower(&'a self, ctx: &'a LowerContext<'a, R>) -> Option<Field> {
        let index = self.index.as_ref()?.value.value as u32;

        let encoding = self.kind.lower(&FieldTypeContext {
            ctx,
            encoding: self.encoding.as_ref(),
        })?;

        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx));

        Some(Field {
            name: self.name.name.clone(),
            index,
            encoding,
            doc,
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                        Struct: FieldTypeContext                            */
/* -------------------------------------------------------------------------- */

/// `FieldTypeContext` provides context for lowering field types, including
/// optional encoding annotations.
pub struct FieldTypeContext<'a, 'b, R: TypeResolver> {
    pub ctx: &'a LowerContext<'b, R>,
    pub encoding: Option<&'a ast::Encoding>,
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use crate::ast;
    use crate::ir::lower;
    use crate::ir::{NativeType, WireFormat};
    use crate::lex::Span;

    use super::*;

    /* ---------------------------- Tests: field ---------------------------- */

    #[test]
    fn test_field_basic_lowering() {
        // Given: A simple field with index and scalar type.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "health".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 1,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint32,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should produce field with correct index and encoding.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert_eq!(ir_field.name, "health");
        assert_eq!(ir_field.index, 1);
        assert!(matches!(
            ir_field.encoding.wire,
            WireFormat::Bits { count: 32 }
        ));
    }

    #[test]
    fn test_field_missing_index() {
        // Given: A field without an index.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "score".to_string(),
                span: Span::default(),
            },
            index: None,
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint64,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should return None (index is required).
        assert!(result.is_none());
    }

    #[test]
    fn test_field_with_encoding_annotation() {
        // Given: A field with custom encoding annotation.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "compressed".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 3,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Int32,
                span: Span::default(),
            }),
            encoding: Some(ast::Encoding {
                encodings: vec![
                    ast::EncodingKind::Bits(ast::Uint {
                        value: 16,
                        span: Span::default(),
                    }),
                    ast::EncodingKind::ZigZag,
                ],
                span: Span::default(),
            }),
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should apply encoding annotation.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert_eq!(ir_field.name, "compressed");
        assert!(matches!(
            ir_field.encoding.wire,
            WireFormat::Bits { count: 16 }
        ));
        assert_eq!(ir_field.encoding.transforms.len(), 1);
        assert!(matches!(
            ir_field.encoding.transforms[0],
            crate::ir::Transform::ZigZag
        ));
    }

    #[test]
    fn test_field_with_documentation() {
        // Given: A field with documentation comment.
        let field = ast::Field {
            comment: Some(ast::CommentBlock {
                comments: vec![ast::Comment {
                    content: "Player health points".to_string(),
                    span: Span::default(),
                }],
                span: Span::default(),
            }),
            name: ast::Ident {
                name: "hp".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 1,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint16,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should include documentation.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert_eq!(ir_field.doc, Some("Player health points".to_string()));
    }

    #[test]
    fn test_field_array_type() {
        // Given: A field with array type.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "scores".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 2,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Array(ast::Array {
                element: Box::new(ast::Type::Scalar(ast::Scalar {
                    kind: ast::ScalarType::Uint32,
                    span: Span::default(),
                })),
                size: None,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should lower array type correctly.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert_eq!(ir_field.name, "scores");
        assert!(matches!(
            ir_field.encoding.wire,
            WireFormat::LengthPrefixed { .. }
        ));
        assert!(matches!(ir_field.encoding.native, NativeType::Array { .. }));
    }

    #[test]
    fn test_field_string_type() {
        // Given: A field with string type.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "message".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 5,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::String,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should use length-prefixed wire format for strings.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert!(matches!(
            ir_field.encoding.wire,
            WireFormat::LengthPrefixed { prefix_bits: 32 }
        ));
        assert!(matches!(ir_field.encoding.native, NativeType::String));
    }

    #[test]
    fn test_field_bool_type() {
        // Given: A field with bool type.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "active".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 0,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Bool,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should use 1-bit wire format for bool.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert!(matches!(
            ir_field.encoding.wire,
            WireFormat::Bits { count: 1 }
        ));
        assert!(matches!(ir_field.encoding.native, NativeType::Bool));
    }

    #[test]
    fn test_field_with_padding() {
        // Given: A field with padding annotation.
        let field = ast::Field {
            comment: None,
            name: ast::Ident {
                name: "padded".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 7,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint8,
                span: Span::default(),
            }),
            encoding: Some(ast::Encoding {
                encodings: vec![ast::EncodingKind::Pad(ast::Uint {
                    value: 4,
                    span: Span::default(),
                })],
                span: Span::default(),
            }),
            span: Span::default(),
        };

        // When: Lowering the field.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = field.lower(&ctx);

        // Then: Should include padding bits.
        assert!(result.is_some());
        let ir_field = result.unwrap();
        assert_eq!(ir_field.encoding.padding_bits, Some(4));
    }
}
