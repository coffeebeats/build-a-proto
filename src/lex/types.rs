use chumsky::input::WithContext;
use chumsky::prelude::*;
use chumsky::text::Char;
use std::num::ParseIntError;

use crate::lex::LexError;
use crate::lex::Span;
use crate::lex::Token;
use crate::lex::spanned;

/* -------------------------------------------------------------------------- */
/*                                 Fn: string                                 */
/* -------------------------------------------------------------------------- */

/// `string` parses a string literal into a token.
pub(super) fn string<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    let quote = just('"');

    any()
        .and_is(quote.not())
        .repeated()
        .to_slice()
        .validate(|input: &'src str, info, emitter| {
            if input.chars().any(|c| c.is_newline()) {
                let msg = "invalid input: strings cannot contain line breaks";
                emitter.emit(Rich::custom(info.span(), msg));

                return spanned(Token::Invalid(info.slice()), info);
            }

            spanned(Token::String(input), info)
        })
        .delimited_by(quote, quote)
}

/* -------------------------------------------------------------------------- */
/*                                  Fn: uint                                  */
/* -------------------------------------------------------------------------- */

/// `uint` parses and validates an unsigned integer into a token.
pub(super) fn uint<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>> {
    chumsky::text::int(10).from_str().validate(
        |result: Result<u64, ParseIntError>, info, emitter| {
            if let Ok(value) = result {
                return spanned(Token::Uint(value), info);
            }

            let msg = match result.unwrap_err().kind() {
                std::num::IntErrorKind::PosOverflow => {
                    format!(
                        "invalid input: integer value exceeds maximum ({}): {}",
                        u64::MAX,
                        info.slice()
                    )
                }
                _ => format!("invalid input: unrecognized value: {}", info.slice()),
            };

            emitter.emit(Rich::custom(info.span(), msg));

            spanned(Token::Invalid(info.slice()), info)
        },
    )
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::tests::*;

    /* ---------------------------- Tests: string --------------------------- */

    #[test]
    fn test_string_parses_empty() {
        // Given: An empty string literal.
        let input = r#""""#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: An empty string token is created successfully.
        assert_parses_to(result, Token::String(""));
    }

    #[test]
    fn test_string_parses_simple_text() {
        // Given: A simple string literal.
        let input = r#""hello world""#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: The string token is created successfully.
        assert_parses_to(result, Token::String("hello world"));
    }

    #[test]
    fn test_string_parses_path() {
        // Given: A string containing a file path.
        let input = r#""foo/bar/baz.baproto""#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: The string token is created successfully.
        assert_parses_to(result, Token::String("foo/bar/baz.baproto"));
    }

    #[test]
    fn test_string_parses_with_spaces() {
        // Given: A string with multiple spaces.
        let input = r#""multiple   spaces   here""#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: The string token preserves all spaces.
        assert_parses_to(result, Token::String("multiple   spaces   here"));
    }

    #[test]
    fn test_string_parses_with_special_characters() {
        // Given: A string with special characters.
        let input = r#""hello@world!#$%^&*()""#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: The string token is created successfully.
        assert_parses_to(result, Token::String("hello@world!#$%^&*()"));
    }

    #[test]
    fn test_string_parses_with_numbers() {
        // Given: A string containing numbers.
        let input = r#""version123""#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: The string token is created successfully.
        assert_parses_to(result, Token::String("version123"));
    }

    #[test]
    fn test_string_rejects_newline() {
        // Given: A string literal containing a newline character.
        let input = "\"hello\nworld\"";

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: Parsing produces an Invalid token with an error.
        assert_parses_to_invalid(result);
    }

    #[test]
    fn test_string_rejects_unclosed() {
        // Given: An unclosed string literal.
        let input = r#""unclosed"#;

        // When: Parsing the string.
        let result = parse_single(string(), input);

        // Then: Parsing fails.
        assert_fails(result);
    }

    /* ----------------------------- Tests: uint ---------------------------- */

    #[test]
    fn test_uint_parses_zero() {
        // Given: The number zero.
        let input = "0";

        // When: Parsing the uint.
        let result = parse_single(uint(), input);

        // Then: The uint token is created successfully.
        assert_parses_to(result, Token::Uint(0));
    }

    #[test]
    fn test_uint_parses_max_u64() {
        // Given: The maximum u64 value.
        let input = "18446744073709551615";

        // When: Parsing the uint.
        let result = parse_single(uint(), input);

        // Then: The uint token is created successfully.
        assert_parses_to(result, Token::Uint(u64::MAX));
    }

    #[test]
    fn test_uint_rejects_overflow() {
        // Given: A number exceeding u64::MAX.
        let input = "18446744073709551616"; // u64::MAX + 1

        // When: Parsing the uint.
        let result = parse_single(uint(), input);

        // Then: Parsing produces an 'Invalid' token with an error.
        assert_parses_to_invalid(result);
    }

    #[test]
    fn test_uint_rejects_negative() {
        // Given: A negative number.
        let input = "-123";

        // When: Parsing the uint.
        let result = parse_single(uint(), input);

        // Then: Parsing fails (negative numbers are not valid uints).
        assert_fails(result);
    }

    #[test]
    fn test_uint_rejects_leading_plus() {
        // Given: A number with a leading plus sign.
        let input = "+123";

        // When: Parsing the uint.
        let result = parse_single(uint(), input);

        // Then: Parsing fails (plus signs are not allowed).
        assert_fails(result);
    }
}
