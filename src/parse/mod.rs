/* ------------------------------ Mod: Comment ------------------------------ */

mod comment;
use comment::*;

/* ------------------------------ Mod: Encoding ----------------------------- */

mod encoding;
use encoding::*;

/* ---------------------------- Mod: Enumeration ---------------------------- */

mod enumeration;
use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

mod message;
use message::*;

/* ------------------------------ Mod: Package ------------------------------ */

mod package;
use package::*;

/* ------------------------------- Mod: Schema ------------------------------ */

mod schema;
use schema::*;

/* ------------------------------- Mod: Types ------------------------------- */

mod types;
use types::*;

/* -------------------------------------------------------------------------- */
/*                                  Fn: Parse                                 */
/* -------------------------------------------------------------------------- */

use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::core::SchemaImport;
use crate::lex::Span;
use crate::lex::Token;

pub const MAX_RECURSION_DEPTH: usize = 10;

/// `parse` parses an input [`Token`] sequence into an [`ast::Schema`].
pub fn parse<'src>(
    input: &'src Vec<Spanned<Token<'src>, Span>>,
    file: SchemaImport,
) -> ParseResult<'src> {
    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        None
    });

    let eoi = input.last().map_or(0, |s| s.span.end());
    let (ast, errors) = missing
        .or(schema(MAX_RECURSION_DEPTH).map(Some))
        .parse(input.as_slice().map(Span::new(file, 0..eoi), |spanned| {
            (&spanned.inner, &spanned.span)
        }))
        .into_output_errors();

    ParseResult {
        ast: ast.flatten(),
        errors,
    }
}

/* --------------------------- Struct: ParseResult -------------------------- */

/// `ParseResult` contains the result of parsing a `.baproto` file.
pub struct ParseResult<'src> {
    /// The parsed AST, if parsing succeeded (possibly with recovered errors).
    pub ast: Option<ast::Schema>,
    /// Errors encountered during parsing.
    pub errors: Vec<ParseError<'src>>,
}

/* ---------------------------- Type: ParseError ---------------------------- */

/// ParseError is a type alias for errors emitted during parsing.
pub type ParseError<'src> = Rich<'src, Token<'src>, Span>;

/* -------------------------------- Fn: text -------------------------------- */

/// `text` creates a new string literal [`Parser`].
fn text<'src, I>() -> impl Parser<'src, I, ast::Text, chumsky::extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let text = select! { Token::String(s) => s };
    text.map_with(|content, e| ast::Text {
        content: content.to_owned(),
        span: e.span(),
    })
}

/* -------------------------------- Fn: ident ------------------------------- */

/// `ident` creates a new identifier [`Parser`].
fn ident<'src, I>() -> impl Parser<'src, I, ast::Ident, chumsky::extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let id = select! { Token::Ident(id) => id };
    id.map_with(|id, e| ast::Ident {
        name: id.to_owned(),
        span: e.span(),
    })
}

/* -------------------------------- Fn: uint -------------------------------- */

