use chumsky::input::WithContext;
use chumsky::prelude::*;
use chumsky::text::newline;

use crate::lex::Keyword;
use crate::lex::LexError;
use crate::lex::Span;
use crate::lex::Spanned;
use crate::lex::Token;
use crate::lex::spanned;

/* -------------------------------------------------------------------------- */
/*                                 Fn: keyword                                */
/* -------------------------------------------------------------------------- */

/// `keyword` parses a single 'baproto' language keyword token. Keywords are
/// only matched as complete words, not as prefixes of identifiers or strings.
pub(super) fn keyword<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    choice((
        chumsky::text::keyword("encoding").map(|_| Token::Keyword(Keyword::Encoding)),
        chumsky::text::keyword("enum").map(|_| Token::Keyword(Keyword::Enum)),
        chumsky::text::keyword("include").map(|_| Token::Keyword(Keyword::Include)),
        chumsky::text::keyword("message").map(|_| Token::Keyword(Keyword::Message)),
        chumsky::text::keyword("package").map(|_| Token::Keyword(Keyword::Package)),
    ))
    .map_with(spanned)
    .labelled("keyword")
}

/* -------------------------------------------------------------------------- */
/*                               Fn: line_break                               */
/* -------------------------------------------------------------------------- */

/// `line_break` parses repeated line breaks into a single line break token.
pub(super) fn line_break<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    newline()
        .repeated()
        .at_least(1)
        .map(|_| Token::Newline)
        .map_with(spanned)
        .labelled("line break")
}

/* -------------------------------------------------------------------------- */
/*                                Fn: operator                                */
/* -------------------------------------------------------------------------- */

/// `operator` parses a single operator token.
pub(super) fn operator<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    just('=')
        .map(|_| Token::Equal)
        .map_with(spanned)
        .labelled("operator")
}

/* -------------------------------------------------------------------------- */
/*                               Fn: punctuation                              */
/* -------------------------------------------------------------------------- */

