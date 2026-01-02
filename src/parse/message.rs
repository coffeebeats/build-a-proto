use std::cell::RefCell;

use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Message                                */
/* -------------------------------------------------------------------------- */

/// `message` creates a new [`Parser`] that parses a message definition into an
/// [`ast::Message`].
pub(super) fn message<'src, I>(
    depth_limit: usize,
) -> impl Parser<'src, I, ast::Message, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let depth = RefCell::new(depth_limit);

    recursive(move |msg| {
        parse::comment_block()
            .or_not()
            .then(just(Token::Keyword(Keyword::Message)).ignore_then(parse::ident()))
            .then(
                choice((
                    msg.map(ast::MessageItem::Message),
                    parse::enumeration().map(ast::MessageItem::Enum),
                    field().map(ast::MessageItem::Field),
                    parse::comment_block().map(ast::MessageItem::CommentBlock),
                ))
                // FIXME: Handle newlines before, between, and after.
                .separated_by(just(Token::Newline).repeated())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<ast::MessageItem>>()
                .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
            )
            .then_ignore(just(Token::Newline).repeated())
            .map_with(|((comment, name), items), e| ast::Message {
                comment,
                items,
                name,
                span: e.span(),
            })
            .labelled("message")
            .try_map(move |m, span| {
                if let Ok(mut depth) = depth.try_borrow_mut() {
                    *depth -= 1;

                    if *depth <= 0 {
                        let msg = format!("exceeded maximum type depth limit: {}", 100);
                        return Err(Rich::custom(span, msg));
                    }
                }

                Ok(m)
            })
            .boxed()
    })
}

/* -------------------------------- Fn: field ------------------------------- */

/// `field` creates a new [`Parser`] that parses a message field into an
/// [`ast::Field`].
pub(super) fn field<'src, I>()
-> impl Parser<'src, I, ast::Field, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(field_index().or_not())
        .then(parse::typ())
        .then(parse::ident())
        .then(just(Token::Equal).ignore_then(parse::encoding()).or_not())
        .then_ignore(just(Token::Semicolon))
        .map_with(
            |((((comment, index), typ), name), encoding), e| ast::Field {
                comment,
                encoding,
                index,
                kind: typ,
                name,
                span: e.span(),
            },
        )
        .labelled("field")
        .boxed()
}

/* ----------------------------- Fn: field_index ---------------------------- */

/// `field_index` creates a new [`Parser`] that parses a field or variant index
/// into an [`ast::FieldIndex`].
pub(super) fn field_index<'src, I>()
-> impl Parser<'src, I, ast::FieldIndex, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::uint()
        .then_ignore(just(Token::Colon))
        .map_with(|value, e| ast::FieldIndex {
            span: e.span(),
            value: value,
        })
}
