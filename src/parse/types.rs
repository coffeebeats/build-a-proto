use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                   Fn: typ                                  */
/* -------------------------------------------------------------------------- */

/// `typ` creates a new [`Parser`] that parses a type declaration into an
/// [`ast::Type`].
pub(super) fn typ<'src, I>()
-> impl Parser<'src, I, ast::Type, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    choice((
        array().map(ast::Type::Array),
        map().map(ast::Type::Map),
        scalar().map(ast::Type::Scalar),
        reference().map(ast::Type::Reference),
    ))
    .labelled("type")
    .boxed()
}

/* -------------------------------- Fn: array ------------------------------- */

/// `array` creates a new [`Parser`] that parses an array type declaration into
/// an [`ast::Array`].
pub(super) fn array<'src, I>()
-> impl Parser<'src, I, ast::Array, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::uint()
        .or_not()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar())
        .map_with(|(size, el), e| ast::Array {
            element: Box::new(ast::Type::Scalar(el)),
            size,
            span: e.span(),
        })
}

/* --------------------------------- Fn: map -------------------------------- */

/// `map` creates a new [`Parser`] that parses an map type declaration into a
/// [`ast::Map`].
pub(super) fn map<'src, I>()
-> impl Parser<'src, I, ast::Map, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::scalar()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(parse::scalar())
        .map_with(|(k, v), e| ast::Map {
            key: Box::new(ast::Type::Scalar(k)),
            value: Box::new(ast::Type::Scalar(v)),
            span: e.span(),
        })
}

/* ------------------------------ Fn: reference ----------------------------- */

/// `reference` creates a new [`Parser`] that parses a reference to another
/// named type into an [`ast::Reference`].
pub(super) fn reference<'src, I>()
-> impl Parser<'src, I, ast::Reference, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Dot)
        .or_not()
        .then(
            parse::ident()
                .separated_by(just(Token::Dot))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map_with(|(leading_dot, components), e| ast::Reference {
            components,
            is_absolute: leading_dot.is_some(),
            span: e.span(),
        })
}

/* ------------------------------- Fn: scalar ------------------------------- */

