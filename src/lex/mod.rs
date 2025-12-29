mod lexer;
mod token;

pub use chumsky::span::SimpleSpan as Span;
pub use chumsky::span::Spanned;

/* ------------------------------- Mod: Lexer ------------------------------- */

pub use lexer::LexError;
pub use lexer::lex;

/* ------------------------------- Mod: Token ------------------------------- */

pub use token::Keyword;
pub use token::Token;
