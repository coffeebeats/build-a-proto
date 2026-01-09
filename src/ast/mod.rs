use derive_more::Display;

/* ------------------------------ Mod: Comment ------------------------------ */

mod comment;
pub use comment::*;

/* ------------------------------ Mod: Encoding ----------------------------- */

mod encoding;
pub use encoding::*;

/* ---------------------------- Mod: Enumeration ---------------------------- */

mod enumeration;
pub use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

mod message;
pub use message::*;

/* ------------------------------ Mod: Package ------------------------------ */

mod package;
pub use package::*;

/* ------------------------------- Mod: Schema ------------------------------ */

mod schema;
pub use schema::*;

/* ------------------------------- Mod: Types ------------------------------- */

mod types;
pub use types::*;

/* -------------------------------------------------------------------------- */
/*                                Struct: Ident                               */
/* -------------------------------------------------------------------------- */

/// `Ident` represents an identifier with its source location.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", name)]
pub struct Ident {
    pub name: String,
    pub span: crate::lex::Span,
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Text                                */
/* -------------------------------------------------------------------------- */

/// `Text` represents a single [`String`] literal with its source location.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("\"{}\"", content)]
pub struct Text {
    pub content: String,
    pub span: crate::lex::Span,
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Uint                                */
/* -------------------------------------------------------------------------- */

/// `Uint` represents an unsigned integer with its source location.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", value)]
pub struct Uint<T = u64> {
    pub value: T,
    pub span: crate::lex::Span,
}
