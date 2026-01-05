/* ------------------------------ Mod: Comment ------------------------------ */

pub(self) mod comment;
pub(self) use comment::*;

/* ------------------------------ Mod: Encoding ----------------------------- */

pub(self) mod encoding;
pub(self) use encoding::*;

/* ---------------------------- Mod: Enumeration ---------------------------- */

pub(self) mod enumeration;
pub(self) use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

pub(self) mod message;
pub(self) use message::*;

/* ------------------------------ Mod: Package ------------------------------ */

pub(self) mod package;
pub(self) use package::*;

/* ------------------------------- Mod: Schema ------------------------------ */

pub(self) mod schema;
pub(self) use schema::*;

/* ------------------------------- Mod: Types ------------------------------- */

pub(self) mod types;
pub(self) use types::*;

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
pub(self) fn text<'src, I>()
-> impl Parser<'src, I, ast::Text, chumsky::extra::Err<ParseError<'src>>>
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
pub(self) fn ident<'src, I>()
-> impl Parser<'src, I, ast::Ident, chumsky::extra::Err<ParseError<'src>>>
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
pub(self) fn uint<'src, I>()
-> impl Parser<'src, I, ast::Uint, chumsky::extra::Err<ParseError<'src>>>
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
/// Test helper macro that lexes a string input and applies a parser.
///
/// Returns `(Option<T>, Vec<ParseError>)` where T is the parser output type.
///
/// # Example
/// ```
/// let (output, errors) = test_parse!("package foo;", package());
/// assert!(errors.is_empty());
/// let package = output.expect("should have output");
/// ```
///
/// # Implementation Note
/// This macro leaks the token vector to give it a 'static lifetime, which allows
/// the parse result to outlive the macro invocation. This is acceptable for tests
/// since test processes are short-lived.
macro_rules! test_parse {
    ($input:expr, $parser:expr) => {{
        let file = crate::core::SchemaImport::anonymous();
        let (tokens_opt, lex_errors) = crate::lex::lex($input, file.clone());

        assert!(
            lex_errors.is_empty(),
            "Lexing failed with {} error(s) for input:\n{}\n\nErrors:\n{:?}",
            lex_errors.len(),
            $input,
            lex_errors
        );

        let tokens = tokens_opt.expect("tokens should exist after successful lexing");
        // Leak the tokens to give them 'static lifetime so they outlive this macro invocation.
        // This is fine for tests since test processes are short-lived.
        let tokens: &'static _ = ::std::boxed::Box::leak(::std::boxed::Box::new(tokens));
        let eoi = tokens.last().map_or(0, |s| s.span.end());

        $parser
            .parse(tokens.as_slice().map(
                crate::lex::Span::new(file, 0..eoi),
                |s| (&s.inner, &s.span),
            ))
            .into_output_errors()
    }};
}

// Make the macro available to all modules under parse for testing
#[cfg(test)]
pub(self) use test_parse;

#[cfg(test)]
mod tests {
    use super::*;

    /* -------------------------- Tests: Shared Parsers ------------------------- */

    #[test]
    fn test_ident_simple_succeeds() {
        // Given: A simple identifier.
        let input = "foo";

        // When: The input is parsed.
        let (ident, errors) = test_parse!(input, ident());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let ident = ident.expect("should have output");

        // Then: The identifier has the correct name.
        assert_eq!(ident.name, "foo");
    }

    #[test]
    fn test_ident_with_underscores_succeeds() {
        // Given: An identifier with underscores.
        let input = "foo_bar_baz";

        // When: The input is parsed.
        let (ident, errors) = test_parse!(input, ident());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let ident = ident.expect("should have output");

        // Then: The identifier has the correct name.
        assert_eq!(ident.name, "foo_bar_baz");
    }

    #[test]
    fn test_ident_with_numbers_succeeds() {
        // Given: An identifier with numbers.
        let input = "foo123";

        // When: The input is parsed.
        let (ident, errors) = test_parse!(input, ident());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let ident = ident.expect("should have output");

        // Then: The identifier has the correct name.
        assert_eq!(ident.name, "foo123");
    }

    #[test]
    fn test_uint_small_value_succeeds() {
        // Given: A small unsigned integer.
        let input = "42";

        // When: The input is parsed.
        let (uint, errors) = test_parse!(input, uint());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let uint = uint.expect("should have output");

        // Then: The value is correct.
        assert_eq!(uint.value, 42);
    }

    #[test]
    fn test_uint_zero_succeeds() {
        // Given: Zero as an unsigned integer.
        let input = "0";

        // When: The input is parsed.
        let (uint, errors) = test_parse!(input, uint());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let uint = uint.expect("should have output");

        // Then: The value is zero.
        assert_eq!(uint.value, 0);
    }

    #[test]
    fn test_uint_large_value_succeeds() {
        // Given: A large unsigned integer.
        let input = "18446744073709551615"; // u64::MAX

        // When: The input is parsed.
        let (uint, errors) = test_parse!(input, uint());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let uint = uint.expect("should have output");

        // Then: The value is correct.
        assert_eq!(uint.value, u64::MAX);
    }

    #[test]
    fn test_text_simple_succeeds() {
        // Given: A simple string literal.
        let input = "\"hello\"";

        // When: The input is parsed.
        let (text, errors) = test_parse!(input, text());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let text = text.expect("should have output");

        // Then: The content is correct.
        assert_eq!(text.content, "hello");
    }

    #[test]
    fn test_text_with_spaces_succeeds() {
        // Given: A string literal with spaces.
        let input = "\"hello world\"";

        // When: The input is parsed.
        let (text, errors) = test_parse!(input, text());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let text = text.expect("should have output");

        // Then: The content includes spaces.
        assert_eq!(text.content, "hello world");
    }

    #[test]
    fn test_text_empty_succeeds() {
        // Given: An empty string literal.
        let input = "\"\"";

        // When: The input is parsed.
        let (text, errors) = test_parse!(input, text());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let text = text.expect("should have output");

        // Then: The content is empty.
        assert_eq!(text.content, "");
    }

    #[test]
    fn test_text_with_path_characters_succeeds() {
        // Given: A string literal with path-like characters.
        let input = "\"foo/bar/baz.baproto\"";

        // When: The input is parsed.
        let (text, errors) = test_parse!(input, text());

        // Then: Parsing succeeds with no errors.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let text = text.expect("should have output");

        // Then: The content preserves path characters.
        assert_eq!(text.content, "foo/bar/baz.baproto");
    }
}
