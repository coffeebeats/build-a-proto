use derive_builder::Builder;
use derive_more::Display;

use super::Descriptor;
use super::Type;

/* -------------------------------------------------------------------------- */
/*                               Struct: Message                              */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Display, PartialEq)]
#[display(
    "Message({name}): [ {} ]",
    self.fields
        .iter()
        .map(Field::to_string)
        .collect::<Vec<_>>()
        .join(", "),
)]
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

#[derive(Builder, Clone, Debug, Display, PartialEq)]
#[display("{}:{}({})",self.index, self.name, self.typ)]
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
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .encoding(value.encoding)
            .name(value.name)
            .index(value.index.unwrap())
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
