use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                 Fn: schema                                 */
/* -------------------------------------------------------------------------- */

// FIXME: HANDLE LINE COMMENTS EVERYWHERE
// FIXME: ADD ERROR RECOVERY (`.recover_with(skip_then_retry_until(any().ignored(), end()))`)

/// `schema` creates a new [`Parser`] that parses a 'baproto' schema.
pub(super) fn schema<'src, I>(
    depth_limit: usize,
) -> impl Parser<'src, I, ast::Schema, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    choice((
        parse::package().map(ast::SchemaItem::Package),
        parse::comment_block().map(ast::SchemaItem::CommentBlock),
    ))
    .separated_by(just(Token::Newline).repeated())
    .allow_leading()
    .allow_trailing()
    .collect::<Vec<_>>()
    .foldl(
        choice((
            parse::import().map(ast::SchemaItem::Include),
            parse::enumeration().map(ast::SchemaItem::Enum),
            parse::message(depth_limit).map(ast::SchemaItem::Message),
            parse::comment_block().map(ast::SchemaItem::CommentBlock),
        ))
        .separated_by(just(Token::Newline).repeated())
        .allow_leading()
        .allow_trailing(),
        |mut a, b| {
            a.push(b);
            a
        },
    )
    .map_with(|items, e| ast::Schema {
        items,
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

    /* ---------------------------- Tests: Schema --------------------------- */

    #[test]
    fn test_schema_package_only_succeeds() {
        // Given: A schema with only a package declaration.
        let input = "package foo;";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: The schema has one package item.
        assert_eq!(schema.items.len(), 1);

        let ast::SchemaItem::Package(pkg) = &schema.items[0] else {
            panic!("expected package");
        };
        assert_eq!(pkg.components.len(), 1);
        assert_eq!(pkg.components[0].name, "foo");
    }

    #[test]
    fn test_schema_package_with_imports_succeeds() {
        // Given: A schema with package and imports.
        let input = "package foo;\ninclude \"bar.baproto\";\ninclude \"baz.baproto\";";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: All items are present.
        assert_eq!(schema.items.len(), 3);

        assert!(matches!(schema.items[0], ast::SchemaItem::Package(_)));
        assert!(matches!(schema.items[1], ast::SchemaItem::Include(_)));
        assert!(matches!(schema.items[2], ast::SchemaItem::Include(_)));
    }

    #[test]
    fn test_schema_package_with_message_succeeds() {
        // Given: A schema with package and message.
        let input = "package foo;\nmessage Bar {\nu8 value;\n}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Package and message are present.
        assert_eq!(schema.items.len(), 2);

        let ast::SchemaItem::Package(pkg) = &schema.items[0] else {
            panic!("expected package");
        };
        assert_eq!(pkg.components[0].name, "foo");

        let ast::SchemaItem::Message(msg) = &schema.items[1] else {
            panic!("expected message");
        };
        assert_eq!(msg.name.name, "Bar");
    }

    #[test]
    fn test_schema_package_with_enum_succeeds() {
        // Given: A schema with package and enum.
        let input = "package foo;\nenum Status {\nOK;\n}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Package and enum are present.
        assert_eq!(schema.items.len(), 2);

        assert!(matches!(schema.items[0], ast::SchemaItem::Package(_)));

        let ast::SchemaItem::Enum(enum_item) = &schema.items[1] else {
            panic!("expected enum");
        };
        assert_eq!(enum_item.name.name, "Status");
    }

    #[test]
    fn test_schema_full_structure_succeeds() {
        // Given: A complete schema with all item types.
        let input = "package foo.bar;\ninclude \"common.baproto\";\nmessage Data {\nu8 value;\n}\nenum Status {\nOK;\n}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: All items are present in order.
        assert_eq!(schema.items.len(), 4);

        assert!(matches!(schema.items[0], ast::SchemaItem::Package(_)));
        assert!(matches!(schema.items[1], ast::SchemaItem::Include(_)));
        assert!(matches!(schema.items[2], ast::SchemaItem::Message(_)));
        assert!(matches!(schema.items[3], ast::SchemaItem::Enum(_)));
    }

    #[test]
    fn test_schema_multiple_messages_succeeds() {
        // Given: A schema with multiple messages.
        let input = "package test;\nmessage First {}\nmessage Second {}\nmessage Third {}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: All messages are present.
        assert_eq!(schema.items.len(), 4);

        let ast::SchemaItem::Message(m1) = &schema.items[1] else {
            panic!("expected message");
        };
        assert_eq!(m1.name.name, "First");

        let ast::SchemaItem::Message(m2) = &schema.items[2] else {
            panic!("expected message");
        };
        assert_eq!(m2.name.name, "Second");

        let ast::SchemaItem::Message(m3) = &schema.items[3] else {
            panic!("expected message");
        };
        assert_eq!(m3.name.name, "Third");
    }

    #[test]
    fn test_schema_multiple_imports_succeeds() {
        // Given: A schema with multiple imports.
        let input = "package test;\ninclude \"a.baproto\";\ninclude \"b.baproto\";\ninclude \"c/d.baproto\";";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: All imports are present.
        assert_eq!(schema.items.len(), 4);

        let ast::SchemaItem::Include(i1) = &schema.items[1] else {
            panic!("expected include");
        };
        assert_eq!(i1.path.to_str().unwrap(), "a.baproto");

        let ast::SchemaItem::Include(i2) = &schema.items[2] else {
            panic!("expected include");
        };
        assert_eq!(i2.path.to_str().unwrap(), "b.baproto");

        let ast::SchemaItem::Include(i3) = &schema.items[3] else {
            panic!("expected include");
        };
        assert_eq!(i3.path.to_str().unwrap(), "c/d.baproto");
    }

    #[test]
    fn test_schema_with_doc_comment_succeeds() {
        // Given: A schema with a doc comment on the package.
        let input = "// Package documentation\npackage foo;";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: The package has a doc comment.
        let ast::SchemaItem::Package(pkg) = &schema.items[0] else {
            panic!("expected package");
        };
        assert!(pkg.comment.is_some());
        let comment = pkg.comment.as_ref().unwrap();
        assert_eq!(comment.comments[0].content, "Package documentation");
    }

    #[test]
    fn test_schema_comments_between_declarations_succeeds() {
        // Given: A schema with comments between declarations.
        let input = "package foo;\n// Import section\ninclude \"bar.baproto\";\n// Message section\nmessage Data {}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Schema has package, include and message (comments get parsed differently).
        // Note: Comments between package/include aren't standalone since include doesn't support comments.
        assert_eq!(
            schema.items.len(),
            4,
            "items: {:?}",
            schema
                .items
                .iter()
                .map(|i| match i {
                    ast::SchemaItem::Package(_) => "Package",
                    ast::SchemaItem::Include(_) => "Include",
                    ast::SchemaItem::Message(_) => "Message",
                    ast::SchemaItem::Enum(_) => "Enum",
                    ast::SchemaItem::CommentBlock(_) => "CommentBlock",
                })
                .collect::<Vec<_>>()
        );

        assert!(matches!(schema.items[0], ast::SchemaItem::Package(_)));
        assert!(matches!(schema.items[1], ast::SchemaItem::CommentBlock(_)));
        assert!(matches!(schema.items[2], ast::SchemaItem::Include(_)));
        assert!(matches!(schema.items[3], ast::SchemaItem::Message(_)));
    }

    #[test]
    fn test_schema_mixed_declaration_order_succeeds() {
        // Given: A schema with declarations in varied order.
        let input = "package test;\nenum First {}\nmessage Second {}\ninclude \"third.baproto\";\nenum Fourth {}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Items are in declaration order.
        assert_eq!(schema.items.len(), 5);

        assert!(matches!(schema.items[0], ast::SchemaItem::Package(_)));
        assert!(matches!(schema.items[1], ast::SchemaItem::Enum(_)));
        assert!(matches!(schema.items[2], ast::SchemaItem::Message(_)));
        assert!(matches!(schema.items[3], ast::SchemaItem::Include(_)));
        assert!(matches!(schema.items[4], ast::SchemaItem::Enum(_)));
    }

    #[test]
    fn test_schema_with_complex_package_name_succeeds() {
        // Given: A schema with a multi-component package name.
        let input = "package com.example.myapp.models;";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: All package components are present.
        let ast::SchemaItem::Package(pkg) = &schema.items[0] else {
            panic!("expected package");
        };
        assert_eq!(pkg.components.len(), 4);
        assert_eq!(pkg.components[0].name, "com");
        assert_eq!(pkg.components[1].name, "example");
        assert_eq!(pkg.components[2].name, "myapp");
        assert_eq!(pkg.components[3].name, "models");
    }

    #[test]
    fn test_schema_with_leading_newlines_succeeds() {
        // Given: A schema with leading newlines.
        let input = "\n\n\npackage foo;";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Leading newlines don't create extra items.
        assert_eq!(schema.items.len(), 1);
    }

    #[test]
    fn test_schema_with_trailing_newlines_succeeds() {
        // Given: A schema with trailing newlines.
        let input = "package foo;\n\n\n";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Trailing newlines don't create extra items.
        assert_eq!(schema.items.len(), 1);
    }

    #[test]
    fn test_schema_with_newlines_between_items_succeeds() {
        // Given: A schema with multiple newlines between items.
        let input = "package foo;\n\n\nmessage Bar {}\n\n\nenum Baz {}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let schema = schema.expect("should have output");

        // Then: Extra newlines don't create extra items.
        assert_eq!(schema.items.len(), 3);
    }

    /* ----------------------- Error Cases -------------------------- */

    #[test]
    fn test_schema_missing_package_succeeds_with_empty_items() {
        // Given: A schema missing the package declaration.
        let input = "message Foo {}";

        // When: The input is parsed.
        let (schema, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing succeeds with error recovery.
        assert!(errors.is_empty());
        let schema = schema.expect("should have output");

        // Then: The schema has one item (the message, even without package).
        assert_eq!(schema.items.len(), 1);
        assert!(matches!(schema.items[0], ast::SchemaItem::Message(_)));
    }

    #[test]
    fn test_schema_import_before_package_fails() {
        // Given: An import before the package declaration.
        let input = "include \"foo.baproto\";\npackage bar;";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_schema_message_before_package_fails() {
        // Given: A message before the package declaration.
        let input = "message Foo {}\npackage bar;";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Schema>, _) =
            parse_single(input, schema(parse::MAX_RECURSION_DEPTH));

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }
}