/// `punctuation` parses a single punctuation token.
pub(super) fn punctuation<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    choice((
        just(',').map(|_| Token::Comma),
        just('.').map(|_| Token::Dot),
        just(';').map(|_| Token::Semicolon),
        just(':').map(|_| Token::Colon),
        just('(').map(|_| Token::FnOpen),
        just(')').map(|_| Token::FnClose),
        just('[').map(|_| Token::ListOpen),
        just(']').map(|_| Token::ListClose),
        just('{').map(|_| Token::BlockOpen),
        just('}').map(|_| Token::BlockClose),
    ))
    .map_with(spanned)
    .labelled("punctuation")
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::tests::*;

    /* ---------------------------- Tests: keyword -------------------------- */

    #[test]
    fn test_keyword_parses_encoding() {
        // Given: The "encoding" keyword.
        let input = "encoding";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword token is created successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Encoding));
    }

    #[test]
    fn test_keyword_parses_enum() {
        // Given: The "enum" keyword.
        let input = "enum";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword token is created successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Enum));
    }

    #[test]
    fn test_keyword_parses_include() {
        // Given: The "include" keyword.
        let input = "include";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword token is created successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Include));
    }

    #[test]
    fn test_keyword_parses_message() {
        // Given: The "message" keyword.
        let input = "message";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword token is created successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Message));
    }

    #[test]
    fn test_keyword_parses_package() {
        // Given: The "package" keyword.
        let input = "package";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword token is created successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Package));
    }

    #[test]
    fn test_keyword_rejects_partial_match() {
        // Given: A string that starts like a keyword but isn't complete.
        let input = "enumerate";

        // When: Attempting to parse as a keyword.
        let result = parse_single(keyword(), input);

        // Then: Parsing fails.
        assert_fails(result);
    }

    #[test]
    fn test_keyword_rejects_capitalized() {
        // Given: A capitalized keyword.
        let input = "Enum";

        // When: Attempting to parse as a keyword.
        let result = parse_single(keyword(), input);

        // Then: Parsing fails (keywords are case-sensitive).
        assert_fails(result);
    }

    #[test]
    fn test_keyword_followed_by_space() {
        // Given: A keyword followed by a space.
        let input = "enum ";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Enum));
    }

    #[test]
    fn test_keyword_followed_by_tab() {
        // Given: A keyword followed by a tab.
        let input = "message\t";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Message));
    }

    #[test]
    fn test_keyword_followed_by_newline() {
        // Given: A keyword followed by a newline.
        let input = "package\n";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Package));
    }

    #[test]
    fn test_keyword_followed_by_comma() {
        // Given: A keyword followed by a comma.
        let input = "enum,";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Enum));
    }

    #[test]
    fn test_keyword_followed_by_semicolon() {
        // Given: A keyword followed by a semicolon.
        let input = "enum;";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Enum));
    }

    #[test]
    fn test_keyword_followed_by_open_brace() {
        // Given: A keyword followed by an open brace.
        let input = "message{";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Message));
    }

    #[test]
    fn test_keyword_followed_by_close_brace() {
        // Given: A keyword followed by a close brace.
        let input = "enum}";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Enum));
    }

    #[test]
    fn test_keyword_followed_by_dot() {
        // Given: A keyword followed by a dot.
        let input = "package.";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Package));
    }

    #[test]
    fn test_keyword_followed_by_equal() {
        // Given: A keyword followed by an equal sign.
        let input = "encoding=";

        // When: Parsing the keyword.
        let result = parse_single(keyword(), input);

        // Then: The keyword is parsed successfully.
        assert_parses_to(result, Token::Keyword(Keyword::Encoding));
    }

    /* -------------------------- Tests: line_break ------------------------- */

    #[test]
    fn test_line_break_parses_single_newline() {
        // Given: A single newline character.
        let input = "\n";

        // When: Parsing the line break.
        let result = parse_single(line_break(), input);

        // Then: A newline token is created.
        assert_parses_to(result, Token::Newline);
    }

    #[test]
    fn test_line_break_parses_multiple_newlines() {
        // Given: Multiple sequential newline characters.
        let input = "\n\n\n";

        // When: Parsing the line breaks.
        let result = parse_single(line_break(), input);

        // Then: A single newline token is created.
        assert_parses_to(result, Token::Newline);
    }

    /* --------------------------- Tests: operator -------------------------- */

    #[test]
    fn test_operator_parses_equal() {
        // Given: The equal sign operator.
        let input = "=";

        // When: Parsing the operator.
        let result = parse_single(operator(), input);

        // Then: The operator token is created successfully.
        assert_parses_to(result, Token::Equal);
    }

    /* -------------------------- Tests: punctuation ------------------------ */

    #[test]
    fn test_punctuation_parses_comma() {
        // Given: A comma character.
        let input = ",";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::Comma);
    }

    #[test]
    fn test_punctuation_parses_dot() {
        // Given: A dot character.
        let input = ".";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::Dot);
    }

    #[test]
    fn test_punctuation_parses_semicolon() {
        // Given: A semicolon character.
        let input = ";";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::Semicolon);
    }

    #[test]
    fn test_punctuation_parses_colon() {
        // Given: A colon character.
        let input = ":";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::Colon);
    }

    #[test]
    fn test_punctuation_parses_fn_open() {
        // Given: An open parenthesis.
        let input = "(";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::FnOpen);
    }

    #[test]
    fn test_punctuation_parses_fn_close() {
        // Given: A close parenthesis.
        let input = ")";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::FnClose);
    }

    #[test]
    fn test_punctuation_parses_list_open() {
        // Given: An open bracket.
        let input = "[";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::ListOpen);
    }

    #[test]
    fn test_punctuation_parses_list_close() {
        // Given: A close bracket.
        let input = "]";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::ListClose);
    }

    #[test]
    fn test_punctuation_parses_block_open() {
        // Given: An open brace.
        let input = "{";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::BlockOpen);
    }

    #[test]
    fn test_punctuation_parses_block_close() {
        // Given: A close brace.
        let input = "}";

        // When: Parsing the punctuation.
        let result = parse_single(punctuation(), input);

        // Then: The punctuation token is created successfully.
        assert_parses_to(result, Token::BlockClose);
    }
}
