use chumsky::error::Rich;
use chumsky::extra::Full;
use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;

use crate::core::SchemaImport;

/* ------------------------------- Mod: Input ------------------------------- */

mod input;
pub(self) use input::*;

/* ------------------------------- Mod: Syntax ------------------------------ */

mod syntax;
pub(self) use syntax::*;

/* ------------------------------- Mod: Token ------------------------------- */

mod token;
pub use token::Keyword;
pub use token::Token;

/* ------------------------------- Mod: Types ------------------------------- */

mod types;
pub(self) use types::*;

/* -------------------------------------------------------------------------- */
/*                               Type: LexError                               */
/* -------------------------------------------------------------------------- */

/// LexError is a type alias for errors emitted during lexing.
pub type LexError<'src> = Full<Rich<'src, char, Span>, (), SchemaImport>;

/* -------------------------------------------------------------------------- */
/*                                 Type: Span                                 */
/* -------------------------------------------------------------------------- */

pub use chumsky::span::Spanned;
pub type Span = chumsky::span::SimpleSpan<usize, SchemaImport>;

/* ------------------------------ Fn: spanned ------------------------------- */

/// Helper function to wrap any value with a span from the parser context.
/// Extracts the [`SchemaImport`] from parser state and creates a contextual
/// span.
pub fn spanned<'src, T, I, E>(value: T, info: &mut MapExtra<'src, '_, I, E>) -> Spanned<T, Span>
where
    I: chumsky::input::Input<'src, Span = Span>,
    E: ParserExtra<'src, I, Context = SchemaImport>,
{
    Spanned {
        inner: value,
        span: info.span(),
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Fn: Lex                                  */
/* -------------------------------------------------------------------------- */

use chumsky::input::WithContext;
use chumsky::prelude::*;
use chumsky::text::inline_whitespace;

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
    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        vec![]
    });

    let tokens = inline_whitespace()
        .or_not()
        .ignore_then(choice((
            // NOTE: `string` and `comment` must be checked before `line_break` so
            // that they may validate the full span of their potential input.
            string(),
            comment(),
            line_break(),
            operator(),
            punctuation(),
            keyword(),
            uint(),
            identifier(),
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
pub(self) mod tests {
    use super::*;

    /* -------------------------- Fn: parse_single -------------------------- */

    /// `parse_single` parses a single token without requiring all input to be
    /// consumed. Returns the parsed token even if there's unconsumed input.
    pub(super) fn parse_single<'src, P>(
        parser: P,
        input: &'src str,
    ) -> Result<Spanned<Token<'src>, Span>, Vec<Rich<'src, char, Span>>>
    where
        P: Parser<'src, WithContext<Span, &'src str>, Spanned<Token<'src>, Span>, LexError<'src>>,
    {
        let file = SchemaImport::anonymous();
        let (output, errors) = parser
            .then_ignore(any().repeated())
            .parse(input.with_context(file))
            .into_output_errors();

        match output {
            Some(output) => Ok(output),
            None => Err(errors),
        }
    }

    /* ------------------------ Fn: assert_parses_to ------------------------ */

    /// Assert that parsing succeeds and returns the expected token.
    pub(super) fn assert_parses_to<'src>(
        result: Result<Spanned<Token<'src>, Span>, Vec<Rich<'src, char, Span>>>,
        expected: Token<'src>,
    ) {
        match result {
            Ok(spanned) => assert_eq!(spanned.inner, expected),
            Err(errors) => panic!("expected success but got errors: {errors:?}"),
        }
    }

    /* -------------------- Fn: assert_parses_to_invalid -------------------- */

    /// Assert that parsing produces an `Token::Invalid` token.
    pub(super) fn assert_parses_to_invalid(
        result: Result<Spanned<Token<'_>, Span>, Vec<Rich<'_, char, Span>>>,
    ) {
        match result {
            Ok(spanned) => assert!(
                matches!(spanned.inner, Token::Invalid(_)),
                "expected Invalid token but got: {:?}",
                spanned.inner
            ),
            Err(errors) => panic!("expected Invalid token but got parse errors: {errors:?}"),
        }
    }

    /* -------------------------- Fn: assert_fails -------------------------- */

    /// Assert that parsing fails.
    pub(super) fn assert_fails(
        result: Result<Spanned<Token<'_>, Span>, Vec<Rich<'_, char, Span>>>,
    ) {
        assert!(result.is_err(), "expected parsing to fail but it succeeded");
    }

    /* ---------------------------- Tests: lexer ---------------------------- */

    #[test]
    fn test_lexer_handles_leading_whitespace() {
        // Given: Input with leading whitespace.
        let input = "   package";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The whitespace is ignored and the keyword is parsed.
        assert!(!output.has_errors());
        let tokens = vec![Spanned {
            inner: Token::Keyword(Keyword::Package),
            span: Span::new(file, 3..10),
        }];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_handles_trailing_whitespace() {
        // Given: Input with trailing whitespace.
        let input = "package   ";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The whitespace is ignored and the keyword is parsed.
        assert!(!output.has_errors());
        let tokens = vec![Spanned {
            inner: Token::Keyword(Keyword::Package),
            span: Span::new(file, 0..7),
        }];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_handles_whitespace_between_tokens() {
        // Given: Input with multiple spaces between tokens.
        let input = "package    abc";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: Tokens are correctly identified with whitespace removed.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Package),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::Ident("abc"),
                span: Span::new(file, 11..14),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_handles_tabs_as_whitespace() {
        // Given: Input with tabs as whitespace.
        let input = "\tpackage\tabc\t";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: Tabs are treated as whitespace.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Package),
                span: Span::new(file.clone(), 1..8),
            },
            Spanned {
                inner: Token::Ident("abc"),
                span: Span::new(file, 9..12),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_preserves_newlines() {
        // Given: Input with newlines.
        let input = "package\nabc";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: Newlines are included as tokens.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Package),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::Newline,
                span: Span::new(file.clone(), 7..8),
            },
            Spanned {
                inner: Token::Ident("abc"),
                span: Span::new(file, 8..11),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_handles_inline_comment() {
        // Given: Input with an inline comment after code.
        let input = "package abc // comment";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: Both code and comment are parsed.
        assert!(!output.has_errors());
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
                inner: Token::Comment("comment"),
                span: Span::new(file, 15..22),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_handles_comment_only_line() {
        // Given: Input with a full-line comment.
        let input = "// this is a comment";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: Only the comment token is created.
        assert!(!output.has_errors());
        let tokens = vec![Spanned {
            inner: Token::Comment("this is a comment"),
            span: Span::new(file, 3..20),
        }];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_respects_keyword_word_boundaries() {
        // Given: An identifier that starts with a keyword substring.
        let input = "enumerate";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The identifier is parsed as a single token, not split.
        assert!(!output.has_errors());
        let tokens = vec![Spanned {
            inner: Token::Ident("enumerate"),
            span: Span::new(file, 0..9),
        }];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_collapses_multiple_newlines() {
        // Given: Input with multiple consecutive newlines.
        let input = "package\n\n\nabc";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: Multiple newlines are collapsed into a single newline token.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Package),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::Newline,
                span: Span::new(file.clone(), 7..10),
            },
            Spanned {
                inner: Token::Ident("abc"),
                span: Span::new(file, 10..13),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_all_punctuation_together() {
        // Given: Input with all punctuation tokens.
        let input = ",.;:()[]{}";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: All punctuation tokens are correctly identified.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Comma,
                span: Span::new(file.clone(), 0..1),
            },
            Spanned {
                inner: Token::Dot,
                span: Span::new(file.clone(), 1..2),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file.clone(), 2..3),
            },
            Spanned {
                inner: Token::Colon,
                span: Span::new(file.clone(), 3..4),
            },
            Spanned {
                inner: Token::FnOpen,
                span: Span::new(file.clone(), 4..5),
            },
            Spanned {
                inner: Token::FnClose,
                span: Span::new(file.clone(), 5..6),
            },
            Spanned {
                inner: Token::ListOpen,
                span: Span::new(file.clone(), 6..7),
            },
            Spanned {
                inner: Token::ListClose,
                span: Span::new(file.clone(), 7..8),
            },
            Spanned {
                inner: Token::BlockOpen,
                span: Span::new(file.clone(), 8..9),
            },
            Spanned {
                inner: Token::BlockClose,
                span: Span::new(file, 9..10),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_empty_input() {
        // Given: Empty input.
        let input = "";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: An error is reported for missing input.
        assert!(output.has_errors());
        assert_eq!(
            output.errors().collect::<Vec<_>>(),
            vec![&Rich::custom(Span::new(file, 0..0), "missing input")]
        );

        // Then: An empty token list is still returned.
        assert_eq!(output.output(), Some(&vec![]));
    }

    #[test]
    fn test_lexer_parses_include_with_string() {
        // Given: An include statement with a string path.
        let input = "include \"foo/bar/baz.baproto\"";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The include keyword and string are parsed correctly.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Keyword(Keyword::Include),
                span: Span::new(file.clone(), 0..7),
            },
            Spanned {
                inner: Token::String("foo/bar/baz.baproto"),
                span: Span::new(file, 9..28),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_enum_definition() {
        // Given: An enum definition with a block.
        let input = "enum SomeEnum { \n }";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The enum structure is parsed correctly.
        assert!(!output.has_errors());
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
                span: Span::new(file, 18..19),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_enum_variant() {
        // Given: A simple enum variant definition.
        let input = "VARIANT_1;";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The variant and semicolon are parsed correctly.
        assert!(!output.has_errors());
        let tokens = vec![
            Spanned {
                inner: Token::Ident("VARIANT_1"),
                span: Span::new(file.clone(), 0..9),
            },
            Spanned {
                inner: Token::Semicolon,
                span: Span::new(file, 9..10),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_indexed_enum_variant() {
        // Given: An indexed enum variant definition.
        let input = "0: VARIANT_1;";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The index, colon, variant, and semicolon are parsed correctly.
        assert!(!output.has_errors());
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
                span: Span::new(file, 12..13),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_field_definition() {
        // Given: A field definition with brackets.
        let input = "[key]value array_field;";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The brackets and identifiers are parsed correctly.
        assert!(!output.has_errors());
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
                span: Span::new(file, 22..23),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_indexed_field_definition() {
        // Given: An indexed field definition with brackets.
        let input = "1: [key]value array_field;";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The index, colon, brackets, and identifiers are parsed correctly.
        assert!(!output.has_errors());
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
                span: Span::new(file, 25..26),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_lexer_parses_field_with_encoding() {
        // Given: A field definition with complex nested encoding.
        let input = "u8 array_field = [delta, bits(var(8))];";
        let file = SchemaImport::anonymous();

        // When: Lexing the input.
        let output = lexer().parse(input.with_context(file.clone()));

        // Then: The complex nested structure is parsed correctly.
        assert!(!output.has_errors());
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
                span: Span::new(file, 38..39),
            },
        ];
        assert_eq!(output.output(), Some(&tokens));
    }
}
