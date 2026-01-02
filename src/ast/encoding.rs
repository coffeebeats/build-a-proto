use crate::ast;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                              Struct: Encoding                              */
/* -------------------------------------------------------------------------- */

/// `Encoding` represents encoding specifications for a field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Encoding {
    pub encodings: Vec<EncodingKind>,
    pub span: Span,
}

/* --------------------------- Enum: EncodingKind --------------------------- */

/// `EncodingKind` specifies how a field should be encoded in the wire format.
/// Note that the wrapped integer types should be later validated that they meet
/// the appropriate size limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncodingKind {
    /// Fixed-size bit encoding (must be less than size of integer type).
    Bits(ast::Uint),

    /// Variable-length bit encoding with a maximum size (must be less than
    /// size of integer type).
    BitsVariable(ast::Uint),

    /// Delta encoding (difference from previous value).
    Delta,

    /// Fixed-point encoding with integer and fractional bits.
    FixedPoint(ast::Uint, ast::Uint),

    /// Padding bits.
    Pad(ast::Uint),

    /// ZigZag encoding for signed integers.
    ZigZag,
}
