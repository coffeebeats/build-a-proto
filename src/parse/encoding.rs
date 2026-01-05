use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                Fn: encoding                                */
/* -------------------------------------------------------------------------- */

/// `encoding` creates a new [`Parser`] that parses either a single encoding
/// definition or a list of them into an [`ast::Encoding`].
pub(super) fn encoding<'src, I>()
-> impl Parser<'src, I, ast::Encoding, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    // FIXME: Not all encoding kinds get spanned correctly?
    choice((
        // Single encoding
        parse::encoding_kind().map(|enc| vec![enc]),
        // Multiple encodings
        parse::encoding_kind()
            // FIXME: Handle no newlines case.
            .separated_by(just(Token::Comma).then(just(Token::Newline).repeated()))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(
                just(Token::ListOpen).then(just(Token::Newline).repeated()),
                just(Token::Newline).repeated().then(just(Token::ListClose)),
            ),
    ))
    .map_with(|encodings, e| ast::Encoding {
        encodings,
        span: e.span(),
    })
}

/* -------------------------------------------------------------------------- */
/*                              Fn: encoding_kind                             */
/* -------------------------------------------------------------------------- */

/// `encoding_kind` creates a new [`Parser`] that parses an encoding into an
/// [`ast::EncodingKind`].
pub(super) fn encoding_kind<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    choice((
        bits(),
        bits_variable(),
        fixed_point(),
        delta(),
        zigzag(),
        pad(),
    ))
    .labelled("encoding")
    .boxed()
}

/* -------------------------------- Fn: bits -------------------------------- */

fn bits<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("bits"))
        .ignore_then(parse::uint().delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(ast::EncodingKind::Bits)
}

/* ---------------------------- Fn: bits_variable --------------------------- */

fn bits_variable<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("bits"))
        .ignore_then(
            just(Token::Ident("var"))
                .ignore_then(parse::uint().delimited_by(just(Token::FnOpen), just(Token::FnClose)))
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(ast::EncodingKind::BitsVariable)
}

/* ----------------------------- Fn: fixed_point ---------------------------- */

fn fixed_point<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("fixed_point"))
        .ignore_then(
            parse::uint()
                .separated_by(just(Token::Comma))
                .exactly(2)
                .collect()
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(|args: Vec<ast::Uint>| ast::EncodingKind::FixedPoint(args[0].clone(), args[1].clone()))
}

/* -------------------------------- Fn: delta ------------------------------- */

fn delta<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("delta")).map(|_| ast::EncodingKind::Delta)
}

/* ------------------------------- Fn: zigzag ------------------------------- */

fn zigzag<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("zigzag")).map(|_| ast::EncodingKind::ZigZag)
}

/* --------------------------------- Fn: pad -------------------------------- */

