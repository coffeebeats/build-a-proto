use derive_more::Display;
use itertools::Itertools;

use crate::ast;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                              Struct: Encoding                              */
/* -------------------------------------------------------------------------- */

/// `Encoding` represents encoding specifications for a field.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("[{}]", encodings.iter().join(","))]
pub struct Encoding {
    pub encodings: Vec<EncodingKind>,
    pub span: Span,
}

/* --------------------------- Enum: EncodingKind --------------------------- */

/// `EncodingKind` specifies how a field should be encoded in the wire format.
/// Note that the wrapped integer types should be later validated that they meet
/// the appropriate size limits.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum EncodingKind {
    /// Fixed-size bit encoding (must be less than size of integer type).
    #[display("bits({_0})")]
    Bits(ast::Uint),

    /// Variable-length bit encoding with a maximum size (must be less than
    /// size of integer type).
    #[display("bits(var({_0}))")]
    BitsVariable(ast::Uint),

    /// Delta encoding (difference from previous value).
    #[display("delta")]
    Delta,

    /// Fixed-point encoding with integer and fractional bits.
    #[display("fixed_point({_0},{_1})")]
    FixedPoint(ast::Uint, ast::Uint),

    /// Padding bits.
    #[display("pad({_0})")]
    Pad(ast::Uint),

    /// ZigZag encoding for signed integers.
    #[display("zig_zag")]
    ZigZag,
}
