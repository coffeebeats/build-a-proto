use derive_builder::Builder;
use derive_more::Display;

use super::Field;

/* -------------------------------------------------------------------------- */
/*                                Struct: Enum                                */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Display, PartialEq)]
#[display("Enum({name})")]
pub struct Enum {
    pub comment: Vec<String>,
    #[builder(setter(into))]
    pub name: String,
    #[builder(default)]
    pub path: String,
    pub variants: Vec<VariantKind>,
}

/* ----------------------------- Struct: Variant ---------------------------- */

#[derive(Builder, Clone, Debug, PartialEq)]
pub struct Variant {
    pub comment: Vec<String>,
    #[builder(default)]
    pub index: usize,
    #[builder(setter(into))]
    pub name: String,
}

/* -------------------- Impl: From<crate::parse::Variant> ------------------- */

impl<'a> From<crate::parse::Variant<'a>> for Variant {
    fn from(value: crate::parse::Variant<'a>) -> Self {
        VariantBuilder::default()
            .comment(
                value
                    .comment
                    .unwrap_or(vec![])
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .name(value.name)
            .build()
            .unwrap()
    }
}

/* ---------------------------- Enum: VariantKind --------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub enum VariantKind {
    Field(Field),
    Variant(Variant),
}

/* ------------------ Impl: From<crate::parse::VariantKind> ----------------- */

impl<'a> From<crate::parse::VariantKind<'a>> for VariantKind {
    fn from(value: crate::parse::VariantKind<'a>) -> Self {
        match value {
            crate::parse::VariantKind::Field(f) => VariantKind::Field(Field::from(f)),
            crate::parse::VariantKind::Variant(v) => VariantKind::Variant(Variant::from(v)),
        }
    }
}
