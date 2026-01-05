use crate::ast;
use crate::ir::{Transform, WireFormat};

/* -------------------------------------------------------------------------- */
/*                            Impl: ast::Encoding                             */
/* -------------------------------------------------------------------------- */

impl ast::Encoding {
    /// Applies encoding transformations to a default wire format.
    pub fn apply_to_wire(&self, default_wire: &WireFormat) -> Option<(WireFormat, Vec<Transform>, Option<u64>)> {
        let mut wire = default_wire.clone();
        let mut transforms = Vec::new();
        let mut padding_bits = None;

        for enc_kind in &self.encodings {
            match enc_kind {
                ast::EncodingKind::Bits(bits) => {
                    wire = WireFormat::Bits { count: bits.value };
                }
                ast::EncodingKind::BitsVariable(max_bits) => {
                    // Variable-length encoding
                    // Compute prefix size needed to represent max_bits value
                    let prefix_bits = if max_bits.value == 0 {
                        8 // Minimum 8 bits for empty/small values
                    } else {
                        // Bits needed to represent the max_bits value
                        (64 - (max_bits.value as u64).leading_zeros()).max(8) as u8
                    };
                    wire = WireFormat::LengthPrefixed { prefix_bits };
                }
                ast::EncodingKind::ZigZag => {
                    transforms.push(Transform::ZigZag);
                }
                ast::EncodingKind::Delta => {
                    transforms.push(Transform::Delta);
                }
                ast::EncodingKind::FixedPoint(int_bits, frac_bits) => {
                    transforms.push(Transform::FixedPoint {
                        integer_bits: int_bits.value as u8,
                        fractional_bits: frac_bits.value as u8,
                    });
                }
                ast::EncodingKind::Pad(bits) => {
                    padding_bits = Some(bits.value);
                }
            }
        }

        Some((wire, transforms, padding_bits))
    }
}
