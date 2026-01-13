use crate::ast;
use crate::ir::{Transform, WireFormat};

/* -------------------------------------------------------------------------- */
/*                            Impl: ast::Encoding                             */
/* -------------------------------------------------------------------------- */

impl ast::Encoding {
    /// `apply_to_wire` applies encoding transformations to a default wire
    /// format.
    pub fn apply_to_wire(
        &self,
        default_wire: &WireFormat,
    ) -> Option<(WireFormat, Vec<Transform>, Option<u64>)> {
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
                        (64 - max_bits.value.leading_zeros()).max(8) as u8
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

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use crate::lex::Span;

    use super::*;

    /* --------------------------- Tests: encoding -------------------------- */

    #[test]
    fn test_encoding_bits_override() {
        // Given: A bits encoding annotation.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::Bits(ast::Uint {
                value: 12,
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Wire format should be overridden.
        assert!(result.is_some());
        let (wire, transforms, padding) = result.unwrap();
        assert!(matches!(wire, WireFormat::Bits { count: 12 }));
        assert!(transforms.is_empty());
        assert_eq!(padding, None);
    }

    #[test]
    fn test_encoding_zigzag_transform() {
        // Given: A zigzag encoding annotation.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::ZigZag],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: ZigZag transform should be added.
        assert!(result.is_some());
        let (wire, transforms, _) = result.unwrap();
        assert!(matches!(wire, WireFormat::Bits { count: 32 }));
        assert_eq!(transforms.len(), 1);
        assert!(matches!(transforms[0], Transform::ZigZag));
    }

    #[test]
    fn test_encoding_delta_transform() {
        // Given: A delta encoding annotation.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::Delta],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 16 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Delta transform should be added.
        assert!(result.is_some());
        let (_, transforms, _) = result.unwrap();
        assert_eq!(transforms.len(), 1);
        assert!(matches!(transforms[0], Transform::Delta));
    }

    #[test]
    fn test_encoding_fixedpoint_transform() {
        // Given: A fixed-point encoding annotation.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::FixedPoint(
                ast::Uint {
                    value: 16,
                    span: Span::default(),
                },
                ast::Uint {
                    value: 8,
                    span: Span::default(),
                },
            )],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: FixedPoint transform should be added with correct bits.
        assert!(result.is_some());
        let (_, transforms, _) = result.unwrap();
        assert_eq!(transforms.len(), 1);
        assert!(matches!(
            transforms[0],
            Transform::FixedPoint {
                integer_bits: 16,
                fractional_bits: 8
            }
        ));
    }

    #[test]
    fn test_encoding_padding() {
        // Given: A padding encoding annotation.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::Pad(ast::Uint {
                value: 7,
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 8 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Padding bits should be set.
        assert!(result.is_some());
        let (_, _, padding) = result.unwrap();
        assert_eq!(padding, Some(7));
    }

    #[test]
    fn test_encoding_combined_transforms() {
        // Given: Multiple encoding annotations including transforms.
        let encoding = ast::Encoding {
            encodings: vec![
                ast::EncodingKind::Bits(ast::Uint {
                    value: 16,
                    span: Span::default(),
                }),
                ast::EncodingKind::ZigZag,
                ast::EncodingKind::Delta,
            ],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Both wire format and transforms should be applied.
        assert!(result.is_some());
        let (wire, transforms, _) = result.unwrap();
        assert!(matches!(wire, WireFormat::Bits { count: 16 }));
        assert_eq!(transforms.len(), 2);
        assert!(matches!(transforms[0], Transform::ZigZag));
        assert!(matches!(transforms[1], Transform::Delta));
    }

    #[test]
    fn test_encoding_variable_bits_zero() {
        // Given: A variable bits encoding with zero max value.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::BitsVariable(ast::Uint {
                value: 0,
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Should use minimum 8-bit prefix.
        assert!(result.is_some());
        let (wire, _, _) = result.unwrap();
        assert!(matches!(
            wire,
            WireFormat::LengthPrefixed { prefix_bits: 8 }
        ));
    }

    #[test]
    fn test_encoding_variable_bits_small() {
        // Given: A variable bits encoding with small max value (127).
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::BitsVariable(ast::Uint {
                value: 127,
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Should use 8-bit prefix (ceil(log2(127)) = 7, max(7, 8) = 8).
        assert!(result.is_some());
        let (wire, _, _) = result.unwrap();
        assert!(matches!(
            wire,
            WireFormat::LengthPrefixed { prefix_bits: 8 }
        ));
    }

    #[test]
    fn test_encoding_variable_bits_large() {
        // Given: A variable bits encoding with large max value.
        let encoding = ast::Encoding {
            encodings: vec![ast::EncodingKind::BitsVariable(ast::Uint {
                value: 65535,
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Applying to a default wire format.
        let default_wire = WireFormat::Bits { count: 32 };
        let result = encoding.apply_to_wire(&default_wire);

        // Then: Should use 16-bit prefix (ceil(log2(65535)) = 16).
        assert!(result.is_some());
        let (wire, _, _) = result.unwrap();
        assert!(matches!(
            wire,
            WireFormat::LengthPrefixed { prefix_bits: 16 }
        ));
    }
}
