use std::collections::HashSet;

use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::ast;
use crate::core::PackageName;
use crate::core::Reference;
use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Spanned;
use crate::lex::Token;
use crate::lex::spanned;

/* ------------------------------- Fn: parser ------------------------------- */

/// [parser] creates a parser which parses an input [`Token`] slice into an
/// optional [`ast::Schema`]. Returns `None` for empty/missing input.
fn parser<'src, I>()
-> impl Parser<'src, I, Option<ast::Schema>, extra::Err<super::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    todo!()
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input_returns_empty_source_file() {
        // Given: An input list of tokens.
        let input = vec![];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error.
        assert!(output.has_errors());
        assert_eq!(
            output.errors().collect::<Vec<_>>(),
            vec![&Rich::custom(Span::from(0..0), "missing input")]
        );

        // Then: The output is None (no valid AST).
        let result = output.output().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_example_header_returns_correct_source_file() {
        // Given: An input list of tokens.
        let input = vec![
            Token::Newline,
            Token::Keyword(Keyword::Package),
            Token::Ident("abc"),
            Token::Dot,
            Token::Ident("def"),
            Token::Semicolon,
            Token::Newline,
            Token::Newline, // Two line breaks!
            Token::Keyword(Keyword::Include),
            Token::String("a/b/c.baproto"),
            Token::Semicolon,
            // No line break!
            Token::Keyword(Keyword::Include),
            Token::String("d.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output Schema matches expectations.
        let source_file = output.output().unwrap().as_ref().unwrap();

        // Check package
        assert_eq!(source_file.package.name.to_string(), "abc.def");

        // Check includes
        assert_eq!(source_file.includes.len(), 2);
        assert_eq!(source_file.includes[0].path, PathBuf::from("a/b/c.baproto"));
        assert_eq!(source_file.includes[1].path, PathBuf::from("d.baproto"));

        // No items (no messages or enums)
        assert!(source_file.items.is_empty());
    }

    #[test]
    fn test_line_comment_in_message_returns_correct_source_file() {
        // Given: An input list of tokens.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::Ident("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Comment("comment"),
            Token::Newline,
            Token::Newline,
            Token::Uint(0),
            Token::Colon,
            Token::Ident("u8"),
            Token::Ident("sequence_id"),
            Token::Semicolon,
            Token::Newline,
            Token::BlockClose,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(
            !output.has_errors(),
            "Errors: {:?}",
            output.errors().collect::<Vec<_>>()
        );

        // Then: The output Schema matches expectations.
        let source_file = output.output().unwrap().as_ref().unwrap();

        // Check package
        assert_eq!(source_file.package.name.to_string(), "test");

        // Check items - should have one message
        assert_eq!(source_file.items.len(), 1);
        let ast::Item::Message(msg) = &source_file.items[0] else {
            panic!("Expected Message item");
        };

        // Check message
        assert_eq!(msg.name.name, "Message");
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].name.name, "sequence_id");
        assert_eq!(msg.fields[0].index.value, 0);
        assert!(matches!(
            msg.fields[0].typ.kind,
            ast::TypeKind::Scalar(ast::ScalarType::Uint8)
        ));
    }

    #[test]
    fn test_relative_type_reference_parses_correctly() {
        // Given: An input with a dot-separated relative type reference.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::Ident("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Uint(0),
            Token::Colon,
            Token::Ident("other"),
            Token::Dot,
            Token::Ident("package"),
            Token::Dot,
            Token::Ident("MyType"),
            Token::Ident("field_name"),
            Token::Semicolon,
            Token::Newline,
            Token::BlockClose,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(
            !output.has_errors(),
            "Errors: {:?}",
            output.errors().collect::<Vec<_>>()
        );

        let source_file = output.output().unwrap().as_ref().unwrap();
        let ast::Item::Message(msg) = &source_file.items[0] else {
            panic!("Expected Message");
        };

        // Then: The field has the correct type reference.
        let ast::TypeKind::Reference(r) = &msg.fields[0].typ.kind else {
            panic!("Expected Reference type");
        };
        assert!(!r.is_absolute(), "Expected relative reference");
        assert_eq!(r.name(), "MyType");
        assert_eq!(r.path(), vec!["other", "package"]);
    }

    #[test]
    fn test_absolute_type_reference_parses_correctly() {
        // Given: An input with a dot-prefixed absolute type reference.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::Ident("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Uint(0),
            Token::Colon,
            Token::Dot, // Leading dot for absolute reference
            Token::Ident("other"),
            Token::Dot,
            Token::Ident("package"),
            Token::Dot,
            Token::Ident("MyType"),
            Token::Ident("field_name"),
            Token::Semicolon,
            Token::Newline,
            Token::BlockClose,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(
            !output.has_errors(),
            "Errors: {:?}",
            output.errors().collect::<Vec<_>>()
        );

        let source_file = output.output().unwrap().as_ref().unwrap();
        let ast::Item::Message(msg) = &source_file.items[0] else {
            panic!("Expected Message");
        };

        // Then: The field has the correct absolute type reference.
        let ast::TypeKind::Reference(r) = &msg.fields[0].typ.kind else {
            panic!("Expected Reference type");
        };
        assert!(r.is_absolute(), "Expected absolute reference");
        assert_eq!(r.path(), vec!["other", "package"]);
        assert_eq!(r.name(), "MyType");
    }

    #[test]
    fn test_package_with_doc_comment_parses_correctly() {
        use crate::lex::lex;

        // Given: An input with doc comments before the package declaration.
        let source = r#"
// Example comment explaining a package.
// Version: 1.0.0
package foo.bar;
"#;

        // When: The source is lexed and parsed.
        let (tokens, lex_errors) = lex(source);
        assert!(lex_errors.is_empty());
        let tokens = tokens.unwrap();
        let result = parse(&tokens, source.len());

        // Then: The input has no errors.
        assert!(result.errors.is_empty());

        // Then: The package has the correct doc comment.
        let ast = result.ast.expect("Expected AST");
        assert_eq!(ast.package.name.to_string(), "foo.bar");

        let doc = ast.package.doc.unwrap();
        assert_eq!(doc.lines.len(), 2);
        assert_eq!(doc.lines[0], "Example comment explaining a package.");
        assert_eq!(doc.lines[1], "Version: 1.0.0");
    }

    #[test]
    fn test_package_without_doc_comment_parses_correctly() {
        use crate::lex::lex;

        // Given: An input without doc comments before the package declaration.
        let source = r#"
package foo.bar;
"#;

        // When: The source is lexed and parsed.
        let (tokens, lex_errors) = lex(source);
        assert!(lex_errors.is_empty());
        let tokens = tokens.unwrap();

        let result = parse(&tokens, source.len());

        // Then: The input has no errors.
        assert!(result.errors.is_empty());

        // Then: The package has no doc comment.
        let ast = result.ast.unwrap();
        assert_eq!(ast.package.name.to_string(), "foo.bar");
        assert!(ast.package.doc.is_none(),);
    }
}