/// `scalar` creates a new [`Parser`] that parses a scalar type declaration into
/// an [`ast::Scalar`].
pub(super) fn scalar<'src, I>()
-> impl Parser<'src, I, ast::Scalar, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    select! {
        Token::Ident("bit") => ast::ScalarType::Bit,
        Token::Ident("bool") => ast::ScalarType::Bool,
        Token::Ident("byte") => ast::ScalarType::Byte,
        Token::Ident("u8") => ast::ScalarType::Uint8,
        Token::Ident("u16") => ast::ScalarType::Uint16,
        Token::Ident("u32") => ast::ScalarType::Uint32,
        Token::Ident("u64") => ast::ScalarType::Uint64,
        Token::Ident("i8") => ast::ScalarType::Int8,
        Token::Ident("i16") => ast::ScalarType::Int16,
        Token::Ident("i32") => ast::ScalarType::Int32,
        Token::Ident("i64") => ast::ScalarType::Int64,
        Token::Ident("f32") => ast::ScalarType::Float32,
        Token::Ident("f64") => ast::ScalarType::Float64,
        Token::Ident("string") => ast::ScalarType::String,
    }
    .map_with(|kind, e| ast::Scalar {
        kind,
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

    /* ----------------------------- Tests: type ---------------------------- */

    #[test]
    fn test_type_empty_brackets_fails() {
        // Given: Empty brackets without a base type.
        let input = "[]";

        // When: The input is parsed.
        assert_parse_fails(parse_single(input, typ()));
    }

    #[test]
    fn test_type_unclosed_array_brackets_fails() {
        // Given: Array type with unclosed brackets.
        let input = "[u8";

        // When: The input is parsed.
        assert_parse_fails(parse_single(input, typ()));
    }

    #[test]
    fn test_type_invalid_scalar_name_becomes_reference() {
        // Given: An invalid scalar type name.
        let input = "uint8"; // Should be u8, not uint8

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: It's parsed as a reference type.
        let ast::Type::Reference(r) = typ else {
            panic!("expected reference type");
        };
        assert_eq!(r.components[0].name, "uint8");
    }

    /* ---------------------------- Tests: scalar --------------------------- */

    #[test]
    fn test_type_scalar_bit_succeeds() {
        // Given: A bit type declaration.
        let input = "bit";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a bit scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type, got {:?}", typ);
        };
        assert_eq!(scalar.kind, ast::ScalarType::Bit);
    }

    #[test]
    fn test_type_scalar_bool_succeeds() {
        // Given: A bool type declaration.
        let input = "bool";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a bool scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Bool);
    }

    #[test]
    fn test_type_scalar_byte_succeeds() {
        // Given: A byte type declaration.
        let input = "byte";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a byte scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Byte);
    }

    #[test]
    fn test_type_scalar_u8_succeeds() {
        // Given: A u8 type declaration.
        let input = "u8";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a u8 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Uint8);
    }

    #[test]
    fn test_type_scalar_u16_succeeds() {
        // Given: A u16 type declaration.
        let input = "u16";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a u16 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Uint16);
    }

    #[test]
    fn test_type_scalar_u32_succeeds() {
        // Given: A u32 type declaration.
        let input = "u32";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a u32 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Uint32);
    }

    #[test]
    fn test_type_scalar_u64_succeeds() {
        // Given: A u64 type declaration.
        let input = "u64";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a u64 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Uint64);
    }

    #[test]
    fn test_type_scalar_i8_succeeds() {
        // Given: An i8 type declaration.
        let input = "i8";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an i8 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Int8);
    }

    #[test]
    fn test_type_scalar_i16_succeeds() {
        // Given: An i16 type declaration.
        let input = "i16";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an i16 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Int16);
    }

    #[test]
    fn test_type_scalar_i32_succeeds() {
        // Given: An i32 type declaration.
        let input = "i32";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an i32 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Int32);
    }

    #[test]
    fn test_type_scalar_i64_succeeds() {
        // Given: An i64 type declaration.
        let input = "i64";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an i64 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Int64);
    }

    #[test]
    fn test_type_scalar_f32_succeeds() {
        // Given: An f32 type declaration.
        let input = "f32";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an f32 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Float32);
    }

    #[test]
    fn test_type_scalar_f64_succeeds() {
        // Given: An f64 type declaration.
        let input = "f64";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an f64 scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Float64);
    }

    #[test]
    fn test_type_scalar_string_succeeds() {
        // Given: A string type declaration.
        let input = "string";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a string scalar.
        let ast::Type::Scalar(scalar) = typ else {
            panic!("expected scalar type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::String);
    }

    /* ---------------------------- Tests: array ---------------------------- */

    #[test]
    fn test_type_dynamic_array_succeeds() {
        // Given: A dynamic array type (no size).
        let input = "[]u32";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an array without a size.
        let ast::Type::Array(arr) = typ else {
            panic!("expected array type");
        };
        assert!(arr.size.is_none());

        // Then: The element type is correct.
        let ast::Type::Scalar(scalar) = *arr.element else {
            panic!("expected scalar element type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Uint32);
    }

    #[test]
    fn test_type_fixed_array_succeeds() {
        // Given: A fixed-size array type.
        let input = "[10]byte";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is an array with the specified size.
        let ast::Type::Array(arr) = typ else {
            panic!("expected array type");
        };
        assert_eq!(arr.size.as_ref().unwrap().value, 10);

        // Then: The element type is correct.
        let ast::Type::Scalar(scalar) = *arr.element else {
            panic!("expected scalar element type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::Byte);
    }

    #[test]
    fn test_type_array_with_large_size_succeeds() {
        // Given: An array with a large size.
        let input = "[1024]u8";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The array size is correct.
        let ast::Type::Array(arr) = typ else {
            panic!("expected array type");
        };
        assert_eq!(arr.size.as_ref().unwrap().value, 1024);
    }

    #[test]
    fn test_type_array_with_string_element_succeeds() {
        // Given: An array of strings.
        let input = "[]string";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The element type is string.
        let ast::Type::Array(arr) = typ else {
            panic!("expected array type");
        };
        let ast::Type::Scalar(scalar) = *arr.element else {
            panic!("expected scalar element type");
        };
        assert_eq!(scalar.kind, ast::ScalarType::String);
    }

    /* ----------------------------- Tests: map ----------------------------- */

    #[test]
    fn test_type_map_string_to_u64_succeeds() {
        // Given: A map type from string to u64.
        let input = "[string]u64";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The type is a map with correct key/value types.
        let ast::Type::Map(m) = typ else {
            panic!("expected map type");
        };

        // Then: The key type is string.
        let ast::Type::Scalar(key_scalar) = *m.key else {
            panic!("expected scalar key type");
        };
        assert_eq!(key_scalar.kind, ast::ScalarType::String);

        // Then: The value type is u64.
        let ast::Type::Scalar(value_scalar) = *m.value else {
            panic!("expected scalar value type");
        };
        assert_eq!(value_scalar.kind, ast::ScalarType::Uint64);
    }

    #[test]
    fn test_type_map_u32_to_string_succeeds() {
        // Given: A map type from u32 to string.
        let input = "[u32]string";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The types are correct.
        let ast::Type::Map(m) = typ else {
            panic!("expected map type");
        };
        let ast::Type::Scalar(key) = *m.key else {
            panic!("expected scalar key");
        };
        assert_eq!(key.kind, ast::ScalarType::Uint32);
        let ast::Type::Scalar(value) = *m.value else {
            panic!("expected scalar value");
        };
        assert_eq!(value.kind, ast::ScalarType::String);
    }

    #[test]
    fn test_type_map_bool_to_byte_succeeds() {
        // Given: A map type from bool to byte.
        let input = "[bool]byte";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The types are correct.
        let ast::Type::Map(m) = typ else {
            panic!("expected map type");
        };
        let ast::Type::Scalar(key) = *m.key else {
            panic!("expected scalar key");
        };
        assert_eq!(key.kind, ast::ScalarType::Bool);
        let ast::Type::Scalar(value) = *m.value else {
            panic!("expected scalar value");
        };
        assert_eq!(value.kind, ast::ScalarType::Byte);
    }

    /* -------------------------- Tests: reference -------------------------- */

    #[test]
    fn test_type_simple_reference_succeeds() {
        // Given: A simple type reference.
        let input = "MyType";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The reference is relative with one component.
        let ast::Type::Reference(r) = typ else {
            panic!("expected reference type");
        };
        assert!(!r.is_absolute);
        assert_eq!(r.components.len(), 1);
        assert_eq!(r.components[0].name, "MyType");
    }

    #[test]
    fn test_type_relative_reference_succeeds() {
        // Given: A relative type reference with multiple components.
        let input = "foo.bar.MyType";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The reference is relative with correct components.
        let ast::Type::Reference(r) = typ else {
            panic!("expected reference type");
        };
        assert!(!r.is_absolute);
        assert_eq!(r.components.len(), 3);
        assert_eq!(r.components[0].name, "foo");
        assert_eq!(r.components[1].name, "bar");
        assert_eq!(r.components[2].name, "MyType");
    }

    #[test]
    fn test_type_absolute_reference_succeeds() {
        // Given: An absolute type reference (leading dot).
        let input = ".root.MyType";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The reference is absolute.
        let ast::Type::Reference(r) = typ else {
            panic!("expected reference type");
        };
        assert!(r.is_absolute);
        assert_eq!(r.components.len(), 2);
        assert_eq!(r.components[0].name, "root");
        assert_eq!(r.components[1].name, "MyType");
    }

    #[test]
    fn test_type_absolute_single_component_reference_succeeds() {
        // Given: An absolute reference with a single component.
        let input = ".Foo";

        // When: The input is parsed.
        let typ = assert_parse_succeeds(parse_single(input, typ()));

        // Then: The reference is absolute with one component.
        let ast::Type::Reference(r) = typ else {
            panic!("expected reference type");
        };
        assert!(r.is_absolute);
        assert_eq!(r.components.len(), 1);
        assert_eq!(r.components[0].name, "Foo");
    }
}
