use derive_more::Display;

use crate::syntax::Reference;

/* -------------------------------------------------------------------------- */
/*                                 Enum: Type                                 */
/* -------------------------------------------------------------------------- */

#[allow(dead_code)]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Type {
    #[display("[{}]{_0}", _1.map(|n| n.to_string()).unwrap_or("".to_owned()))]
    Array(Box<Type>, Option<usize>),
    #[display("[{_0}]{_1}")]
    Map(Box<Type>, Box<Type>),
    #[display("{_0}")]
    Reference(Reference),
    Scalar(Scalar),
}

/* -------------------------------------------------------------------------- */
/*                                Enum: Scalar                                */
/* -------------------------------------------------------------------------- */

#[allow(dead_code)]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Scalar {
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
    #[display("i16")]
    SignedInt16,
    #[display("i32")]
    SignedInt32,
    #[display("i64")]
    SignedInt64,
    #[display("i8")]
    SignedInt8,
    #[display("string")]
    String,
    #[display("u16")]
    UnsignedInt16,
    #[display("u32")]
    UnsignedInt32,
    #[display("u64")]
    UnsignedInt64,
    #[display("u8")]
    UnsignedInt8,
}
