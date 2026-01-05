mod lexer;
mod token;

use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;

use crate::core::SchemaImport;

pub type Span = chumsky::span::SimpleSpan<usize, SchemaImport>;
pub use chumsky::span::Spanned;

/* ------------------------------- Mod: Lexer ------------------------------- */

pub use lexer::lex;

/* ------------------------------- Mod: Token ------------------------------- */

pub use token::Keyword;
pub use token::Token;

/* ------------------------------ Fn: spanned ------------------------------- */

/// Helper function to wrap any value with a span from the parser context.
/// Extracts the [`SchemaImport`] from parser state and creates a contextual
/// span.
pub fn spanned<'src, T, I, E>(value: T, info: &mut MapExtra<'src, '_, I, E>) -> Spanned<T, Span>
where
    I: chumsky::input::Input<'src, Span = Span>,
    E: ParserExtra<'src, I, Context = SchemaImport>,
{
    Spanned {
        inner: value,
        span: info.span(),
    }
}
