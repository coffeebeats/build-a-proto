use super::TypeKind;

use crate::ast;

use super::{Lower, LowerContext, TypeResolver};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

impl<'a, R: TypeResolver<TypeKind>> Lower<'a, String, LowerContext<'a, R>> for ast::CommentBlock {
    fn lower(&'a self, _ctx: &'a LowerContext<'a, R>) -> Option<String> {
        let doc = self
            .comments
            .iter()
            .map(|c| c.content.trim())
            .collect::<Vec<_>>()
            .join("\n");

        if doc.is_empty() { None } else { Some(doc) }
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use crate::ast;
    use crate::ir::lower;
    use crate::lex::Span;

    use super::*;

    /* ------------------------ Tests: comment_block ------------------------ */

    #[test]
    fn test_comment_single_line() {
        // Given: A comment block with a single line.
        let comment = ast::CommentBlock {
            comments: vec![ast::Comment {
                content: "  A single comment line  ".to_string(),
                span: Span::default(),
            }],
            span: Span::default(),
        };

        // When: Lowering the comment.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = comment.lower(&ctx);

        // Then: Should produce trimmed single line.
        assert_eq!(result, Some("A single comment line".to_string()));
    }

    #[test]
    fn test_comment_multiple_lines() {
        // Given: A comment block with multiple lines.
        let comment = ast::CommentBlock {
            comments: vec![
                ast::Comment {
                    content: "  First line  ".to_string(),
                    span: Span::default(),
                },
                ast::Comment {
                    content: "  Second line  ".to_string(),
                    span: Span::default(),
                },
                ast::Comment {
                    content: "  Third line  ".to_string(),
                    span: Span::default(),
                },
            ],
            span: Span::default(),
        };

        // When: Lowering the comment.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = comment.lower(&ctx);

        // Then: Should produce trimmed lines joined by newlines.
        assert_eq!(
            result,
            Some("First line\nSecond line\nThird line".to_string())
        );
    }

    #[test]
    fn test_comment_empty_content() {
        // Given: A comment block with empty content.
        let comment = ast::CommentBlock {
            comments: vec![ast::Comment {
                content: "   ".to_string(),
                span: Span::default(),
            }],
            span: Span::default(),
        };

        // When: Lowering the comment.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = comment.lower(&ctx);

        // Then: Should produce None for empty content.
        assert_eq!(result, None);
    }

    #[test]
    fn test_comment_multiple_empty_lines() {
        // Given: A comment block with multiple whitespace-only lines.
        let comment = ast::CommentBlock {
            comments: vec![
                ast::Comment {
                    content: "   ".to_string(),
                    span: Span::default(),
                },
                ast::Comment {
                    content: "  ".to_string(),
                    span: Span::default(),
                },
            ],
            span: Span::default(),
        };

        // When: Lowering the comment.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = comment.lower(&ctx);

        // Then: Trimmed empty lines result in single newline.
        assert_eq!(result, Some("\n".to_string()));
    }

    #[test]
    fn test_comment_mixed_empty_and_content() {
        // Given: A comment block with mix of empty and non-empty lines.
        let comment = ast::CommentBlock {
            comments: vec![
                ast::Comment {
                    content: "  Content line  ".to_string(),
                    span: Span::default(),
                },
                ast::Comment {
                    content: "   ".to_string(),
                    span: Span::default(),
                },
                ast::Comment {
                    content: "  Another line  ".to_string(),
                    span: Span::default(),
                },
            ],
            span: Span::default(),
        };

        // When: Lowering the comment.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = comment.lower(&ctx);

        // Then: Empty lines should become empty string lines in output.
        assert_eq!(result, Some("Content line\n\nAnother line".to_string()));
    }
}
