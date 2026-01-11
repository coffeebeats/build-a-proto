use std::cell::RefCell;

use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Message                                */
/* -------------------------------------------------------------------------- */

/// `message` creates a new [`Parser`] that parses a message definition into an
/// [`ast::Message`].
pub(super) fn message<'src, I>(
    depth_limit: usize,
) -> impl Parser<'src, I, ast::Message, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let depth = RefCell::new(depth_limit);

    recursive(move |msg| {
        parse::comment_block()
            .or_not()
            .then(just(Token::Keyword(Keyword::Message)).ignore_then(parse::ident()))
            .then(
                choice((
                    msg.map(ast::MessageItem::Message),
                    parse::enumeration().map(ast::MessageItem::Enum),
                    field().map(ast::MessageItem::Field),
                    parse::comment_block().map(ast::MessageItem::CommentBlock),
                ))
                // FIXME: Handle newlines before, between, and after.
                .separated_by(just(Token::Newline).repeated())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<ast::MessageItem>>()
                .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
            )
            .then_ignore(just(Token::Newline).repeated())
            .map_with(|((comment, name), items), e| ast::Message {
                comment,
                items,
                name,
                span: e.span(),
            })
            .labelled("message")
            .try_map(move |m, span| {
                if let Ok(mut depth) = depth.try_borrow_mut() {
                    *depth = depth.saturating_sub(1);

                    if *depth == 0 {
                        let msg = format!("exceeded maximum type depth limit: {}", depth_limit);
                        return Err(Rich::custom(span, msg));
                    }
                }

                Ok(m)
            })
            .boxed()
    })
}

/* -------------------------------- Fn: field ------------------------------- */

/// `field` creates a new [`Parser`] that parses a message field into an
/// [`ast::Field`].
pub(super) fn field<'src, I>()
-> impl Parser<'src, I, ast::Field, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(field_index().or_not())
        .then(parse::typ())
        .then(parse::ident())
        .then(just(Token::Equal).ignore_then(parse::encoding()).or_not())
        .then_ignore(just(Token::Semicolon))
        .map_with(
            |((((comment, index), typ), name), encoding), e| ast::Field {
                comment,
                encoding,
                index,
                kind: typ,
                name,
                span: e.span(),
            },
        )
        .labelled("field")
        .boxed()
}

/* ----------------------------- Fn: field_index ---------------------------- */

