use chumsky::input::WithContext;
use chumsky::prelude::*;
use chumsky::text::ascii::ident;
use chumsky::text::inline_whitespace;
use chumsky::text::newline;

use crate::lex::LexError;
use crate::lex::Span;
use crate::lex::Spanned;
use crate::lex::Token;
use crate::lex::spanned;

/* -------------------------------------------------------------------------- */
/*                               Fn: identifier                               */
/* -------------------------------------------------------------------------- */

/// `identifier` parses a variable/declaration identifier (i.e. a name or
/// reference) into a token.
pub(super) fn identifier<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    ident().to_slice().map(Token::Ident).map_with(spanned)
}

/* -------------------------------------------------------------------------- */
/*                                 Fn: comment                                */
/* -------------------------------------------------------------------------- */

/// `comment` parses a comment into a token. Note that this matches inline and
/// full-line comments.
pub(super) fn comment<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    just("//").then(inline_whitespace().at_most(1)).ignore_then(
        any()
            .and_is(newline().not())
            .repeated()
            .to_slice()
            .map(Token::Comment)
            .map_with(spanned),
    )
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::tests::*;

    /* -------------------------- Tests: identifier ------------------------- */

    #[test]
    fn test_identifier_parses_simple_name() {
        // Given: A simple identifier string.
        let input = "myvar";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The identifier token is created successfully.
        assert_parses_to(result, Token::Ident("myvar"));
    }

    #[test]
    fn test_identifier_parses_with_underscores() {
        // Given: An identifier with underscores.
        let input = "my_var_name";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The identifier token is created successfully.
        assert_parses_to(result, Token::Ident("my_var_name"));
    }

    #[test]
    fn test_identifier_parses_with_numbers() {
        // Given: An identifier with numbers (not starting with a number).
        let input = "var123";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The identifier token is created successfully.
        assert_parses_to(result, Token::Ident("var123"));
    }

    #[test]
    fn test_identifier_parses_uppercase() {
        // Given: An identifier in uppercase.
        let input = "MY_CONSTANT";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The identifier token is created successfully.
        assert_parses_to(result, Token::Ident("MY_CONSTANT"));
    }

    #[test]
    fn test_identifier_parses_camel_case() {
        // Given: An identifier in camelCase.
        let input = "myVariableName";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The identifier token is created successfully.
        assert_parses_to(result, Token::Ident("myVariableName"));
    }

    #[test]
    fn test_identifier_parses_pascal_case() {
        // Given: An identifier in PascalCase.
        let input = "MyTypeName";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The identifier token is created successfully.
        assert_parses_to(result, Token::Ident("MyTypeName"));
    }

    #[test]
    fn test_identifier_rejects_starting_with_number() {
        // Given: An invalid identifier starting with a number.
        let input = "123invalid";

        // When: Attempting to parse the identifier.
        let result = parse_single(identifier(), input);

        // Then: Parsing fails.
        assert_fails(result);
    }

    #[test]
    fn test_identifier_with_trailing_underscore() {
        // Given: An identifier ending with an underscore.
        let input = "foo_";

        // When: Parsing the identifier.
        let result = parse_single(identifier(), input);

        // Then: The entire identifier is consumed.
        assert_parses_to(result, Token::Ident("foo_"));
    }

    /* --------------------------- Tests: comment --------------------------- */

    #[test]
    fn test_comment_parses_simple() {
        // Given: A simple comment.
        let input = "// this is a comment";

        // When: Parsing the comment.
        let result = parse_single(comment(), input);

        // Then: The comment token is created successfully.
        assert_parses_to(result, Token::Comment("this is a comment"));
    }

    #[test]
    fn test_comment_parses_empty() {
        // Given: An empty comment.
        let input = "//";

        // When: Parsing the comment.
        let result = parse_single(comment(), input);

        // Then: The comment token is created with empty content.
        assert_parses_to(result, Token::Comment(""));
    }

    #[test]
    fn test_comment_parses_with_single_space() {
        // Given: A comment with a single space after the slashes.
        let input = "// comment text";

        // When: Parsing the comment.
        let result = parse_single(comment(), input);

        // Then: The comment token is created successfully.
        assert_parses_to(result, Token::Comment("comment text"));
    }

    #[test]
    fn test_comment_preserves_multiple_spaces() {
        // Given: A comment with multiple spaces after the slashes.
        let input = "//  comment with extra spaces";

        // When: Parsing the comment.
        let result = parse_single(comment(), input);

        // Then: The comment includes all spaces after the first.
        assert_parses_to(result, Token::Comment(" comment with extra spaces"));
    }

    #[test]
    fn test_comment_parses_with_special_characters() {
        // Given: A comment containing special characters.
        let input = "// TODO: fix @bug #123 (important!)";

        // When: Parsing the comment.
        let result = parse_single(comment(), input);

        // Then: The comment token preserves all special characters.
        assert_parses_to(result, Token::Comment("TODO: fix @bug #123 (important!)"));
    }

    #[test]
    fn test_comment_parses_with_nested_slashes() {
        // Given: A comment containing additional slashes.
        let input = "// http://example.com // nested comment";

        // When: Parsing the comment.
        let result = parse_single(comment(), input);

        // Then: The comment token includes the nested slashes.
        assert_parses_to(
            result,
            Token::Comment("http://example.com // nested comment"),
        );
    }
}