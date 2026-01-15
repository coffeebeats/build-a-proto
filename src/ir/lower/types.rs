use super::TypeKind;

use crate::ast;
use crate::ir::{Encoding, NativeType, WireFormat};

use super::{Lower, TypeResolver, field::FieldTypeContext};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* ---------------------------- Struct: ast::Type --------------------------- */

impl<'a, 'b, R: TypeResolver<TypeKind>> Lower<'a, Encoding, FieldTypeContext<'a, 'b, R>>
    for ast::Type
{
    fn lower(&'a self, ctx: &'a FieldTypeContext<'a, 'b, R>) -> Option<Encoding> {
        match self {
            ast::Type::Scalar(scalar) => scalar.lower(ctx),
            ast::Type::Reference(ref_ty) => ref_ty.lower(ctx),
            ast::Type::Array(array) => array.lower(ctx),
            ast::Type::Map(map) => map.lower(ctx),
        }
    }
}

/* --------------------------- Struct: ast::Scalar -------------------------- */

impl<'a, 'b, R: TypeResolver<TypeKind>> Lower<'a, Encoding, FieldTypeContext<'a, 'b, R>>
    for ast::Scalar
{
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b, R>) -> Option<Encoding> {
        use ast::ScalarType::*;

        // Determine native type and default wire format
        let (native, default_wire) = match self.kind {
            Bool | Bit => (NativeType::Bool, WireFormat::Bits { count: 1 }),
            Byte | Uint8 => (
                NativeType::Int {
                    bits: 8,
                    signed: false,
                },
                WireFormat::Bits { count: 8 },
            ),
            Uint16 => (
                NativeType::Int {
                    bits: 16,
                    signed: false,
                },
                WireFormat::Bits { count: 16 },
            ),
            Uint32 => (
                NativeType::Int {
                    bits: 32,
                    signed: false,
                },
                WireFormat::Bits { count: 32 },
            ),
            Uint64 => (
                NativeType::Int {
                    bits: 64,
                    signed: false,
                },
                WireFormat::Bits { count: 64 },
            ),
            Int8 => (
                NativeType::Int {
                    bits: 8,
                    signed: true,
                },
                WireFormat::Bits { count: 8 },
            ),
            Int16 => (
                NativeType::Int {
                    bits: 16,
                    signed: true,
                },
                WireFormat::Bits { count: 16 },
            ),
            Int32 => (
                NativeType::Int {
                    bits: 32,
                    signed: true,
                },
                WireFormat::Bits { count: 32 },
            ),
            Int64 => (
                NativeType::Int {
                    bits: 64,
                    signed: true,
                },
                WireFormat::Bits { count: 64 },
            ),
            Float32 => (
                NativeType::Float { bits: 32 },
                WireFormat::Bits { count: 32 },
            ),
            Float64 => (
                NativeType::Float { bits: 64 },
                WireFormat::Bits { count: 64 },
            ),
            String => (
                NativeType::String,
                WireFormat::LengthPrefixed { prefix_bits: 32 },
            ),
        };

        // Apply encoding annotations if present
        let (wire, transforms, padding_bits) = if let Some(enc) = field_ctx.encoding {
            enc.apply_to_wire(&default_wire)?
        } else {
            (default_wire, vec![], None)
        };

        Some(Encoding {
            wire,
            native,
            transforms,
            padding_bits,
        })
    }
}

/* ------------------------- Struct: ast::Reference ------------------------- */

impl<'a, 'b, R: super::TypeResolver<TypeKind>> Lower<'a, Encoding, FieldTypeContext<'a, 'b, R>>
    for ast::Reference
{
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b, R>) -> Option<Encoding> {
        // Get current scope
        let scope = &field_ctx.ctx.scope;

        // Use TypeResolver to resolve the reference
        let (descriptor, kind) = field_ctx.ctx.resolver.resolve(&scope, self)?;

        // Map kind to NativeType
        let native = match kind {
            super::TypeKind::Message => NativeType::Message { descriptor },
            super::TypeKind::Enum => NativeType::Enum { descriptor },
            super::TypeKind::Package => return None, // Not a valid reference.
        };

        Some(Encoding {
            wire: WireFormat::Embedded,
            native,
            transforms: vec![],
            padding_bits: None,
        })
    }
}

