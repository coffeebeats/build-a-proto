use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                Fn: encoding                                */
/* -------------------------------------------------------------------------- */

/// `encoding` creates a new [`Parser`] that parses either a single encoding
/// definition or a list of them into an [`ast::Encoding`].
pub(super) fn encoding<'src, I>()
-> impl Parser<'src, I, ast::Encoding, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    // FIXME: Not all encoding kinds get spanned correctly?
    choice((
        // Single encoding
        parse::encoding_kind().map(|enc| vec![enc]),
        // Multiple encodings
        parse::encoding_kind()
            // FIXME: Handle no newlines case.
            .separated_by(just(Token::Comma).then(just(Token::Newline).repeated()))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(
                just(Token::ListOpen).then(just(Token::Newline).repeated()),
                just(Token::ListClose),
            ),
    ))
    .map_with(|encodings, e| ast::Encoding {
        encodings,
        span: e.span(),
    })
}

/* -------------------------------------------------------------------------- */
/*                              Fn: encoding_kind                             */
/* -------------------------------------------------------------------------- */

/// `encoding_kind` creates a new [`Parser`] that parses an encoding into an
/// [`ast::EncodingKind`].
pub(super) fn encoding_kind<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    choice((
        bits(),
        bits_variable(),
        fixed_point(),
        delta(),
        zigzag(),
        pad(),
    ))
    .labelled("encoding")
    .boxed()
}

/* -------------------------------- Fn: bits -------------------------------- */

fn bits<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("bits"))
        .ignore_then(parse::uint().delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(ast::EncodingKind::Bits)
}

/* ---------------------------- Fn: bits_variable --------------------------- */

fn bits_variable<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("bits"))
        .ignore_then(
            just(Token::Ident("var"))
                .ignore_then(parse::uint().delimited_by(just(Token::FnOpen), just(Token::FnClose)))
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(ast::EncodingKind::BitsVariable)
}

/* ----------------------------- Fn: fixed_point ---------------------------- */

fn fixed_point<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("fixed_point"))
        .ignore_then(
            parse::uint()
                .separated_by(just(Token::Comma))
                .exactly(2)
                .collect()
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(|args: Vec<ast::Uint>| ast::EncodingKind::FixedPoint(args[0], args[1]))
}

/* -------------------------------- Fn: delta ------------------------------- */

fn delta<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("delta")).map(|_| ast::EncodingKind::Delta)
}

/* ------------------------------- Fn: zigzag ------------------------------- */

fn zigzag<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("zigzag")).map(|_| ast::EncodingKind::ZigZag)
}

/* --------------------------------- Fn: pad -------------------------------- */

fn pad<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("pad"))
        .ignore_then(parse::uint().delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(ast::EncodingKind::Pad)
}
