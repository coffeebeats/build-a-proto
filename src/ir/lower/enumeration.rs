use super::TypeKind;

use crate::ast;
use crate::ir::{Encoding, Enum, NativeType, Variant, WireFormat};

use super::{Lower, LowerContext, TypeResolver};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* ---------------------------- Struct: ast::Enum --------------------------- */

impl<'a, R: TypeResolver<TypeKind>> Lower<'a, Enum, LowerContext<'a, R>> for ast::Enum {
    fn lower(&'a self, ctx: &'a LowerContext<'a, R>) -> Option<Enum> {
        let name = self.name.name.to_string();
        let child_ctx = ctx.with(&name);

        let mut variants = Vec::new();
        for item in &self.items {
            match item {
                ast::EnumItem::UnitVariant(uv) => {
                    if let Some(variant) = uv.lower(&child_ctx) {
                        variants.push(variant);
                    }
                }
                ast::EnumItem::FieldVariant(field) => {
                    if let Some(variant) = field.lower(&FieldVariantContext(&child_ctx)) {
                        variants.push(variant);
                    }
                }
                _ => {}
            }
        }

        let variant_count = variants.len().max(2); // At least 2 values (0 and 1)
        let bits_needed = (usize::BITS - (variant_count - 1).leading_zeros()) as u8;

        let bits = if bits_needed <= 8 {
            8
        } else if bits_needed <= 16 {
            16
        } else if bits_needed <= 32 {
            32
        } else {
            64
        };

        let discriminant = Encoding {
            wire: WireFormat::Bits { count: bits as u64 },
            native: NativeType::Int {
                bits,
                signed: false,
            },
            transforms: vec![],
            padding_bits: None,
        };

        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx));

        Some(Enum {
            descriptor: child_ctx.scope,
            discriminant,
            doc,
            variants,
        })
    }
}

/* ------------------------ Struct: ast::UnitVariant ------------------------ */

impl<'a, R: TypeResolver<TypeKind>> Lower<'a, Variant, LowerContext<'a, R>> for ast::UnitVariant {
    fn lower(&'a self, ctx: &'a LowerContext<'a, R>) -> Option<Variant> {
        let index = self.index.as_ref()?.value.value as u32;
        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx));

        Some(Variant::Unit {
            name: self.name.name.clone(),
            index,
            doc,
        })
    }
}

/* ------------------------ Struct: ast::FieldVariant ----------------------- */

