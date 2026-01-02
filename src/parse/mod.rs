/* ------------------------------ Mod: Comment ------------------------------ */

pub(self) mod comment;
pub(self) use comment::*;

/* ------------------------------ Mod: Encoding ----------------------------- */

pub(self) mod encoding;
pub(self) use encoding::*;

/* ---------------------------- Mod: Enumeration ---------------------------- */

pub(self) mod enumeration;
pub(self) use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

pub(self) mod message;
pub(self) use message::*;

/* ------------------------------ Mod: Package ------------------------------ */

pub(self) mod package;
pub(self) use package::*;

/* ------------------------------- Mod: Schema ------------------------------ */

pub(self) mod schema;
pub(self) use schema::*;

/* ------------------------------- Mod: Types ------------------------------- */

pub(self) mod types;
pub(self) use types::*;

/* -------------------------------------------------------------------------- */
/*                                  Fn: Parse                                 */
/* -------------------------------------------------------------------------- */

use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;

pub const MAX_RECURSION_DEPTH: usize = 10;

/// `parse` parses an input [`Token`] sequence into an [`ast::Schema`].
pub fn parse<'src>(input: &'src Vec<Spanned<Token<'src>>>, size: usize) -> ParseResult<'src> {
    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        None
    });

    let (ast, errors) = missing
        .or(schema(MAX_RECURSION_DEPTH).map(Some))
        .parse(input.as_slice().map(Span::from(0..size), |spanned| {
            (&spanned.inner, &spanned.span)
        }))
        .into_output_errors();

    ParseResult {
        ast: ast.flatten(),
        errors,
    }
}

/* --------------------------- Struct: ParseResult -------------------------- */

/// `ParseResult` contains the result of parsing a `.baproto` file.
pub struct ParseResult<'src> {
    /// The parsed AST, if parsing succeeded (possibly with recovered errors).
    pub ast: Option<ast::Schema>,
    /// Errors encountered during parsing.
    pub errors: Vec<ParseError<'src>>,
}

/* ---------------------------- Type: ParseError ---------------------------- */

/// ParseError is a type alias for errors emitted during parsing.
pub type ParseError<'src> = Rich<'src, Token<'src>, Span>;

/* -------------------------------- Fn: text -------------------------------- */

/// `text` creates a new string literal [`Parser`].
pub(self) fn text<'src, I>()
-> impl Parser<'src, I, ast::Text, chumsky::extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let text = select! { Token::String(s) => s };
    text.map_with(|content, e| ast::Text {
        content: content.to_owned(),
        span: e.span(),
    })
}

/* -------------------------------- Fn: ident ------------------------------- */

/// `ident` creates a new identifier [`Parser`].
pub(self) fn ident<'src, I>()
-> impl Parser<'src, I, ast::Ident, chumsky::extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let id = select! { Token::Ident(id) => id };
    id.map_with(|id, e| ast::Ident {
        name: id.to_owned(),
        span: e.span(),
    })
}

/* -------------------------------- Fn: uint -------------------------------- */

/// `uint` creates a new unsigned integer [`Parser`].
pub(self) fn uint<'src, I>()
-> impl Parser<'src, I, ast::Uint, chumsky::extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let value = select! { Token::Uint(n) => n };
    value.map_with(|value, e| ast::Uint {
        value,
        span: e.span(),
    })
}
