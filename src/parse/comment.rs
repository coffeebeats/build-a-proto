use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                              Fn: comment_block                             */
/* -------------------------------------------------------------------------- */

/// `comment_block` creates a new [`Parser`] that parses a contiguous group of
/// comments (i.e. comments delimited by [`Token::Newline`] tokens) into an
/// [`ast::CommentBlock`].
pub(super) fn comment_block<'src, I>()
-> impl Parser<'src, I, ast::CommentBlock, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    comment()
        .map(|c| vec![c])
        .foldl(
            just(Token::Newline).ignore_then(comment()).repeated(),
            |mut v, c| {
                v.push(c);
                v
            },
        )
        .then_ignore(just(Token::Newline))
        .map_with(|comments, e| ast::CommentBlock {
            comments,
            span: e.span(),
        })
        .labelled("doc comment")
}

/* -------------------------------------------------------------------------- */
/*                             Fn: inline_comment                             */
/* -------------------------------------------------------------------------- */

/// `inline_comment` creates a new [`Parser`] that parses an inline comment
/// (i.e. a comment that comes after [`Token`]s within a line).
pub(super) fn inline_comment<'src, I>()
-> impl Parser<'src, I, ast::Comment, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Newline)
        .not()
        .ignore_then(comment())
        .then_ignore(just(Token::Newline))
        .labelled("inline comment")
}

/* -------------------------------------------------------------------------- */
/*                                 Fn: comment                                */
/* -------------------------------------------------------------------------- */

/// `comment` creates a new comment [`Parser`].
fn comment<'src, I>()
-> impl Parser<'src, I, ast::Comment, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let comment = select! { Token::Comment(c) => c };
    comment.map_with(|c, e| ast::Comment {
        content: c.to_owned(),
        span: e.span(),
    })
}