impl<'a, R: TypeResolver<TypeKind>> Lower<'a, Variant, FieldVariantContext<'a, R>> for ast::Field {
    fn lower(&'a self, ctx: &'a FieldVariantContext<'a, R>) -> Option<Variant> {
        use crate::ir::Field;

        // Lower the field normally
        let ir_field: Field = Lower::lower(self, ctx.0)?;
        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx.0));

        Some(Variant::Field {
            name: self.name.name.clone(),
            index: ir_field.index,
            field: ir_field,
            doc,
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                       Struct: FieldVariantContext                          */
/* -------------------------------------------------------------------------- */

/// `FieldVariantContext` is a marker context to indicate we're lowering a field
/// as an enum variant.
pub struct FieldVariantContext<'a, R: TypeResolver<TypeKind>>(pub &'a LowerContext<'a, R>);

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

    /* ----------------------------- Tests: enum ---------------------------- */

    #[test]
    fn test_enum_discriminant_two_variants() {
        // Given: An enum with 2 variants.
        let enum_ast = make_enum("Status", vec!["Active", "Inactive"]);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should use 8-bit discriminant (smallest standard size).
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 8 }
        ));
        assert!(matches!(
            ir_enum.discriminant.native,
            NativeType::Int {
                bits: 8,
                signed: false
            }
        ));
    }

    #[test]
    fn test_enum_discriminant_255_variants() {
        // Given: An enum with 255 variants (max for 8 bits).
        let variants: Vec<_> = (0..255).map(|i| format!("Variant{}", i)).collect();
        let enum_ast = make_enum("Large", variants);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should use 8-bit discriminant.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 8 }
        ));
    }

    #[test]
    fn test_enum_discriminant_256_variants() {
        // Given: An enum with 256 variants (exactly fits in 8 bits: 0-255).
        let variants: Vec<_> = (0..256).map(|i| format!("Variant{}", i)).collect();
        let enum_ast = make_enum("Huge", variants);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should use 8-bit discriminant.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 8 }
        ));
    }

    #[test]
    fn test_enum_discriminant_257_variants() {
        // Given: An enum with 257 variants (needs 9 bits, rounds to 16).
        let variants: Vec<_> = (0..257).map(|i| format!("V{}", i)).collect();
        let enum_ast = make_enum("Massive", variants);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should use 16-bit discriminant.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 16 }
        ));
    }

    #[test]
    fn test_enum_discriminant_65536_variants() {
        // Given: An enum with 65536 variants (exactly fits in 16 bits).
        let variants: Vec<_> = (0..65536).map(|i| format!("V{}", i)).collect();
        let enum_ast = make_enum("Gigantic", variants);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should use 16-bit discriminant.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 16 }
        ));
    }

    #[test]
    fn test_enum_discriminant_65537_variants() {
        // Given: An enum with 65537 variants (needs 17 bits, rounds to 32).
        let variants: Vec<_> = (0..65537).map(|i| format!("V{}", i)).collect();
        let enum_ast = make_enum("Enormous", variants);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should use 32-bit discriminant.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 32 }
        ));
    }

    #[test]
    fn test_unit_variant_basic() {
        // Given: A simple unit variant with index.
        let variant = ast::UnitVariant {
            comment: None,
            name: ast::Ident {
                name: "Option1".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 0,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            span: Span::default(),
        };

        // When: Lowering the variant.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = variant.lower(&ctx);

        // Then: Should produce Unit variant with correct index.
        assert!(result.is_some());
        let ir_variant = result.unwrap();
        assert!(matches!(
            ir_variant,
            Variant::Unit {
                name: _,
                index: 0,
                doc: None
            }
        ));
    }

    #[test]
    fn test_unit_variant_missing_index() {
        // Given: A unit variant without an index.
        let variant = ast::UnitVariant {
            comment: None,
            name: ast::Ident {
                name: "NoIndex".to_string(),
                span: Span::default(),
            },
            index: None,
            span: Span::default(),
        };

        // When: Lowering the variant.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = variant.lower(&ctx);

        // Then: Should return None (index is required).
        assert!(result.is_none());
    }

    #[test]
    fn test_unit_variant_with_doc_comment() {
        // Given: A unit variant with documentation.
        let variant = ast::UnitVariant {
            comment: Some(ast::CommentBlock {
                comments: vec![ast::Comment {
                    content: "Primary option".to_string(),
                    span: Span::default(),
                }],
                span: Span::default(),
            }),
            name: ast::Ident {
                name: "Primary".to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: 1,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            span: Span::default(),
        };

        // When: Lowering the variant.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = variant.lower(&ctx);

        // Then: Should include documentation.
        assert!(result.is_some());
        if let Some(Variant::Unit { doc, .. }) = result {
            assert_eq!(doc, Some("Primary option".to_string()));
        } else {
            panic!("Expected Unit variant");
        }
    }

    #[test]
    fn test_enum_descriptor_generation() {
        // Given: An enum in a package context.
        let enum_ast = make_enum("MyEnum", vec!["A", "B"]);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should generate proper descriptor.
        assert!(result.is_some());
        let enm = result.unwrap();
        assert_eq!(enm.name(), Some("MyEnum"));
        assert!(enm.descriptor.to_string().contains("MyEnum"));
    }

    #[test]
    fn test_enum_preserves_variant_order() {
        // Given: An enum with specific variant order.
        let enum_ast = make_enum("Ordered", vec!["First", "Second", "Third"]);

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should preserve variant order.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert_eq!(ir_enum.variants.len(), 3);
        if let Variant::Unit { name, index, .. } = &ir_enum.variants[0] {
            assert_eq!(name, "First");
            assert_eq!(*index, 0);
        }
        if let Variant::Unit { name, index, .. } = &ir_enum.variants[1] {
            assert_eq!(name, "Second");
            assert_eq!(*index, 1);
        }
        if let Variant::Unit { name, index, .. } = &ir_enum.variants[2] {
            assert_eq!(name, "Third");
            assert_eq!(*index, 2);
        }
    }

    #[test]
    fn test_enum_with_doc_comment() {
        // Given: An enum with documentation.
        let enum_ast = ast::Enum {
            comment: Some(ast::CommentBlock {
                comments: vec![ast::Comment {
                    content: "Represents status codes".to_string(),
                    span: Span::default(),
                }],
                span: Span::default(),
            }),
            name: ast::Ident {
                name: "StatusCode".to_string(),
                span: Span::default(),
            },
            items: vec![ast::EnumItem::UnitVariant(ast::UnitVariant {
                comment: None,
                name: ast::Ident {
                    name: "Ok".to_string(),
                    span: Span::default(),
                },
                index: Some(ast::FieldIndex {
                    value: ast::Uint {
                        value: 0,
                        span: Span::default(),
                    },
                    span: Span::default(),
                }),
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should include documentation.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert_eq!(ir_enum.doc, Some("Represents status codes".to_string()));
    }

    #[test]
    fn test_enum_empty_variants() {
        // Given: An enum with no variants.
        let enum_ast = ast::Enum {
            comment: None,
            name: ast::Ident {
                name: "Empty".to_string(),
                span: Span::default(),
            },
            items: vec![],
            span: Span::default(),
        };

        // When: Lowering the enum.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = enum_ast.lower(&ctx);

        // Then: Should still produce valid enum with 8-bit discriminant.
        assert!(result.is_some());
        let ir_enum = result.unwrap();
        assert_eq!(ir_enum.variants.len(), 0);
        assert!(matches!(
            ir_enum.discriminant.wire,
            WireFormat::Bits { count: 8 }
        ));
    }

    /* ---------------------------- Fn: make_enum --------------------------- */

    fn make_enum(name: &str, variant_names: Vec<impl AsRef<str>>) -> ast::Enum {
        let items: Vec<_> = variant_names
            .iter()
            .enumerate()
            .map(|(i, v)| {
                ast::EnumItem::UnitVariant(ast::UnitVariant {
                    comment: None,
                    name: ast::Ident {
                        name: v.as_ref().to_string(),
                        span: Span::default(),
                    },
                    index: Some(ast::FieldIndex {
                        value: ast::Uint {
                            value: i as u64,
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    span: Span::default(),
                })
            })
            .collect();

        ast::Enum {
            comment: None,
            name: ast::Ident {
                name: name.to_string(),
                span: Span::default(),
            },
            items,
            span: Span::default(),
        }
    }
}
