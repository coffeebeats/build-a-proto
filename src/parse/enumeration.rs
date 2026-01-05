use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                               Fn: Enumeration                              */
/* -------------------------------------------------------------------------- */

/// `enumeration` creates a new [`Parser`] that parses an enum definition into
/// an [`ast::Enum`].
pub(super) fn enumeration<'src, I>()
-> impl Parser<'src, I, ast::Enum, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(just(Token::Keyword(Keyword::Enum)).ignore_then(parse::ident()))
        .then(
            choice((
                parse::field().map(ast::EnumItem::FieldVariant),
                unit_variant().map(ast::EnumItem::UnitVariant),
                parse::comment_block().map(ast::EnumItem::CommentBlock),
            ))
            // FIXME: Handle newlines before, between, and after.
            .separated_by(just(Token::Newline).repeated())
            .allow_leading()
            .allow_trailing()
            .collect::<Vec<ast::EnumItem>>()
            .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
        )
        .then_ignore(just(Token::Newline).repeated())
        .map_with(|((comment, name), items), e| ast::Enum {
            comment,
            items,
            name,
            span: e.span(),
        })
        .labelled("enum")
        .boxed()
}

/* ---------------------------- Fn: unit_variant ---------------------------- */

/// `unit_variant` creates a new [`Parser`] that parses a unit enum variant into
/// an [`ast::UnitVariant`].
fn unit_variant<'src, I>()
-> impl Parser<'src, I, ast::UnitVariant, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(parse::field_index().or_not())
        .then(parse::ident())
        .then_ignore(just(Token::Semicolon))
        .map_with(|((comment, index), name), e| ast::UnitVariant {
            comment,
            index,
            name,
            span: e.span(),
        })
        .labelled("unit variant")
        .boxed()
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::test_parse;

    /* -------------------- Enumeration Parser Tests -------------------- */

    #[test]
    fn test_enumeration_empty_body_succeeds() {
        // Given: An enum with an empty body.
        let input = "enum Foo {}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: The enum has the correct name and no items.
        assert_eq!(enumeration.name.name, "Foo");
        assert!(enumeration.items.is_empty());
        assert!(enumeration.comment.is_none());
    }

    #[test]
    fn test_enumeration_single_unit_variant_succeeds() {
        // Given: An enum with a single unit variant.
        let input = "enum Status {\nOK;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: The enum has one unit variant.
        assert_eq!(enumeration.name.name, "Status");
        assert_eq!(enumeration.items.len(), 1);

        let ast::EnumItem::UnitVariant(variant) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(variant.name.name, "OK");
        assert!(variant.index.is_none());
        assert!(variant.comment.is_none());
    }

    #[test]
    fn test_enumeration_multiple_unit_variants_succeeds() {
        // Given: An enum with multiple unit variants.
        let input = "enum Status {\nOK;\nERROR;\nPENDING;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: All variants are present.
        assert_eq!(enumeration.items.len(), 3);

        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.name.name, "OK");

        let ast::EnumItem::UnitVariant(v2) = &enumeration.items[1] else {
            panic!("expected unit variant");
        };
        assert_eq!(v2.name.name, "ERROR");

        let ast::EnumItem::UnitVariant(v3) = &enumeration.items[2] else {
            panic!("expected unit variant");
        };
        assert_eq!(v3.name.name, "PENDING");
    }

    #[test]
    fn test_enumeration_variant_with_explicit_index_succeeds() {
        // Given: An enum with variants having explicit indices.
        let input = "enum Code {\n0: SUCCESS;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: The variant has the correct index.
        assert_eq!(enumeration.items.len(), 1);

        let ast::EnumItem::UnitVariant(variant) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(variant.name.name, "SUCCESS");
        assert!(variant.index.is_some());
        assert_eq!(variant.index.as_ref().unwrap().value.value, 0);
    }

    #[test]
    fn test_enumeration_variants_with_multiple_explicit_indices_succeeds() {
        // Given: An enum where all variants have explicit indices.
        let input = "enum Code {\n0: SUCCESS;\n1: FAILURE;\n42: UNKNOWN;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: All variants have their indices.
        assert_eq!(enumeration.items.len(), 3);

        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.name.name, "SUCCESS");
        assert_eq!(v1.index.as_ref().unwrap().value.value, 0);

        let ast::EnumItem::UnitVariant(v2) = &enumeration.items[1] else {
            panic!("expected unit variant");
        };
        assert_eq!(v2.name.name, "FAILURE");
        assert_eq!(v2.index.as_ref().unwrap().value.value, 1);

        let ast::EnumItem::UnitVariant(v3) = &enumeration.items[2] else {
            panic!("expected unit variant");
        };
        assert_eq!(v3.name.name, "UNKNOWN");
        assert_eq!(v3.index.as_ref().unwrap().value.value, 42);
    }

    #[test]
    fn test_enumeration_mixed_indexed_and_unindexed_variants_succeeds() {
        // Given: An enum with both indexed and unindexed variants.
        let input = "enum Mixed {\n0: FIRST;\nSECOND;\n3: THIRD;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: Variants have correct index presence.
        assert_eq!(enumeration.items.len(), 3);

        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.name.name, "FIRST");
        assert!(v1.index.is_some());
        assert_eq!(v1.index.as_ref().unwrap().value.value, 0);

        let ast::EnumItem::UnitVariant(v2) = &enumeration.items[1] else {
            panic!("expected unit variant");
        };
        assert_eq!(v2.name.name, "SECOND");
        assert!(v2.index.is_none());

        let ast::EnumItem::UnitVariant(v3) = &enumeration.items[2] else {
            panic!("expected unit variant");
        };
        assert_eq!(v3.name.name, "THIRD");
        assert!(v3.index.is_some());
        assert_eq!(v3.index.as_ref().unwrap().value.value, 3);
    }

    #[test]
    fn test_enumeration_with_doc_comment_succeeds() {
        // Given: An enum with a preceding doc comment.
        let input = "// Status enum\nenum Status {\nOK;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: The doc comment is captured.
        assert!(enumeration.comment.is_some());
        let comment = enumeration.comment.as_ref().unwrap();
        assert_eq!(comment.comments.len(), 1);
        assert_eq!(comment.comments[0].content, "Status enum");
    }

    #[test]
    fn test_enumeration_with_multi_line_doc_comment_succeeds() {
        // Given: An enum with a multi-line doc comment.
        let input = "// Status enumeration\n// Represents operation status\nenum Status {\nOK;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: All comment lines are captured.
        assert!(enumeration.comment.is_some());
        let comment = enumeration.comment.as_ref().unwrap();
        assert_eq!(comment.comments.len(), 2);
        assert_eq!(comment.comments[0].content, "Status enumeration");
        assert_eq!(comment.comments[1].content, "Represents operation status");
    }

    #[test]
    fn test_enumeration_variant_with_doc_comment_succeeds() {
        // Given: An enum with a variant that has a doc comment.
        let input = "enum Status {\n// Success status\nOK;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: The variant's doc comment is captured.
        assert_eq!(enumeration.items.len(), 1);

        let ast::EnumItem::UnitVariant(variant) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert!(variant.comment.is_some());
        let comment = variant.comment.as_ref().unwrap();
        assert_eq!(comment.comments.len(), 1);
        assert_eq!(comment.comments[0].content, "Success status");
    }

    #[test]
    fn test_enumeration_multiple_variants_with_doc_comments_succeeds() {
        // Given: An enum where each variant has a doc comment.
        let input = "enum Status {\n// Success\nOK;\n// Failure\nERROR;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: Each variant has its doc comment.
        assert_eq!(enumeration.items.len(), 2);

        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.comment.as_ref().unwrap().comments[0].content, "Success");

        let ast::EnumItem::UnitVariant(v2) = &enumeration.items[1] else {
            panic!("expected unit variant");
        };
        assert_eq!(v2.comment.as_ref().unwrap().comments[0].content, "Failure");
    }

    #[test]
    fn test_enumeration_with_comment_block_item_succeeds() {
        // Given: An enum with a comment between variants.
        let input = "enum Status {\nOK;\n// Internal separator\nERROR;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: The comment is attached to the following variant.
        assert_eq!(enumeration.items.len(), 2);

        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.name.name, "OK");
        assert!(v1.comment.is_none());

        let ast::EnumItem::UnitVariant(v2) = &enumeration.items[1] else {
            panic!("expected unit variant");
        };
        assert_eq!(v2.name.name, "ERROR");
        assert!(v2.comment.is_some());
        assert_eq!(v2.comment.as_ref().unwrap().comments[0].content, "Internal separator");
    }

    #[test]
    fn test_enumeration_with_field_variant_succeeds() {
        // Given: An enum with a field variant (variant with data).
        let input = "enum Result {\nOK;\nu32 error_code;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: Both variants are present.
        assert_eq!(enumeration.items.len(), 2);

        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.name.name, "OK");

        let ast::EnumItem::FieldVariant(field) = &enumeration.items[1] else {
            panic!("expected field variant");
        };
        assert_eq!(field.name.name, "error_code");
    }

    #[test]
    fn test_enumeration_mixed_item_types_succeeds() {
        // Given: An enum with mixed item types.
        let input = "enum Complex {\n// Header comment\nOK;\nu32 code;\n// Footer comment\nERROR;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: All items are captured with comments attached to following items.
        assert_eq!(enumeration.items.len(), 3);

        // First variant with header comment
        let ast::EnumItem::UnitVariant(v1) = &enumeration.items[0] else {
            panic!("expected unit variant");
        };
        assert_eq!(v1.name.name, "OK");
        assert!(v1.comment.is_some());
        assert_eq!(v1.comment.as_ref().unwrap().comments[0].content, "Header comment");

        // Field variant without comment
        let ast::EnumItem::FieldVariant(field) = &enumeration.items[1] else {
            panic!("expected field variant");
        };
        assert_eq!(field.name.name, "code");

        // Last variant with footer comment
        let ast::EnumItem::UnitVariant(v2) = &enumeration.items[2] else {
            panic!("expected unit variant");
        };
        assert_eq!(v2.name.name, "ERROR");
        assert!(v2.comment.is_some());
        assert_eq!(v2.comment.as_ref().unwrap().comments[0].content, "Footer comment");
    }

    #[test]
    fn test_enumeration_with_trailing_newlines_succeeds() {
        // Given: An enum with trailing newlines in the body.
        let input = "enum Status {\nOK;\n\n\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: Trailing newlines don't create extra items.
        assert_eq!(enumeration.items.len(), 1);
    }

    #[test]
    fn test_enumeration_with_leading_newlines_succeeds() {
        // Given: An enum with leading newlines in the body.
        let input = "enum Status {\n\n\nOK;\n}";

        // When: The input is parsed.
        let (enumeration, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing succeeds.
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        let enumeration = enumeration.expect("should have output");

        // Then: Leading newlines don't create extra items.
        assert_eq!(enumeration.items.len(), 1);
    }

    /* ----------------------- Error Cases -------------------------- */

    #[test]
    fn test_enumeration_missing_name_fails() {
        // Given: An enum declaration missing the name.
        let input = "enum {}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_enumeration_variant_missing_semicolon_fails() {
        // Given: A variant without a semicolon.
        let input = "enum Status {\nOK\n}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_enumeration_missing_closing_brace_fails() {
        // Given: An enum missing the closing brace.
        let input = "enum Status {\nOK;";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }

    #[test]
    fn test_enumeration_invalid_index_syntax_fails() {
        // Given: A variant with invalid index syntax (missing colon).
        let input = "enum Status {\n0 OK;\n}";

        // When: The input is parsed.
        let (_result, errors): (Option<ast::Enum>, _) = test_parse!(input, enumeration());

        // Then: Parsing fails.
        assert!(!errors.is_empty(), "expected parsing to fail");
    }
}