fn pad<'src, I>()
-> impl Parser<'src, I, ast::EncodingKind, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Ident("pad"))
        .ignore_then(parse::uint().delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(ast::EncodingKind::Pad)
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::test_parse;

    /* -------------------- Single Encoding Tests ----------------------- */

    #[test]
    fn test_encoding_delta_succeeds() {
        // Given: A delta encoding.
        let input = "delta";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let encoding = encoding.expect("should have output");

        // Then: The encoding is delta.
        assert_eq!(encoding.encodings.len(), 1);
        assert!(matches!(encoding.encodings[0], ast::EncodingKind::Delta));
    }

    #[test]
    fn test_encoding_zigzag_succeeds() {
        // Given: A zigzag encoding.
        let input = "zigzag";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: The encoding is zigzag.
        assert_eq!(encoding.encodings.len(), 1);
        assert!(matches!(encoding.encodings[0], ast::EncodingKind::ZigZag));
    }

    #[test]
    fn test_encoding_bits_succeeds() {
        // Given: A bits encoding with a value.
        let input = "bits(8)";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: The encoding is bits with the correct value.
        assert_eq!(encoding.encodings.len(), 1);
        let ast::EncodingKind::Bits(n) = &encoding.encodings[0] else {
            panic!("expected Bits encoding");
        };
        assert_eq!(n.value, 8);
    }

    #[test]
    fn test_encoding_bits_with_large_value_succeeds() {
        // Given: A bits encoding with a large value.
        let input = "bits(128)";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: The value is correct.
        let ast::EncodingKind::Bits(n) = &encoding.encodings[0] else {
            panic!("expected Bits encoding");
        };
        assert_eq!(n.value, 128);
    }

    #[test]
    fn test_encoding_bits_variable_succeeds() {
        // Given: A variable-length bits encoding.
        let input = "bits(var(16))";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: The encoding is bits variable with correct value.
        assert_eq!(encoding.encodings.len(), 1);
        let ast::EncodingKind::BitsVariable(n) = &encoding.encodings[0] else {
            panic!("expected BitsVariable encoding");
        };
        assert_eq!(n.value, 16);
    }

    #[test]
    fn test_encoding_fixed_point_succeeds() {
        // Given: A fixed-point encoding.
        let input = "fixed_point(16, 8)";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: The encoding has both integer and fractional parts.
        assert_eq!(encoding.encodings.len(), 1);
        let ast::EncodingKind::FixedPoint(int_bits, frac_bits) = &encoding.encodings[0] else {
            panic!("expected FixedPoint encoding");
        };
        assert_eq!(int_bits.value, 16);
        assert_eq!(frac_bits.value, 8);
    }

    #[test]
    fn test_encoding_fixed_point_with_same_values_succeeds() {
        // Given: A fixed-point encoding with equal integer and fractional bits.
        let input = "fixed_point(12, 12)";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: Both parts are correct.
        let ast::EncodingKind::FixedPoint(int_bits, frac_bits) = &encoding.encodings[0] else {
            panic!("expected FixedPoint encoding");
        };
        assert_eq!(int_bits.value, 12);
        assert_eq!(frac_bits.value, 12);
    }

    #[test]
    fn test_encoding_pad_succeeds() {
        // Given: A pad encoding.
        let input = "pad(4)";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: The encoding is pad with correct bits.
        assert_eq!(encoding.encodings.len(), 1);
        let ast::EncodingKind::Pad(n) = &encoding.encodings[0] else {
            panic!("expected Pad encoding");
        };
        assert_eq!(n.value, 4);
    }

    /* -------------------- Multiple Encodings Tests -------------------- */

    #[test]
    fn test_encoding_list_with_two_encodings_succeeds() {
        // Given: Multiple encodings in a list.
        let input = "[\ndelta,\nbits(8)]";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let encoding = encoding.expect("should have output");

        // Then: Both encodings are present.
        assert_eq!(encoding.encodings.len(), 2);
        assert!(matches!(encoding.encodings[0], ast::EncodingKind::Delta));
        let ast::EncodingKind::Bits(n) = &encoding.encodings[1] else {
            panic!("expected Bits encoding");
        };
        assert_eq!(n.value, 8);
    }

    #[test]
    fn test_encoding_list_with_three_encodings_succeeds() {
        // Given: Three encodings in a list.
        let input = "[\ndelta,\nzigzag,\nbits(16)]";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: All encodings are present.
        assert_eq!(encoding.encodings.len(), 3);
        assert!(matches!(encoding.encodings[0], ast::EncodingKind::Delta));
        assert!(matches!(encoding.encodings[1], ast::EncodingKind::ZigZag));
        let ast::EncodingKind::Bits(n) = &encoding.encodings[2] else {
            panic!("expected Bits encoding");
        };
        assert_eq!(n.value, 16);
    }

    #[test]
    fn test_encoding_list_with_trailing_comma_succeeds() {
        // Given: An encoding list with a trailing comma.
        let input = "[\ndelta,\nzigzag,\n]";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds with trailing comma.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: Both encodings are present (trailing comma ignored).
        assert_eq!(encoding.encodings.len(), 2);
    }

    #[test]
    fn test_encoding_list_with_complex_encodings_succeeds() {
        // Given: A list with complex parameterized encodings.
        let input = "[\nfixed_point(16, 8),\nbits(var(32)),\npad(4)]";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds.
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: All encodings are correct.
        assert_eq!(encoding.encodings.len(), 3);

        // Then: Fixed point encoding is correct.
        let ast::EncodingKind::FixedPoint(int_bits, frac_bits) = &encoding.encodings[0] else {
            panic!("expected FixedPoint encoding");
        };
        assert_eq!(int_bits.value, 16);
        assert_eq!(frac_bits.value, 8);

        // Then: Bits variable encoding is correct.
        let ast::EncodingKind::BitsVariable(n) = &encoding.encodings[1] else {
            panic!("expected BitsVariable encoding");
        };
        assert_eq!(n.value, 32);

        // Then: Pad encoding is correct.
        let ast::EncodingKind::Pad(n) = &encoding.encodings[2] else {
            panic!("expected Pad encoding");
        };
        assert_eq!(n.value, 4);
    }

    /* ----------------------- Error Cases -------------------------- */

    #[test]
    fn test_encoding_bits_without_value_fails() {
        // Given: A bits encoding missing its value.
        let input = "bits()";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_encoding_fixed_point_with_one_arg_fails() {
        // Given: A fixed_point with only one argument.
        let input = "fixed_point(16)";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_encoding_fixed_point_with_three_args_fails() {
        // Given: A fixed_point with three arguments.
        let input = "fixed_point(16, 8, 4)";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_encoding_unknown_name_fails() {
        // Given: An unknown encoding name.
        let input = "unknown_encoding";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_encoding_bits_variable_missing_var_fails() {
        // Given: bits with parentheses but missing 'var'.
        let input = "bits(16)";

        // When: The input is parsed.
        let (encoding, errors): (Option<ast::Encoding>, _) = test_parse!(input, encoding());

        // Then: Parsing succeeds as regular bits (not bits_variable).
        assert!(errors.is_empty());
        let encoding = encoding.expect("should have output");

        // Then: It's parsed as Bits, not BitsVariable.
        assert!(matches!(encoding.encodings[0], ast::EncodingKind::Bits(_)));
    }
}
