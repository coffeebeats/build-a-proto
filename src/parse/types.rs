use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                   Fn: typ                                  */
/* -------------------------------------------------------------------------- */

/// `typ` creates a new [`Parser`] that parses a type declaration into an
/// [`ast::Type`].
pub(super) fn typ<'src, I>()
-> impl Parser<'src, I, ast::Type, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    choice((
        array().map(ast::Type::Array),
        map().map(ast::Type::Map),
        reference().map(ast::Type::Reference),
        scalar().map(ast::Type::Scalar),
    ))
    .labelled("type")
    .boxed()
}

/* -------------------------------- Fn: array ------------------------------- */

/// `array` creates a new [`Parser`] that parses an array type declaration into
/// an [`ast::Array`].
pub(super) fn array<'src, I>()
-> impl Parser<'src, I, ast::Array, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::uint()
        .or_not()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar())
        .map_with(|(size, el), e| ast::Array {
            element: Box::new(ast::Type::Scalar(el)),
            size,
            span: e.span(),
        })
}

/* --------------------------------- Fn: map -------------------------------- */

/// `map` creates a new [`Parser`] that parses an map type declaration into a
/// [`ast::Map`].
pub(super) fn map<'src, I>()
-> impl Parser<'src, I, ast::Map, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::scalar()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(parse::scalar())
        .map_with(|(k, v), e| ast::Map {
            key: Box::new(ast::Type::Scalar(k)),
            value: Box::new(ast::Type::Scalar(v)),
            span: e.span(),
        })
}

/* ------------------------------ Fn: reference ----------------------------- */

/// `reference` creates a new [`Parser`] that parses a reference to another
/// named type into an [`ast::Reference`].
pub(super) fn reference<'src, I>()
-> impl Parser<'src, I, ast::Reference, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Dot)
        .or_not()
        .then(
            parse::ident()
                .separated_by(just(Token::Dot))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map_with(|(leading_dot, components), e| ast::Reference {
            components,
            is_absolute: leading_dot.is_some(),
            span: e.span(),
        })
}

/* ------------------------------- Fn: scalar ------------------------------- */

/// `scalar` creates a new [`Parser`] that parses a scalar type declaration into
/// an [`ast::Scalar`].
pub(super) fn scalar<'src, I>()
-> impl Parser<'src, I, ast::Scalar, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    select! {
        Token::Ident("bit") => ast::ScalarType::Bit,
        Token::Ident("bool") => ast::ScalarType::Bool,
        Token::Ident("byte") => ast::ScalarType::Byte,
        Token::Ident("u8") => ast::ScalarType::Uint8,
        Token::Ident("u16") => ast::ScalarType::Uint16,
        Token::Ident("u32") => ast::ScalarType::Uint32,
        Token::Ident("u64") => ast::ScalarType::Uint64,
        Token::Ident("i8") => ast::ScalarType::Int8,
        Token::Ident("i16") => ast::ScalarType::Int16,
        Token::Ident("i32") => ast::ScalarType::Int32,
        Token::Ident("i64") => ast::ScalarType::Int64,
        Token::Ident("f32") => ast::ScalarType::Float32,
        Token::Ident("f64") => ast::ScalarType::Float64,
        Token::Ident("string") => ast::ScalarType::String,
    }
    .map_with(|kind, e| ast::Scalar {
        kind,
        span: e.span(),
    })
}