/// `field_index` creates a new [`Parser`] that parses a field or variant index
/// into an [`ast::FieldIndex`].
pub(super) fn field_index<'src, I>()
-> impl Parser<'src, I, ast::FieldIndex, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::uint()
        .then_ignore(just(Token::Colon))
        .map_with(|value, e| ast::FieldIndex {
            span: e.span(),
            value,
        })
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::tests::*;

    /* --------------------------- Tests: message --------------------------- */

    #[test]
    fn test_message_missing_name_fails() {
        // Given: A message declaration missing the name.
        let input = "message {}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_message_field_missing_semicolon_fails() {
        // Given: A field without a semicolon.
        let input = "message Foo {\nu8 value\n}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_message_missing_closing_brace_fails() {
        // Given: A message missing the closing brace.
        let input = "message Foo {\nu8 value;";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_message_field_invalid_index_syntax_fails() {
        // Given: A field with invalid index syntax (missing colon).
        let input = "message Foo {\n0 u8 value;\n}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_message_field_missing_type_fails() {
        // Given: A field missing its type.
        let input = "message Foo {\nvalue;\n}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_message_field_missing_name_fails() {
        // Given: A field missing its name.
        let input = "message Foo {\nu8;\n}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_message_exceeding_depth_limit_fails() {
        // Given: Nested messages exceeding a depth limit of 3.
        let depth_limit = 3;
        let input = "message L1 {\nmessage L2 {\nmessage L3 {\nmessage L4 {\nu8 value;\n}\n}\n}\n}";

        // When: The input is parsed with depth limit of 3.
        let (_result, errors): (Option<ast::Message>, _) =
            parse_single(input, message(depth_limit));

        // Then: Parsing fails with a depth limit error.
        assert!(!errors.is_empty(), "expected parsing to fail");

        // Then: The error message contains the correct depth limit.
        let error_message = format!("{}", errors[0]);
        assert!(
            error_message.contains(&format!(
                "exceeded maximum type depth limit: {}",
                depth_limit
            )),
            "expected error message to contain correct depth limit, got: {}",
            error_message
        );
    }

    #[test]
    fn test_message_empty_body_succeeds() {
        // Given: A message with an empty body.
        let input = "message Foo {}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The message has the correct name and no items.
        assert_eq!(msg.name.name, "Foo");
        assert!(msg.items.is_empty());
        assert!(msg.comment.is_none());
    }

    #[test]
    fn test_message_single_field_succeeds() {
        // Given: A message with a single field.
        let input = "message Player {\nu8 health;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The message has one field.
        assert_eq!(msg.name.name, "Player");
        assert_eq!(msg.items.len(), 1);

        let ast::MessageItem::Field(field) = &msg.items[0] else {
            panic!("expected field");
        };
        assert_eq!(field.name.name, "health");
        assert!(field.index.is_none());
        assert!(field.encoding.is_none());
        assert!(field.comment.is_none());
    }

    #[test]
    fn test_message_multiple_fields_succeeds() {
        // Given: A message with multiple fields.
        let input = "message Player {\nu8 health;\nu32 score;\nstring name;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: All fields are present.
        assert_eq!(msg.items.len(), 3);

        let ast::MessageItem::Field(f1) = &msg.items[0] else {
            panic!("expected field");
        };
        assert_eq!(f1.name.name, "health");

        let ast::MessageItem::Field(f2) = &msg.items[1] else {
            panic!("expected field");
        };
        assert_eq!(f2.name.name, "score");

        let ast::MessageItem::Field(f3) = &msg.items[2] else {
            panic!("expected field");
        };
        assert_eq!(f3.name.name, "name");
    }

    #[test]
    fn test_message_field_with_explicit_index_succeeds() {
        // Given: A message field with an explicit index.
        let input = "message Data {\n0: u32 value;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The field has the correct index.
        let ast::MessageItem::Field(field) = &msg.items[0] else {
            panic!("expected field");
        };
        assert!(field.index.is_some());
        assert_eq!(field.index.as_ref().unwrap().value.value, 0);
    }

    #[test]
    fn test_message_fields_with_multiple_indices_succeeds() {
        // Given: A message where all fields have explicit indices.
        let input = "message Data {\n0: u8 a;\n1: u16 b;\n5: u32 c;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: All fields have their indices.
        assert_eq!(msg.items.len(), 3);

        let ast::MessageItem::Field(f1) = &msg.items[0] else {
            panic!("expected field");
        };
        assert_eq!(f1.index.as_ref().unwrap().value.value, 0);

        let ast::MessageItem::Field(f2) = &msg.items[1] else {
            panic!("expected field");
        };
        assert_eq!(f2.index.as_ref().unwrap().value.value, 1);

        let ast::MessageItem::Field(f3) = &msg.items[2] else {
            panic!("expected field");
        };
        assert_eq!(f3.index.as_ref().unwrap().value.value, 5);
    }

    #[test]
    fn test_message_field_with_encoding_succeeds() {
        // Given: A message field with an encoding.
        let input = "message Data {\nu32 value = delta;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The field has an encoding.
        let ast::MessageItem::Field(field) = &msg.items[0] else {
            panic!("expected field");
        };
        assert!(field.encoding.is_some());
        let encoding = field.encoding.as_ref().unwrap();
        assert_eq!(encoding.encodings.len(), 1);
        assert!(matches!(encoding.encodings[0], ast::EncodingKind::Delta));
    }

    #[test]
    fn test_message_field_with_complex_encoding_succeeds() {
        // Given: A field with a complex encoding.
        let input = "message Data {\nu32 value = [\ndelta,\nzigzag,\nbits(16)];\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The encoding has all components.
        let ast::MessageItem::Field(field) = &msg.items[0] else {
            panic!("expected field");
        };
        assert!(field.encoding.is_some());
        let encoding = field.encoding.as_ref().unwrap();
        assert_eq!(encoding.encodings.len(), 3);
    }

    #[test]
    fn test_message_field_with_index_and_encoding_succeeds() {
        // Given: A field with both index and encoding.
        let input = "message Data {\n0: u32 value = delta;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: Both index and encoding are present.
        let ast::MessageItem::Field(field) = &msg.items[0] else {
            panic!("expected field");
        };
        assert!(field.index.is_some());
        assert_eq!(field.index.as_ref().unwrap().value.value, 0);
        assert!(field.encoding.is_some());
    }

    #[test]
    fn test_message_with_doc_comment_succeeds() {
        // Given: A message with a preceding doc comment.
        let input = "// Player data\nmessage Player {\nu8 health;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The doc comment is captured.
        assert!(msg.comment.is_some());
        let comment = msg.comment.as_ref().unwrap();
        assert_eq!(comment.comments.len(), 1);
        assert_eq!(comment.comments[0].content, "Player data");
    }

    #[test]
    fn test_message_field_with_doc_comment_succeeds() {
        // Given: A message with a field that has a doc comment.
        let input = "message Player {\n// Current health\nu8 health;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The field's doc comment is captured.
        let ast::MessageItem::Field(field) = &msg.items[0] else {
            panic!("expected field");
        };
        assert!(field.comment.is_some());
        let comment = field.comment.as_ref().unwrap();
        assert_eq!(comment.comments.len(), 1);
        assert_eq!(comment.comments[0].content, "Current health");
    }

    #[test]
    fn test_message_with_nested_message_succeeds() {
        // Given: A message with a nested message.
        let input = "message Outer {\nmessage Inner {\nu8 value;\n}\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The nested message is present.
        assert_eq!(msg.name.name, "Outer");
        assert_eq!(msg.items.len(), 1);

        let ast::MessageItem::Message(inner) = &msg.items[0] else {
            panic!("expected nested message");
        };
        assert_eq!(inner.name.name, "Inner");
        assert_eq!(inner.items.len(), 1);
    }

    #[test]
    fn test_message_with_nested_enum_succeeds() {
        // Given: A message with a nested enum.
        let input = "message Container {\nenum Status {\nOK;\nERROR;\n}\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: The nested enum is present.
        assert_eq!(msg.items.len(), 1);

        let ast::MessageItem::Enum(enum_item) = &msg.items[0] else {
            panic!("expected nested enum");
        };
        assert_eq!(enum_item.name.name, "Status");
        assert_eq!(enum_item.items.len(), 2);
    }

    #[test]
    fn test_message_mixed_item_types_succeeds() {
        // Given: A message with mixed item types.
        let input =
            "message Complex {\nu8 field1;\nmessage Nested {}\nenum Status { OK; }\nu32 field2;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: All items are present in order.
        assert_eq!(msg.items.len(), 4);

        assert!(matches!(msg.items[0], ast::MessageItem::Field(_)));
        assert!(matches!(msg.items[1], ast::MessageItem::Message(_)));
        assert!(matches!(msg.items[2], ast::MessageItem::Enum(_)));
        assert!(matches!(msg.items[3], ast::MessageItem::Field(_)));
    }

    #[test]
    fn test_message_deeply_nested_succeeds() {
        // Given: Messages nested to depth 3.
        let input = "message L1 {\nmessage L2 {\nmessage L3 {\nu8 value;\n}\n}\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: All nesting levels are present.
        assert_eq!(msg.name.name, "L1");

        let ast::MessageItem::Message(l2) = &msg.items[0] else {
            panic!("expected nested message");
        };
        assert_eq!(l2.name.name, "L2");

        let ast::MessageItem::Message(l3) = &l2.items[0] else {
            panic!("expected nested message");
        };
        assert_eq!(l3.name.name, "L3");
    }

    #[test]
    fn test_message_with_various_field_types_succeeds() {
        // Given: A message with fields of various types.
        let input = "message Data {\nu8 a;\nstring b;\n[]u32 c;\n[5]bool d;\n[string]u64 e;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: All field types are parsed correctly.
        assert_eq!(msg.items.len(), 5);

        for item in &msg.items {
            assert!(matches!(item, ast::MessageItem::Field(_)));
        }
    }

    #[test]
    fn test_message_with_trailing_newlines_succeeds() {
        // Given: A message with trailing newlines in the body.
        let input = "message Foo {\nu8 value;\n\n\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: Trailing newlines don't create extra items.
        assert_eq!(msg.items.len(), 1);
    }

    #[test]
    fn test_message_with_leading_newlines_succeeds() {
        // Given: A message with leading newlines in the body.
        let input = "message Foo {\n\n\nu8 value;\n}";

        // When: The input is parsed.
        let (msg, errors): (Option<ast::Message>, _) =
            parse_single(input, message(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let msg = msg.expect("should have output");

        // Then: Leading newlines don't create extra items.
        assert_eq!(msg.items.len(), 1);
    }

    /* ------------------------- Tests: field_index ------------------------- */

    #[test]
    fn test_field_index_zero_succeeds() {
        // Given: A field index of zero.
        let input = "0:";

        // When: The input is parsed.
        let index = assert_parse_succeeds(parse_single(input, field_index()));

        // Then: The index value is correct.
        assert_eq!(index.value.value, 0);
    }

    #[test]
    fn test_field_index_large_value_succeeds() {
        // Given: A field index with a large value.
        let input = "12345:";

        // When: The input is parsed.
        let index = assert_parse_succeeds(parse_single(input, field_index()));

        // Then: The index value is correct.
        assert_eq!(index.value.value, 12345);
    }

    #[test]
    fn test_field_index_missing_colon_fails() {
        // Given: A field index missing the colon.
        let input = "0";

        // When: The input is parsed.
        assert_parse_fails(parse_single(input, field_index()));
    }
}
