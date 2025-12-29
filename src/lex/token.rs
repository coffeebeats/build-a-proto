use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;
use derive_more::Display;

use super::Span;
use super::Spanned;

/* -------------------------------------------------------------------------- */
/*                                 Enum: Token                                */
/* -------------------------------------------------------------------------- */

/// `Token` enumerates the set of potential tokens recognized by the parser.
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Token<'src> {
    Invalid(&'src str),

    // Syntax
    BlockClose,
    BlockOpen,
    Colon,
    Comma,
    Dot,
    Equal,
    FnClose,
    FnOpen,
    Keyword(Keyword),
    ListClose,
    ListOpen,
    Semicolon,

    // Whitespace
    Newline,

    // Input
    Comment(&'src str),
    Ident(&'src str),
    String(&'src str),
    Uint(usize),
}

/* ------------------------------- Impl: Token ------------------------------ */

impl<'src> Token<'src> {
    /// `with_span` is a convenience method for creating a [`Spanned`] item from
    /// the provided [`chumsky::MapExtra`] details.
    pub(crate) fn with_span<E>(
        self,
        info: &mut MapExtra<'src, '_, &'src str, E>,
    ) -> Spanned<Token<'src>, Span>
    where
        E: ParserExtra<'src, &'src str>,
    {
        Spanned {
            inner: self,
            span: info.span(),
        }
    }
}

/* ------------------------------ Enum: Keyword ----------------------------- */

/// Keyword enumerates the language's reserved keywords.
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Keyword {
    Encoding,
    Enum,
    Include,
    Message,
    Package,
}
