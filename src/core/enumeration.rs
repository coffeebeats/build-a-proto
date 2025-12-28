use derive_builder::Builder;
use derive_more::Display;

use super::Field;

/* -------------------------------------------------------------------------- */
/*                                Struct: Enum                                */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Display, PartialEq)]
#[display(
    "Enum({name}): [ {} ]",
    self.variants
        .iter()
        .map(VariantKind::to_string)
        .collect::<Vec<_>>()
        .join(","),
)]
pub struct Enum {
    pub comment: Vec<String>,
    #[builder(setter(into))]
    pub name: String,
    #[builder(default)]
    pub path: String,
    pub variants: Vec<VariantKind>,
}

/* ----------------------------- Struct: Variant ---------------------------- */

#[derive(Builder, Clone, Debug, Display, PartialEq)]
#[display("{}:{}", self.index, self.name)]
pub struct Variant {
    pub comment: Vec<String>,
    #[builder(default)]
    pub index: usize,
    #[builder(setter(into))]
    pub name: String,
}

/* ---------------------------- Enum: VariantKind --------------------------- */

#[derive(Clone, Debug, Display, PartialEq)]
pub enum VariantKind {
    Field(Field),
    Variant(Variant),
}
