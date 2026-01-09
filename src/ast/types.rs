use derive_more::Display;

use crate::ast;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                                 Enum: Type                                 */
/* -------------------------------------------------------------------------- */

/// `Type` is a parsed type with its source location.
#[allow(unused)]
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum Type {
    /// An array type with an optional fixed size.
    Array(Array),

    /// A map type with key and value types.
    Map(Map),

    /// A reference to another type (message or enum).
    Reference(Reference),

    /// A scalar (primitive) type.
    Scalar(Scalar),
}

/* -------------------------------------------------------------------------- */
/*                              Struct: Reference                             */
/* -------------------------------------------------------------------------- */

/// `Reference` defines a reference to another [`crate::ast::Type`],
/// [`crate::ast::Message`], or [`crate::ast::Enum`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}{}", if self.is_absolute { "." } else { "" }, itertools::join(components, "."))]
pub struct Reference {
    pub components: Vec<super::Ident>,
    pub is_absolute: bool,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Scalar                               */
/* -------------------------------------------------------------------------- */

/// `Scalar` represents a [`ScalarType`] definition.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", kind)]
pub struct Scalar {
    pub kind: ScalarType,
    pub span: Span,
}

/* ---------------------------- Enum: ScalarType ---------------------------- */

/// `ScalarType` enumerates the primitive types supported by the schema language.
#[allow(unused)]
#[derive(Clone, Debug, Display, Eq, PartialEq)]
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

/* -------------------------------------------------------------------------- */
/*                                Struct: Array                               */
/* -------------------------------------------------------------------------- */

/// `Array` represents an array type declaration.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("[{}]", element)]
pub struct Array {
    pub element: Box<Type>,
    pub size: Option<ast::Uint>,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                                 Struct: Map                                */
/* -------------------------------------------------------------------------- */

/// `Map` represents an array type declaration.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("[{}]{}", key, value)]
pub struct Map {
    pub key: Box<Type>,
    pub value: Box<Type>,
    pub span: Span,
}
