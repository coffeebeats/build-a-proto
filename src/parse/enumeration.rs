use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                               Fn: Enumeration                              */
/* -------------------------------------------------------------------------- */

/// `enumeration` creates a new [`Parser`] that parses an enum definition into
/// an [`ast::Enum`].
pub(super) fn enumeration<'src, I>()
-> impl Parser<'src, I, ast::Enum, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(just(Token::Keyword(Keyword::Enum)).ignore_then(parse::ident()))
        .then(
            choice((
                parse::field().map(ast::EnumItem::FieldVariant),
                unit_variant().map(ast::EnumItem::UnitVariant),
                parse::comment_block().map(ast::EnumItem::CommentBlock),
            ))
            // FIXME: Handle newlines before, between, and after.
            .separated_by(just(Token::Newline).repeated())
            .allow_leading()
            .allow_trailing()
            .collect::<Vec<ast::EnumItem>>()
            .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
        )
        .then_ignore(just(Token::Newline).repeated())
        .map_with(|((comment, name), items), e| ast::Enum {
            comment,
            items,
            name,
            span: e.span(),
        })
        .labelled("enum")
        .boxed()
}

/* ---------------------------- Fn: unit_variant ---------------------------- */

/// `unit_variant` creates a new [`Parser`] that parses a unit enum variant into
/// an [`ast::UnitVariant`].
fn unit_variant<'src, I>()
-> impl Parser<'src, I, ast::UnitVariant, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(parse::field_index().or_not())
        .then(parse::ident())
        .then_ignore(just(Token::Semicolon))
        .map_with(|((comment, index), name), e| ast::UnitVariant {
            comment,
            index,
            name,
            span: e.span(),
        })
        .labelled("unit variant")
        .boxed()
}