/// `uint` creates a new unsigned integer [`Parser`].
fn uint<'src, I>() -> impl Parser<'src, I, ast::Uint, chumsky::extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let value = select! { Token::Uint(n) => n };
    value.map_with(|value, e| ast::Uint {
        value,
        span: e.span(),
    })
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use chumsky::input::MappedInput;

    use crate::lex::LexResult;
    use crate::lex::Spanned;

    use super::*;

    /* ------------------------ Type: TestParserInput ----------------------- */

    /// Type alias for the parser test input.
    ///
    /// This uses chumsky's `MappedInput` to split `Spanned<Token, Span>`
    /// elements into separate token and span components.
    pub(super) type TestParserInput<'src> =
        MappedInput<'src, Token<'src>, Span, &'src [Spanned<Token<'src>, Span>]>;

    /* -------------------------- Fn: parse_single -------------------------- */

    /// Parse test input with the given parser.
    pub(super) fn parse_single<T, P>(
        input: &'static str,
        parser: P,
    ) -> (Option<T>, Vec<ParseError<'static>>)
    where
        P: Parser<'static, TestParserInput<'static>, T, chumsky::extra::Err<ParseError<'static>>>,
    {
        let file = SchemaImport::anonymous();
        let LexResult { tokens, errors } = crate::lex::lex(input, file.clone());

        assert!(
            errors.is_empty(),
            "Lexing failed with {} error(s) for input:\n{}\n\nErrors:\n{:?}",
            errors.len(),
            input,
            errors
        );

        let tokens = tokens.expect("tokens should exist after successful lexing");

        // Leak the tokens to give them 'static lifetime. This is acceptable
        // for tests since test processes are short-lived.
        let tokens: &'static _ = Box::leak(Box::new(tokens));
        let eoi = tokens.last().map_or(0, |s| s.span.end());

        parser
            .parse(tokens.as_slice().split_spanned(Span::new(file, 0..eoi)))
            .into_output_errors()
    }

    /* ---------------------- Fn: assert_parse_succeeds --------------------- */

    /// Assert that parsing succeeds with no errors and return the parsed value.
    pub(in crate::parse) fn assert_parse_succeeds<T>(
        result: (Option<T>, Vec<ParseError<'_>>),
    ) -> T {
        let (output, errors) = result;
        assert!(errors.is_empty(), "expected no errors but got: {errors:?}");
        output.expect("expected output but got none")
    }

    /* ----------------------- Fn: assert_parse_fails ----------------------- */

    /// Assert that parsing fails with at least one error.
    pub(in crate::parse) fn assert_parse_fails<T>(result: (Option<T>, Vec<ParseError<'_>>)) {
        let (_, errors) = result;
        assert!(
            !errors.is_empty(),
            "expected parsing to fail but it succeeded"
        );
    }

    /* ---------------------------- Tests: ident ---------------------------- */

    #[test]
    fn test_ident_simple_succeeds() {
        // Given: A simple identifier.
        let input = "foo";

        // When: The input is parsed.
        let ident = assert_parse_succeeds(parse_single(input, ident()));

        // Then: The identifier has the correct name.
        assert_eq!(ident.name, "foo");
    }

    #[test]
    fn test_ident_with_underscores_succeeds() {
        // Given: An identifier with underscores.
        let input = "foo_bar_baz";

        // When: The input is parsed.
        let ident = assert_parse_succeeds(parse_single(input, ident()));

        // Then: The identifier has the correct name.
        assert_eq!(ident.name, "foo_bar_baz");
    }

    #[test]
    fn test_ident_with_numbers_succeeds() {
        // Given: An identifier with numbers.
        let input = "foo123";

        // When: The input is parsed.
        let ident = assert_parse_succeeds(parse_single(input, ident()));

        // Then: The identifier has the correct name.
        assert_eq!(ident.name, "foo123");
    }

    /* ----------------------------- Tests: uint ---------------------------- */

    #[test]
    fn test_uint_small_value_succeeds() {
        // Given: A small unsigned integer.
        let input = "42";

        // When: The input is parsed.
        let uint = assert_parse_succeeds(parse_single(input, uint()));

        // Then: The value is correct.
        assert_eq!(uint.value, 42);
    }

    #[test]
    fn test_uint_zero_succeeds() {
        // Given: Zero as an unsigned integer.
        let input = "0";

        // When: The input is parsed.
        let uint = assert_parse_succeeds(parse_single(input, uint()));

        // Then: The value is zero.
        assert_eq!(uint.value, 0);
    }

    #[test]
    fn test_uint_large_value_succeeds() {
        // Given: A large unsigned integer.
        let input = "18446744073709551615"; // u64::MAX

        // When: The input is parsed.
        let uint = assert_parse_succeeds(parse_single(input, uint()));

        // Then: The value is correct.
        assert_eq!(uint.value, u64::MAX);
    }

    /* ----------------------------- Tests: text ---------------------------- */

    #[test]
    fn test_text_simple_succeeds() {
        // Given: A simple string literal.
        let input = "\"hello\"";

        // When: The input is parsed.
        let text = assert_parse_succeeds(parse_single(input, text()));

        // Then: The content is correct.
        assert_eq!(text.content, "hello");
    }

    #[test]
    fn test_text_with_spaces_succeeds() {
        // Given: A string literal with spaces.
        let input = "\"hello world\"";

        // When: The input is parsed.
        let text = assert_parse_succeeds(parse_single(input, text()));

        // Then: The content includes spaces.
        assert_eq!(text.content, "hello world");
    }

    #[test]
    fn test_text_empty_succeeds() {
        // Given: An empty string literal.
        let input = "\"\"";

        // When: The input is parsed.
        let text = assert_parse_succeeds(parse_single(input, text()));

        // Then: The content is empty.
        assert_eq!(text.content, "");
    }

    #[test]
    fn test_text_with_path_characters_succeeds() {
        // Given: A string literal with path-like characters.
        let input = "\"foo/bar/baz.baproto\"";

        // When: The input is parsed.
        let text = assert_parse_succeeds(parse_single(input, text()));

        // Then: The content preserves path characters.
        assert_eq!(text.content, "foo/bar/baz.baproto");
    }
}
