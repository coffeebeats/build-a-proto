use crate::ast;
use crate::ir::{Encoding, Enum, NativeType, Variant, WireFormat};

use super::{Lower, LowerContext};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* ---------------------------- Struct: ast::Enum --------------------------- */

impl<'a> Lower<'a, Enum> for ast::Enum {
    fn lower(&'a self, ctx: &'a LowerContext) -> Option<Enum> {
        let name = self.name.name.clone();
        let descriptor = ctx.build_child_descriptor(&name);

        // Create child context for variants
        let child_ctx = ctx.with_child(name.clone());

        // Lower variants
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

        // Compute discriminant encoding based on variant count
        // Need enough bits to represent all variant indices
        let variant_count = variants.len().max(2); // At least 2 values (0 and 1)
        let bits_needed = (usize::BITS - (variant_count - 1).leading_zeros()) as u8;

        // Round up to nearest power of 2 for standard sizes (8, 16, 32, 64)
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
            descriptor,
            name,
            discriminant,
            variants,
            doc,
        })
    }
}

/* ------------------------ Struct: ast::UnitVariant ------------------------ */

impl<'a> Lower<'a, Variant> for ast::UnitVariant {
    fn lower(&'a self, ctx: &'a LowerContext) -> Option<Variant> {
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

impl<'a> Lower<'a, Variant, FieldVariantContext<'a>> for ast::Field {
    fn lower(&'a self, ctx: &'a FieldVariantContext<'a>) -> Option<Variant> {
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

/// Marker context to indicate we're lowering a field as an enum variant.
pub struct FieldVariantContext<'a>(pub &'a LowerContext<'a>);
