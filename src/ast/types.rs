use derive_more::Display;

use crate::core::Reference;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                                Struct: Type                                */
/* -------------------------------------------------------------------------- */

/// `Type` is a parsed type with its source location.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                               Enum: TypeKind                               */
/* -------------------------------------------------------------------------- */

/// `TypeKind` enumerates the different kinds of types in the schema language.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    /// Placeholder to support error recovery during parsing.
    Invalid,

    /// A scalar (primitive) type.
    Scalar(ScalarType),

    /// A reference to another type (message or enum).
    Reference(Reference),

    /// An array type with an optional fixed size.
    Array {
        element: Box<Type>,
        size: Option<u64>,
    },

    /// A map type with key and value types.
    Map { key: Box<Type>, value: Box<Type> },
}

/* -------------------------------------------------------------------------- */
/*                              Enum: ScalarType                              */
/* -------------------------------------------------------------------------- */

/// `ScalarType` enumerates the primitive types supported by the schema language.
#[allow(unused)]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum ScalarType {
    #[display("bit")]
    Bit,
    #[display("bool")]
    Bool,
    #[display("byte")]
    Byte,
    #[display("f32")]
    Float32,
    #[display("f64")]
    Float64,
    #[display("i8")]
    Int8,
    #[display("i16")]
    Int16,
    #[display("i32")]
    Int32,
    #[display("i64")]
    Int64,
    #[display("string")]
    String,
    #[display("u8")]
    Uint8,
    #[display("u16")]
    Uint16,
    #[display("u32")]
    Uint32,
    #[display("u64")]
    Uint64,
}
