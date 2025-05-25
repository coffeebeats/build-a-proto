use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;
use chumsky::input::ValueInput;
use derive_builder::Builder;

use crate::core::Encoding;

use super::Span;
use super::Spanned;
use super::Token;

/* -------------------------------------------------------------------------- */
/*                                 Enum: Expr                                 */
/* -------------------------------------------------------------------------- */

/// `Expr` enumerates the set of potential expressions recognized by the
/// compiler.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr<'src> {
    Invalid(&'src [Token<'src>]),

    // Metadata
    Comment(&'src str),
    Include(&'src str),
    Package(&'src str),

    // Properties
    Field(Field<'src>),
    Variant(Variant<'src>),

    // Definitions
    Message(Message<'src>),
    Enum(Enum<'src>),
}

/* ----------------------------- Impl: with_span ---------------------------- */

impl<'src> Expr<'src> {
    /// `with_span`` is a convenience method for creating a [`Spanned`] item
    /// from the provided [`chumsky::MapExtra`] details.
    pub(super) fn with_span<I, E>(self, info: &mut MapExtra<'src, '_, I, E>) -> Spanned<Expr<'src>>
    where
        I: ValueInput<'src, Token = Token<'src>, Span = Span>,
        E: ParserExtra<'src, I>,
    {
        (self, info.span())
    }
}

/* --------------------------- Impl: From<Message> -------------------------- */

impl<'src> From<Message<'src>> for Expr<'src> {
    fn from(msg: Message<'src>) -> Self {
        Expr::Message(msg)
    }
}

/* ---------------------------- Impl: From<Enum> ---------------------------- */

impl<'src> From<Enum<'src>> for Expr<'src> {
    fn from(enm: Enum<'src>) -> Self {
        Expr::Enum(enm)
    }
}

/* ---------------------------- Impl: From<Field> --------------------------- */

impl<'src> From<Field<'src>> for Expr<'src> {
    fn from(f: Field<'src>) -> Self {
        Expr::Field(f)
    }
}

/* --------------------------- Impl: From<Variant> -------------------------- */

impl<'src> From<Variant<'src>> for Expr<'src> {
    fn from(v: Variant<'src>) -> Self {
        Expr::Variant(v)
    }
}

/* ----------------------------- Struct: Message ---------------------------- */

#[derive(Builder, Clone, Debug, PartialEq)]
pub struct Message<'src> {
    #[builder(default, setter(strip_option))]
    pub comment: Option<Vec<&'src str>>,
    #[builder(default)]
    pub enums: Vec<Enum<'src>>,
    #[builder(default)]
    pub fields: Vec<Field<'src>>,
    #[builder(default)]
    pub messages: Vec<Message<'src>>,
    pub name: &'src str,
}

/* ------------------------------ Struct: Enum ------------------------------ */

#[derive(Builder, Clone, Debug, PartialEq)]
pub struct Enum<'src> {
    #[builder(default, setter(strip_option))]
    pub comment: Option<Vec<&'src str>>,
    pub name: &'src str,
    #[builder(default)]
    pub variants: Vec<VariantKind<'src>>,
}

/* ------------------------------ Struct: Field ----------------------------- */

#[derive(Builder, Clone, Debug, PartialEq)]
pub struct Field<'src> {
    #[builder(default, setter(strip_option))]
    pub comment: Option<Vec<&'src str>>,
    #[builder(default, setter(strip_option))]
    pub encoding: Option<Vec<Encoding>>,
    #[builder(default, setter(strip_option))]
    pub index: Option<usize>,
    pub name: &'src str,
    pub typ: Type<'src>,
}

/* ----------------------------- Struct: Variant ---------------------------- */

#[derive(Builder, Clone, Debug, PartialEq)]
pub struct Variant<'src> {
    #[builder(setter(strip_option))]
    pub comment: Option<Vec<&'src str>>,
    #[builder(default, setter(strip_option))]
    pub index: Option<usize>,
    pub name: &'src str,
}

/* ---------------------------- Enum: VariantKind --------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub enum VariantKind<'src> {
    Field(Field<'src>),
    Variant(Variant<'src>),
}

/* ------------------------------- Enum: Type ------------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub enum Type<'src> {
    Reference(&'src str),

    // Scalars
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

    // Containers
    Array(Box<Type<'src>>, Option<usize>),
    Map(Box<Type<'src>>, Box<Type<'src>>),
}