/* --------------------------- Struct: ast::Array --------------------------- */

impl<'a, 'b, R: TypeResolver<TypeKind>> Lower<'a, Encoding, FieldTypeContext<'a, 'b, R>>
    for ast::Array
{
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b, R>) -> Option<Encoding> {
        // Lower element type (without encoding annotations)
        let element_ctx = FieldTypeContext {
            ctx: field_ctx.ctx,
            encoding: None,
        };
        let element = self.element.as_ref().lower(&element_ctx)?;

        // Apply encoding to the array itself if present
        let (wire, _, padding_bits) = if let Some(enc) = field_ctx.encoding {
            enc.apply_to_wire(&WireFormat::LengthPrefixed { prefix_bits: 32 })?
        } else {
            (WireFormat::LengthPrefixed { prefix_bits: 32 }, vec![], None)
        };

        Some(Encoding {
            wire,
            native: NativeType::Array {
                element: Box::new(element),
            },
            transforms: vec![],
            padding_bits,
        })
    }
}

/* ---------------------------- Struct: ast::Map ---------------------------- */

impl<'a, 'b, R: TypeResolver<TypeKind>> Lower<'a, Encoding, FieldTypeContext<'a, 'b, R>>
    for ast::Map
{
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b, R>) -> Option<Encoding> {
        // Lower key and value types (without encoding annotations)
        let element_ctx = FieldTypeContext {
            ctx: field_ctx.ctx,
            encoding: None,
        };

        let key = self.key.as_ref().lower(&element_ctx)?;
        let value = self.value.as_ref().lower(&element_ctx)?;

        Some(Encoding {
            wire: WireFormat::LengthPrefixed { prefix_bits: 32 },
            native: NativeType::Map {
                key: Box::new(key),
                value: Box::new(value),
            },
            transforms: vec![],
            padding_bits: None,
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use crate::ast;
    use crate::core::{DescriptorBuilder, PackageName};
    use crate::ir::lower::{LowerContext, TypeKind};
    use crate::ir::lower::{MockResolver, make_context};
    use crate::lex::Span;

    use super::*;

    /* ---------------------------- Tests: scalar --------------------------- */

    #[test]
    fn test_scalar_bool_default() {
        // Given: A bool scalar type.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::Bool,
            span: Span::default(),
        };

        // When: Lowering without encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = scalar.lower(&ctx);

        // Then: Should use 1-bit wire format.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Bits { count: 1 }));
        assert!(matches!(encoding.native, NativeType::Bool));
        assert!(encoding.transforms.is_empty());
    }

    #[test]
    fn test_scalar_uint32_default() {
        // Given: A uint32 scalar type.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::Uint32,
            span: Span::default(),
        };

        // When: Lowering without encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = scalar.lower(&ctx);

        // Then: Should use 32-bit wire format.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Bits { count: 32 }));
        assert!(matches!(
            encoding.native,
            NativeType::Int {
                bits: 32,
                signed: false
            }
        ));
    }

    #[test]
    fn test_scalar_int64_default() {
        // Given: An int64 scalar type.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::Int64,
            span: Span::default(),
        };

        // When: Lowering without encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = scalar.lower(&ctx);

        // Then: Should use 64-bit signed wire format.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Bits { count: 64 }));
        assert!(matches!(
            encoding.native,
            NativeType::Int {
                bits: 64,
                signed: true
            }
        ));
    }

    #[test]
    fn test_scalar_float32_default() {
        // Given: A float32 scalar type.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::Float32,
            span: Span::default(),
        };

        // When: Lowering without encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = scalar.lower(&ctx);

        // Then: Should use 32-bit float wire format.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Bits { count: 32 }));
        assert!(matches!(encoding.native, NativeType::Float { bits: 32 }));
    }

    #[test]
    fn test_scalar_string_default() {
        // Given: A string scalar type.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::String,
            span: Span::default(),
        };

        // When: Lowering without encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = scalar.lower(&ctx);

        // Then: Should use length-prefixed wire format.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(
            encoding.wire,
            WireFormat::LengthPrefixed { prefix_bits: 32 }
        ));
        assert!(matches!(encoding.native, NativeType::String));
    }

    #[test]
    fn test_scalar_with_custom_bits() {
        // Given: A uint32 with custom 12-bit encoding.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::Uint32,
            span: Span::default(),
        };
        let encoding_annotation = Box::leak(Box::new(ast::Encoding {
            encodings: vec![ast::EncodingKind::Bits(ast::Uint {
                value: 12,
                span: Span::default(),
            })],
            span: Span::default(),
        }));

        // When: Lowering with custom encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: Some(encoding_annotation),
        };
        let result = scalar.lower(&ctx);

        // Then: Should use custom 12-bit wire format.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Bits { count: 12 }));
    }

    #[test]
    fn test_scalar_with_zigzag() {
        // Given: An int32 with zigzag encoding.
        let scalar = ast::Scalar {
            kind: ast::ScalarType::Int32,
            span: Span::default(),
        };
        let encoding_annotation = Box::leak(Box::new(ast::Encoding {
            encodings: vec![ast::EncodingKind::ZigZag],
            span: Span::default(),
        }));

        // When: Lowering with zigzag transform.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: Some(encoding_annotation),
        };
        let result = scalar.lower(&ctx);

        // Then: Should include ZigZag transform.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert_eq!(encoding.transforms.len(), 1);
        assert!(matches!(
            encoding.transforms[0],
            crate::ir::Transform::ZigZag
        ));
    }

    /* -------------------------- Tests: reference -------------------------- */

    #[test]
    fn test_reference_absolute_message() {
        // Given: An absolute reference to a message type.
        let reference = ast::Reference {
            is_absolute: true,
            components: vec![
                ast::Ident {
                    name: "foo".to_string(),
                    span: Span::default(),
                },
                ast::Ident {
                    name: "Bar".to_string(),
                    span: Span::default(),
                },
            ],
            span: Span::default(),
        };

        // When: Lowering with resolver that returns a message type.
        let pkg = PackageName::try_from(vec!["foo"]).unwrap();
        let descriptor = DescriptorBuilder::default()
            .package(pkg)
            .path(vec!["Bar".to_string()])
            .build()
            .unwrap();
        let resolver = Box::leak(Box::new(MockResolver {
            result: Some((descriptor.clone(), TypeKind::Message)),
        }));
        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["test"]).unwrap())
            .build()
            .unwrap();
        let lower_ctx = LowerContext { resolver, scope };
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = reference.lower(&ctx);

        // Then: Should resolve to message with descriptor.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Embedded));
        assert!(matches!(
            encoding.native,
            NativeType::Message { descriptor: _ }
        ));
        if let NativeType::Message { descriptor: got } = encoding.native {
            assert_eq!(descriptor, got);
        }
    }

    #[test]
    fn test_reference_relative_enum() {
        // Given: A relative reference to an enum type.
        let reference = ast::Reference {
            is_absolute: false,
            components: vec![ast::Ident {
                name: "Bar".to_string(),
                span: Span::default(),
            }],
            span: Span::default(),
        };

        // When: Lowering with resolver that returns an enum type.
        let pkg = PackageName::try_from(vec!["foo"]).unwrap();
        let descriptor = DescriptorBuilder::default()
            .package(pkg)
            .path(vec!["Bar".to_string()])
            .build()
            .unwrap();
        let resolver = Box::leak(Box::new(MockResolver {
            result: Some((descriptor.clone(), TypeKind::Enum)),
        }));
        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["test".to_string()]).unwrap())
            .build()
            .unwrap();
        let lower_ctx = LowerContext { resolver, scope };
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = reference.lower(&ctx);

        // Then: Should resolve to enum with descriptor.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::Embedded));
        assert!(matches!(
            encoding.native,
            NativeType::Enum { descriptor: _ }
        ));
        if let NativeType::Enum { descriptor: got } = encoding.native {
            assert_eq!(descriptor, got);
        }
    }

    #[test]
    fn test_reference_unresolved() {
        // Given: A reference that cannot be resolved.
        let reference = ast::Reference {
            is_absolute: false,
            components: vec![ast::Ident {
                name: "Unknown".to_string(),
                span: Span::default(),
            }],
            span: Span::default(),
        };

        // When: Lowering with resolver that returns None.
        let resolver = Box::leak(Box::new(MockResolver { result: None }));
        let scope = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["test".to_string()]).unwrap())
            .build()
            .unwrap();
        let lower_ctx = LowerContext { resolver, scope };
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = reference.lower(&ctx);

        // Then: Should return None.
        assert!(result.is_none());
    }

    /* ---------------------------- Tests: array ---------------------------- */

    #[test]
    fn test_array_of_scalars() {
        // Given: An array of uint32.
        let array = ast::Array {
            element: Box::new(ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint32,
                span: Span::default(),
            })),
            size: None,
            span: Span::default(),
        };

        // When: Lowering the array.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = array.lower(&ctx);

        // Then: Should use length-prefixed wire format with array native type.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(
            encoding.wire,
            WireFormat::LengthPrefixed { prefix_bits: 32 }
        ));
        assert!(matches!(encoding.native, NativeType::Array { .. }));
    }

    #[test]
    fn test_array_nested() {
        // Given: An array of arrays (2D array).
        let array = ast::Array {
            element: Box::new(ast::Type::Array(ast::Array {
                element: Box::new(ast::Type::Scalar(ast::Scalar {
                    kind: ast::ScalarType::Uint8,
                    span: Span::default(),
                })),
                size: None,
                span: Span::default(),
            })),
            size: None,
            span: Span::default(),
        };

        // When: Lowering the nested array.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = array.lower(&ctx);

        // Then: Should create nested array structure.
        assert!(result.is_some());
        let encoding = result.unwrap();
        if let NativeType::Array { element } = encoding.native {
            assert!(matches!(element.native, NativeType::Array { .. }));
        } else {
            panic!("Expected Array native type");
        }
    }

    #[test]
    fn test_array_with_custom_encoding() {
        // Given: An array with custom encoding annotation.
        let array = ast::Array {
            element: Box::new(ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint16,
                span: Span::default(),
            })),
            size: None,
            span: Span::default(),
        };
        let encoding_annotation = Box::leak(Box::new(ast::Encoding {
            encodings: vec![ast::EncodingKind::BitsVariable(ast::Uint {
                value: 255,
                span: Span::default(),
            })],
            span: Span::default(),
        }));

        // When: Lowering with custom encoding.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: Some(encoding_annotation),
        };
        let result = array.lower(&ctx);

        // Then: Should apply encoding to array (variable length prefix).
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(encoding.wire, WireFormat::LengthPrefixed { .. }));
    }

    /* ----------------------------- Tests: map ----------------------------- */

    #[test]
    fn test_map_string_to_uint() {
        // Given: A map from string to uint32.
        let map = ast::Map {
            key: Box::new(ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::String,
                span: Span::default(),
            })),
            value: Box::new(ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint32,
                span: Span::default(),
            })),
            span: Span::default(),
        };

        // When: Lowering the map.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = map.lower(&ctx);

        // Then: Should create map with key and value types.
        assert!(result.is_some());
        let encoding = result.unwrap();
        assert!(matches!(
            encoding.wire,
            WireFormat::LengthPrefixed { prefix_bits: 32 }
        ));
        if let NativeType::Map { key, value } = encoding.native {
            assert!(matches!(key.native, NativeType::String));
            assert!(matches!(
                value.native,
                NativeType::Int {
                    bits: 32,
                    signed: false
                }
            ));
        } else {
            panic!("Expected Map native type");
        }
    }

    #[test]
    fn test_map_uint_to_bool() {
        // Given: A map from uint8 to bool.
        let map = ast::Map {
            key: Box::new(ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint8,
                span: Span::default(),
            })),
            value: Box::new(ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Bool,
                span: Span::default(),
            })),
            span: Span::default(),
        };

        // When: Lowering the map.
        let resolver = MockResolver::new();
        let lower_ctx = make_context(&resolver);
        let ctx = FieldTypeContext {
            ctx: &lower_ctx,
            encoding: None,
        };
        let result = map.lower(&ctx);

        // Then: Should lower both key and value types correctly.
        assert!(result.is_some());
        let encoding = result.unwrap();
        if let NativeType::Map { key, value } = encoding.native {
            assert!(matches!(
                key.native,
                NativeType::Int {
                    bits: 8,
                    signed: false
                }
            ));
            assert!(matches!(value.native, NativeType::Bool));
        } else {
            panic!("Expected Map native type");
        }
    }
}
