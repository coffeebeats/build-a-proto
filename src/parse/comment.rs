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

#[allow(unused)] // TODO(#60): Handle inline comments during parsing.
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

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::tests::*;

    /* ------------------------ Tests: comment_block ------------------------ */

    #[test]
    fn test_comment_block_without_trailing_newline_fails() {
        // Given: A comment without a trailing newline.
        let input = "// No trailing newline";

        // When: Parsing the input.
        // Then: Parsing fails (requires trailing newline).
        assert_parse_fails(parse_single(input, comment_block()));
    }

    #[test]
    fn test_comment_block_single_line() {
        // Given: A single comment line.
        let input = "// This is a comment\n";

        // When: Parsing the input.
        let block = assert_parse_succeeds(parse_single(input, comment_block()));

        // Then: The block contains one comment.
        assert_eq!(block.comments.len(), 1);
        assert_eq!(block.comments[0].content, "This is a comment");
    }

    #[test]
    fn test_comment_block_multiple_lines() {
        // Given: Multiple contiguous comment lines.
        let input = "// First line\n// Second line\n// Third line\n";

        // When: Parsing the input.
        let block = assert_parse_succeeds(parse_single(input, comment_block()));

        // Then: All comments are captured.
        assert_eq!(block.comments.len(), 3);
        assert_eq!(block.comments[0].content, "First line");
        assert_eq!(block.comments[1].content, "Second line");
        assert_eq!(block.comments[2].content, "Third line");
    }

    #[test]
    fn test_comment_block_with_empty_content() {
        // Given: A comment with no content after //.
        let input = "//\n";

        // When: Parsing the input.
        let block = assert_parse_succeeds(parse_single(input, comment_block()));

        // Then: The comment is captured with empty content.
        assert_eq!(block.comments.len(), 1);
        assert_eq!(block.comments[0].content, "");
    }

    #[test]
    fn test_comment_block_with_special_characters() {
        // Given: A comment with special characters.
        let input = "// Special: @#$%^&*(){}[]<>?/\n";

        // When: Parsing the input.
        let block = assert_parse_succeeds(parse_single(input, comment_block()));

        // Then: Special characters are preserved.
        assert_eq!(block.comments[0].content, "Special: @#$%^&*(){}[]<>?/");
    }

    #[test]
    fn test_comment_block_with_varying_content() {
        // Given: Comments with varying content styles.
        let input = "// TODO: implement this\n// FIXME: broken\n// Copyright (c) 2024\n";

        // When: Parsing the input.
        let block = assert_parse_succeeds(parse_single(input, comment_block()));

        // Then: All comment styles are preserved.
        assert_eq!(block.comments.len(), 3);
        assert_eq!(block.comments[0].content, "TODO: implement this");
        assert_eq!(block.comments[1].content, "FIXME: broken");
        assert_eq!(block.comments[2].content, "Copyright (c) 2024");
    }

    #[test]
    fn test_comment_block_with_different_spacing() {
        // Given: Two comments with different spacing patterns.
        let input = "//No space\n//  Two spaces\n";

        // When: Parsing the input.
        let block = assert_parse_succeeds(parse_single(input, comment_block()));

        // Then: Spacing is preserved.
        assert_eq!(block.comments.len(), 2);
        assert_eq!(block.comments[0].content, "No space");
        assert_eq!(block.comments[1].content, " Two spaces");
    }
}
