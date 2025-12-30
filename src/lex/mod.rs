mod lexer;
mod token;

use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;

pub use chumsky::span::SimpleSpan as Span;
pub use chumsky::span::Spanned;

/* ------------------------------- Mod: Lexer ------------------------------- */

pub use lexer::lex;

/* ------------------------------- Mod: Token ------------------------------- */

pub use token::Keyword;
pub use token::Token;

/* ------------------------------ Fn: spanned ------------------------------- */

/// Helper function to wrap any value with a span from the parser context. This
/// is a generic utility that works with any type and parser context.
pub fn spanned<'src, T, I, E>(value: T, info: &mut MapExtra<'src, '_, I, E>) -> Spanned<T, Span>
where
    I: chumsky::input::Input<'src, Span = Span>,
    E: ParserExtra<'src, I>,
{
    Spanned {
        inner: value,
        span: info.span(),
    }
}
