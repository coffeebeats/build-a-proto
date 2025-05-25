use derive_more::Display;

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
    Reference(String),
    Scalar(Scalar),
}

/* --------------------- Impl: From<crate::parse::Type> --------------------- */

impl<'a> From<&crate::parse::Type<'a>> for Type {
    fn from(value: &crate::parse::Type<'a>) -> Self {
        match value {
            crate::parse::Type::Reference(s) => Type::Reference((*s).to_owned()),
            crate::parse::Type::Bit => Type::Scalar(Scalar::Bit),
            crate::parse::Type::Bool => Type::Scalar(Scalar::Bool),
            crate::parse::Type::Byte => Type::Scalar(Scalar::Byte),
            crate::parse::Type::Float32 => Type::Scalar(Scalar::Float32),
            crate::parse::Type::Float64 => Type::Scalar(Scalar::Float64),
            crate::parse::Type::SignedInt16 => Type::Scalar(Scalar::SignedInt16),
            crate::parse::Type::SignedInt32 => Type::Scalar(Scalar::SignedInt32),
            crate::parse::Type::SignedInt64 => Type::Scalar(Scalar::SignedInt64),
            crate::parse::Type::SignedInt8 => Type::Scalar(Scalar::SignedInt8),
            crate::parse::Type::String => Type::Scalar(Scalar::String),
            crate::parse::Type::UnsignedInt16 => Type::Scalar(Scalar::UnsignedInt16),
            crate::parse::Type::UnsignedInt32 => Type::Scalar(Scalar::UnsignedInt32),
            crate::parse::Type::UnsignedInt64 => Type::Scalar(Scalar::UnsignedInt64),
            crate::parse::Type::UnsignedInt8 => Type::Scalar(Scalar::UnsignedInt8),
            crate::parse::Type::Array(typ, size) => {
                Type::Array(Box::new(Type::from(typ.as_ref())), *size)
            }
            crate::parse::Type::Map(k, v) => Type::Map(
                Box::new(Type::from(k.as_ref())),
                Box::new(Type::from(v.as_ref())),
            ),
        }
    }
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
