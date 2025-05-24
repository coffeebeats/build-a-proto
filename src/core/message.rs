use derive_builder::Builder;
use derive_more::Display;

use super::Descriptor;

/* -------------------------------------------------------------------------- */
/*                               Struct: Message                              */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Display, PartialEq)]
#[display("Message({name})")]
pub struct Message {
    pub comment: Vec<String>,
    #[builder(setter(into))]
    pub name: String,
    #[builder(default)]
    pub enums: Vec<Descriptor>,
    #[builder(default)]
    pub fields: Vec<Field>,
    #[builder(default)]
    pub messages: Vec<Descriptor>,
}

/* ------------------------------ Struct: Field ----------------------------- */

#[derive(Builder, Clone, Debug, PartialEq)]
pub struct Field {
    pub comment: Vec<String>,
    pub encoding: Option<Vec<Encoding>>,
    #[builder(default)]
    pub index: usize,
    #[builder(setter(into))]
    pub name: String,
    pub typ: Type,
}

/* --------------------- Impl: From<crate::parse::Field> -------------------- */

impl<'a> From<crate::parse::Field<'a>> for Field {
    fn from(value: crate::parse::Field<'a>) -> Self {
        FieldBuilder::default()
            .comment(
                value
                    .comment
                    .unwrap_or(vec![])
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .encoding(value.encoding)
            .name(value.name)
            .typ(Type::from(&value.typ))
            .build()
            .unwrap()
    }
}

/* ----------------------------- Enum: Encoding ----------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub enum Encoding {
    // Sizing
    Bits(usize),
    BitsVariable(usize),
    FixedPoint(usize, usize),

    // Encodings
    Delta,
    Pad(usize),
    ZigZag,
}

/* ------------------------------- Enum: Type ------------------------------- */

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Array(Box<Type>, Option<usize>),
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
                Type::Array(Box::new(Type::from(typ.as_ref())), size.clone())
            }
            crate::parse::Type::Map(k, v) => Type::Map(
                Box::new(Type::from(k.as_ref())),
                Box::new(Type::from(v.as_ref())),
            ),
        }
    }
}

/* ------------------------------ Enum: Scalar ------------------------------ */

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Bit,
    Bool,
    Byte,
    Float32,
    Float64,
    SignedInt16,
    SignedInt32,
    SignedInt64,
    SignedInt8,
    String,
    UnsignedInt16,
    UnsignedInt32,
    UnsignedInt64,
    UnsignedInt8,
}
