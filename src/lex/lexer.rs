use chumsky::input::WithContext;
use chumsky::prelude::*;
use chumsky::text::Char;
use chumsky::text::ascii::ident;
use chumsky::text::inline_whitespace;
use chumsky::text::newline;
use std::num::ParseIntError;

use crate::core::SchemaImport;

use super::Keyword;
use super::Span;
use super::Spanned;
use super::Token;
use super::spanned;

/* -------------------------------------------------------------------------- */
/*                                   Fn: Lex                                  */
/* -------------------------------------------------------------------------- */

/// LexError is a type alias for errors emitted during lexing.
pub type LexError<'src> = extra::Full<Rich<'src, char, Span>, (), SchemaImport>;

/// `lex` lexes an input string into [`Token`]s recognized by the parser.
pub fn lex<'src>(
    input: &'src str,
    file: SchemaImport,
) -> (
    Option<Vec<Spanned<Token<'src>, Span>>>,
    Vec<Rich<'src, char, Span>>,
) {
    lexer().parse(input.with_context(file)).into_output_errors()
}

/* -------------------------------- Fn: lexer ------------------------------- */

/// [lexer] creates a lexer which lexes an input string slice into a sequence
/// of [`Token`]s.
fn lexer<'src>()
-> impl Parser<'src, WithContext<Span, &'src str>, Vec<Spanned<Token<'src>, Span>>, LexError<'src>>
{
    // Syntax

    let control = choice((
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
        just('=').map(|_| Token::Equal),
    ))
    .map_with(spanned);

    let keyword = choice((
        just("encoding").map(|_| Token::Keyword(Keyword::Encoding)),
        just("enum").map(|_| Token::Keyword(Keyword::Enum)),
        just("include").map(|_| Token::Keyword(Keyword::Include)),
        just("message").map(|_| Token::Keyword(Keyword::Message)),
        just("package").map(|_| Token::Keyword(Keyword::Package)),
    ))
    .map_with(spanned)
    .labelled("keyword");

    // Whitespace

    let line_break = newline()
        .map(|_| Token::Newline)
        .map_with(spanned)
        .labelled("line break");

    // Input

    let uint = chumsky::text::int(10).from_str().validate(
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
    );

    let quote = just('"');
    let string = any()
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
        .delimited_by(quote, quote);

    let comment = just("//").then(inline_whitespace().at_most(1)).ignore_then(
        any()
            .and_is(newline().not())
            .repeated()
            .to_slice()
            .map(Token::Comment)
            .map_with(spanned),
    );

    let identifier = ident().to_slice().map(Token::Ident).map_with(spanned);

    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        vec![]
    });

    let tokens = inline_whitespace()
        .or_not()
        .ignore_then(choice((
            // NOTE: `string` and `comment` must be checked before `line_break` so
            // that they may validate the full span of their potential input.
            string, comment, line_break, control, keyword, uint, identifier,
        )))
        .then_ignore(inline_whitespace())
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect();

    missing.or(tokens)
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    // FIXME: Add unit test to validate `enums` isn't lexed into a keyword.

    #[test]
    fn test_empty_input_returns_empty_token_list() {
        // Given: An input string.
        let input = "";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has an error.
        assert!(output.has_errors());
        assert_eq!(
            output.errors().collect::<Vec<_>>(),
            vec![&Rich::custom(
                Span::new(file.clone(), 0..0),
                "missing input"
            )]
        );

        // Then: The output token list matches expectations.
        let tokens = vec![];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_package_keyword_returns_correct_token_list() {
        // Given: An input string with package using dot-separated identifiers.
        let input = "package abc.def";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Package),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::Ident("abc"),
                span: Span::new(file.clone(), 8..11),
            },
            Spanned {
                inner: Token::Dot,
                span: Span::new(file.clone(), 11..12),
            },
            Spanned {
                inner: Token::Ident("def"),
                span: Span::new(file.clone(), 12..15),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_include_keyword_returns_correct_token_list() {
        // Given: An input string.
        let input = "include \"foo/bar/baz.baproto\"";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Include),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::String("foo/bar/baz.baproto"),
                span: Span::new(file.clone(), 9..28),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_comment_returns_correct_token_list() {
        // Given: An input string.
        let input = "// comment // that includes a comment";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![Spanned {
            inner: Token::Comment("comment // that includes a comment"),
            span: Span::new(file.clone(), 3..37),
        }];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_enum_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "enum SomeEnum { \n }";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Enum),
                span: Span::new(file.clone(), 0..4),
            },
            Spanned {
                inner: Token::Ident("SomeEnum"),
                span: Span::new(file.clone(), 5..13),
            },
            Spanned {
                inner: Token::BlockOpen,
                span: Span::new(file.clone(), 14..15),
            },
            Spanned {
                inner: Token::Newline,
                span: Span::new(file.clone(), 16..17),
            },
            Spanned {
                inner: Token::BlockClose,
                span: Span::new(file.clone(), 18..19),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_message_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "message SomeMessage { \n }";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Message),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::Ident("SomeMessage"),
                span: Span::new(file.clone(), 8..19),
            },
            Spanned {
                inner: Token::BlockOpen,
                span: Span::new(file.clone(), 20..21),
            },
            Spanned {
                inner: Token::Newline,
                span: Span::new(file.clone(), 22..23),
            },
            Spanned {
                inner: Token::BlockClose,
                span: Span::new(file.clone(), 24..25),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_non_indexed_enum_variant_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "VARIANT_1;";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Ident("VARIANT_1"),
                span: Span::new(file.clone(), 0..9),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file.clone(), 9..10),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_indexed_enum_variant_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "0: VARIANT_1;";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Uint(0),
                span: Span::new(file.clone(), 0..1),
            },
            Spanned {
                inner: Token::Colon,
                span: Span::new(file.clone(), 1..2),
            },
            Spanned {
                inner: Token::Ident("VARIANT_1"),
                span: Span::new(file.clone(), 3..12),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file.clone(), 12..13),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_non_indexed_field_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "[key]value array_field;";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::ListOpen,
                span: Span::new(file.clone(), 0..1),
            },
            Spanned {
                inner: Token::Ident("key"),
                span: Span::new(file.clone(), 1..4),
            },
            Spanned {
                inner: Token::ListClose,
                span: Span::new(file.clone(), 4..5),
            },
            Spanned {
                inner: Token::Ident("value"),
                span: Span::new(file.clone(), 5..10),
            },
            Spanned {
                inner: Token::Ident("array_field"),
                span: Span::new(file.clone(), 11..22),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file.clone(), 22..23),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_indexed_field_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "1: [key]value array_field;";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Uint(1),
                span: Span::new(file.clone(), 0..1),
            },
            Spanned {
                inner: Token::Colon,
                span: Span::new(file.clone(), 1..2),
            },
            Spanned {
                inner: Token::ListOpen,
                span: Span::new(file.clone(), 3..4),
            },
            Spanned {
                inner: Token::Ident("key"),
                span: Span::new(file.clone(), 4..7),
            },
            Spanned {
                inner: Token::ListClose,
                span: Span::new(file.clone(), 7..8),
            },
            Spanned {
                inner: Token::Ident("value"),
                span: Span::new(file.clone(), 8..13),
            },
            Spanned {
                inner: Token::Ident("array_field"),
                span: Span::new(file.clone(), 14..25),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file.clone(), 25..26),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_field_definition_with_encoding_returns_correct_token_list() {
        // Given: An input string.
        let input = "u8 array_field = [delta, bits(var(8))];";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list matches expectations.
        let tokens = vec![
            Spanned {
                inner: Token::Ident("u8"),
                span: Span::new(file.clone(), 0..2),
            },
            Spanned {
                inner: Token::Ident("array_field"),
                span: Span::new(file.clone(), 3..14),
            },
            Spanned {
                inner: Token::Equal,
                span: Span::new(file.clone(), 15..16),
            },
            Spanned {
                inner: Token::ListOpen,
                span: Span::new(file.clone(), 17..18),
            },
            Spanned {
                inner: Token::Ident("delta"),
                span: Span::new(file.clone(), 18..23),
            },
            Spanned {
                inner: Token::Comma,
                span: Span::new(file.clone(), 23..24),
            },
            Spanned {
                inner: Token::Ident("bits"),
                span: Span::new(file.clone(), 25..29),
            },
            Spanned {
                inner: Token::FnOpen,
                span: Span::new(file.clone(), 29..30),
            },
            Spanned {
                inner: Token::Ident("var"),
                span: Span::new(file.clone(), 30..33),
            },
            Spanned {
                inner: Token::FnOpen,
                span: Span::new(file.clone(), 33..34),
            },
            Spanned {
                inner: Token::Uint(8),
                span: Span::new(file.clone(), 34..35),
            },
            Spanned {
                inner: Token::FnClose,
                span: Span::new(file.clone(), 35..36),
            },
            Spanned {
                inner: Token::FnClose,
                span: Span::new(file.clone(), 36..37),
            },
            Spanned {
                inner: Token::ListClose,
                span: Span::new(file.clone(), 37..38),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file.clone(), 38..39),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_u64_max_value_parses_successfully() {
        // Given: An input string with u64::MAX.
        let input = "18446744073709551615";
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output token list contains the max u64 value.
        let tokens = vec![Spanned {
            inner: Token::Uint(u64::MAX),
            span: Span::new(file.clone(), 0..20),
        }];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_value_exceeding_u64_max_produces_error() {
        // Given: An input string with a value that exceeds u64::MAX.
        let input = "18446744073709551616"; // u64::MAX + 1
        let file = SchemaImport::anonymous();

        // When: The input is lexed.
        let output = lexer().parse(input.with_context(file));

        // Then: The input has an error.
        assert!(output.has_errors());
        let errors = output.errors().collect::<Vec<_>>();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .reason()
                .to_string()
                .contains("integer value exceeds maximum")
        );

        // Then: The output contains an Invalid token.
        let tokens = output.output().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].inner, Token::Invalid(_)));
    }
}
