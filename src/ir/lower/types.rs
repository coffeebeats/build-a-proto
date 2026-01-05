use crate::ast;
use crate::compile::TypeKind;
use crate::ir::{Encoding, NativeType, WireFormat};

use super::{field::FieldTypeContext, Lower};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* ---------------------------- Struct: ast::Type --------------------------- */

impl<'a, 'b> Lower<'a, Encoding, FieldTypeContext<'a, 'b>> for ast::Type {
    fn lower(&'a self, ctx: &'a FieldTypeContext<'a, 'b>) -> Option<Encoding> {
        match self {
            ast::Type::Scalar(scalar) => scalar.lower(ctx),
            ast::Type::Reference(ref_ty) => ref_ty.lower(ctx),
            ast::Type::Array(array) => array.lower(ctx),
            ast::Type::Map(map) => map.lower(ctx),
        }
    }
}

/* --------------------------- Struct: ast::Scalar -------------------------- */

impl<'a, 'b> Lower<'a, Encoding, FieldTypeContext<'a, 'b>> for ast::Scalar {
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b>) -> Option<Encoding> {
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
            Float32 => (NativeType::Float { bits: 32 }, WireFormat::Bits { count: 32 }),
            Float64 => (NativeType::Float { bits: 64 }, WireFormat::Bits { count: 64 }),
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

impl<'a, 'b> Lower<'a, Encoding, FieldTypeContext<'a, 'b>> for ast::Reference {
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b>) -> Option<Encoding> {
        // Build reference string from components
        let reference = self
            .components
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(".");

        // Use Symbols::resolve to resolve the reference
        let scope = field_ctx.ctx.scope();
        let package = &field_ctx.ctx.parent.package;
        let descriptor = field_ctx.ctx.symbols.resolve(package, &scope, &reference)?;
        let descriptor_str = descriptor.to_string();

        // Determine type kind
        let native = match field_ctx.ctx.symbols.get_type(&descriptor)? {
            TypeKind::Message => NativeType::Message {
                descriptor: descriptor_str,
            },
            TypeKind::Enum => NativeType::Enum {
                descriptor: descriptor_str,
            },
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

impl<'a, 'b> Lower<'a, Encoding, FieldTypeContext<'a, 'b>> for ast::Array {
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b>) -> Option<Encoding> {
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

impl<'a, 'b> Lower<'a, Encoding, FieldTypeContext<'a, 'b>> for ast::Map {
    fn lower(&'a self, field_ctx: &'a FieldTypeContext<'a, 'b>) -> Option<Encoding> {
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