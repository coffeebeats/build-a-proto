use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;
use chumsky::prelude::*;
use chumsky::text::Char;
use chumsky::text::ascii::ident;
use chumsky::text::inline_whitespace;
use chumsky::text::newline;
use derive_more::Display;
use std::num::ParseIntError;

/* -------------------------------------------------------------------------- */
/*                                Type: Spanned                               */
/* -------------------------------------------------------------------------- */

/// `Span` is a type alias for [`chumsky::SimpleSpan`].
pub type Span = SimpleSpan;

/// `Spanned` is a type alias for a tuple containing a type `T` and a
/// [`chumsky::SimpleSpan`].
pub type Spanned<T> = (T, Span);

/* -------------------------------------------------------------------------- */
/*                                 Enum: Token                                */
/* -------------------------------------------------------------------------- */

/// `Token` enumerates the set of potential tokens recognized by the parser.
#[allow(dead_code)]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Token<'src> {
    Invalid(&'src str),

    // Syntax
    BlockClose,
    BlockOpen,
    Colon,
    Comma,
    Dot,
    Equal,
    FnClose,
    FnOpen,
    Keyword(Keyword),
    ListClose,
    ListOpen,
    Semicolon,

    // Whitespace
    Newline,

    // Input
    Comment(&'src str),
    Ident(&'src str),
    String(&'src str),
    Uint(usize),
}

/* ----------------------------- Impl: with_span ---------------------------- */

impl<'src> Token<'src> {
    /// `with_span`` is a convenience method for creating a [`Spanned`] item
    /// from the provided [`chumsky::MapExtra`] details.
    fn with_span<E>(self, info: &mut MapExtra<'src, '_, &'src str, E>) -> Spanned<Token<'src>>
    where
        E: ParserExtra<'src, &'src str>,
    {
        (self, info.span())
    }
}

/* -------------------------------------------------------------------------- */
/*                                Enum: Keyword                               */
/* -------------------------------------------------------------------------- */

/// Keyword enumerates the language's reserved keywords.
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Keyword {
    Encoding,
    Enum,
    Include,
    Message,
    Package,
}

/* -------------------------------------------------------------------------- */
/*                                  Fn: Lexer                                 */
/* -------------------------------------------------------------------------- */

/// [lexer] creates a lexer which lexes an input string slice into a sequence
/// of [`Token`]s.
#[allow(dead_code)]
pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>> {
    // Syntax

    let control = choice((
        just(',').map(|_| Token::Comma),
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
    .map_with(Token::with_span);

    let keyword = choice((
        just("encoding").map(|_| Token::Keyword(Keyword::Encoding)),
        just("enum").map(|_| Token::Keyword(Keyword::Enum)),
        just("include").map(|_| Token::Keyword(Keyword::Include)),
        just("message").map(|_| Token::Keyword(Keyword::Message)),
        just("package").map(|_| Token::Keyword(Keyword::Package)),
    ))
    .map_with(Token::with_span)
    .labelled("keyword");

    // Whitespace

    let line_break = newline()
        .map(|_| Token::Newline)
        .map_with(Token::with_span)
        .labelled("line break");

    // Input

    let uint = chumsky::text::int(10).from_str().validate(
        |result: Result<usize, ParseIntError>, info, emitter| {
            if let Ok(value) = result {
                return Token::Uint(value).with_span(info);
            }

            let msg = match result.unwrap_err().kind() {
                std::num::IntErrorKind::PosOverflow => {
                    format!("invalid input: value too large: {}", info.slice())
                }
                _ => format!("invalid input: unrecognized value: {}", info.slice()),
            };

            emitter.emit(Rich::custom(info.span(), msg));

            Token::Invalid(info.slice()).with_span(info)
        },
    );

    let quote = just('"');
    let string = quote
        .ignore_then(any().and_is(quote.not()).repeated().to_slice().validate(
            |input: &'src str, info, emitter| {
                if input.chars().any(|c| c.is_newline()) {
                    let msg = "invalid input: strings cannot contain line breaks";
                    emitter.emit(Rich::custom(info.span(), msg));

                    return Token::Invalid(info.slice()).with_span(info);
                }

                Token::String(input).with_span(info)
            },
        ))
        .then_ignore(quote);

    let comment = just("//").then(inline_whitespace().at_most(1)).ignore_then(
        any()
            .and_is(newline().not())
            .repeated()
            .to_slice()
            .map(Token::Comment)
            .map_with(Token::with_span),
    );

    let identifier = ident().map(Token::Ident).map_with(Token::with_span);

    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        vec![]
    });

    let idl = choice((
        // NOTE: `string` and `comment` must be checked before `line_break` so
        // that they may validate the full span of their potential input.
        string, comment, line_break, control, keyword, uint, identifier,
    ))
    .padded_by(inline_whitespace())
    .recover_with(skip_then_retry_until(any().ignored(), end()))
    .repeated()
    .collect();

    missing.or(idl)
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input_returns_empty_token_list() {
        // Given: An input string.
        let input = "";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has an error.
        assert_eq!(output.has_errors(), true);
        assert_eq!(
            output.errors().collect::<Vec<_>>(),
            vec![&Rich::custom(Span::from(0..0), "missing input")]
        );

        // Then: The output token list matches expectations.
        let tokens = vec![];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_package_keyword_returns_correct_token_list() {
        // Given: An input string.
        let input = "package \"abc.def\"";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Keyword(Keyword::Package), Span::from(0..7)),
            (Token::String("abc.def"), Span::from(9..16)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_include_keyword_returns_correct_token_list() {
        // Given: An input string.
        let input = "include \"../a//b/'path with spaces'/file.ext\"";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Keyword(Keyword::Include), Span::from(0..7)),
            (
                Token::String("../a//b/'path with spaces'/file.ext"),
                Span::from(9..44),
            ),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_comment_returns_correct_token_list() {
        // Given: An input string.
        let input = "// comment // that includes a comment";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![(
            Token::Comment("comment // that includes a comment"),
            Span::from(3..37),
        )];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_enum_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "enum SomeEnum { \n }";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Keyword(Keyword::Enum), Span::from(0..4)),
            (Token::Ident("SomeEnum"), Span::from(5..13)),
            (Token::BlockOpen, Span::from(14..15)),
            (Token::Newline, Span::from(16..17)),
            (Token::BlockClose, Span::from(18..19)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_message_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "message SomeMessage { \n }";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Keyword(Keyword::Message), Span::from(0..7)),
            (Token::Ident("SomeMessage"), Span::from(8..19)),
            (Token::BlockOpen, Span::from(20..21)),
            (Token::Newline, Span::from(22..23)),
            (Token::BlockClose, Span::from(24..25)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_non_indexed_enum_variant_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "VARIANT_1;";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Ident("VARIANT_1"), Span::from(0..9)),
            (Token::Semicolon, Span::from(9..10)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_indexed_enum_variant_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "0: VARIANT_1;";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Uint(0), Span::from(0..1)),
            (Token::Colon, Span::from(1..2)),
            (Token::Ident("VARIANT_1"), Span::from(3..12)),
            (Token::Semicolon, Span::from(12..13)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_non_indexed_field_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "[key]value array_field;";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::ListOpen, Span::from(0..1)),
            (Token::Ident("key"), Span::from(1..4)),
            (Token::ListClose, Span::from(4..5)),
            (Token::Ident("value"), Span::from(5..10)),
            (Token::Ident("array_field"), Span::from(11..22)),
            (Token::Semicolon, Span::from(22..23)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_indexed_field_definition_returns_correct_token_list() {
        // Given: An input string.
        let input = "1: [key]value array_field;";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Uint(1), Span::from(0..1)),
            (Token::Colon, Span::from(1..2)),
            (Token::ListOpen, Span::from(3..4)),
            (Token::Ident("key"), Span::from(4..7)),
            (Token::ListClose, Span::from(7..8)),
            (Token::Ident("value"), Span::from(8..13)),
            (Token::Ident("array_field"), Span::from(14..25)),
            (Token::Semicolon, Span::from(25..26)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }

    #[test]
    fn test_input_with_field_definition_with_encoding_returns_correct_token_list() {
        // Given: An input string.
        let input = "u8 array_field = [delta, bits(var(8))];";

        // When: The input is lexed.
        let output = lexer().parse(input);

        // Then: The input has no errors.
        assert_eq!(output.has_errors(), false);

        // Then: The output token list matches expectations.
        let tokens = vec![
            (Token::Ident("u8"), Span::from(0..2)),
            (Token::Ident("array_field"), Span::from(3..14)),
            (Token::Equal, Span::from(15..16)),
            (Token::ListOpen, Span::from(17..18)),
            (Token::Ident("delta"), Span::from(18..23)),
            (Token::Comma, Span::from(23..24)),
            (Token::Ident("bits"), Span::from(25..29)),
            (Token::FnOpen, Span::from(29..30)),
            (Token::Ident("var"), Span::from(30..33)),
            (Token::FnOpen, Span::from(33..34)),
            (Token::Uint(8), Span::from(34..35)),
            (Token::FnClose, Span::from(35..36)),
            (Token::FnClose, Span::from(36..37)),
            (Token::ListClose, Span::from(37..38)),
            (Token::Semicolon, Span::from(38..39)),
        ];
        assert_eq!(output.output(), Some(&tokens));
    }
}
