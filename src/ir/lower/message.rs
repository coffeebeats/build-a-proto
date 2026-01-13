use crate::ast;
use crate::ir::Message;

use super::{Lower, LowerContext, TypeResolver};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

impl<'a, R: TypeResolver> Lower<'a, Message, LowerContext<'a, R>> for ast::Message {
    fn lower(&'a self, ctx: &'a LowerContext<'a, R>) -> Option<Message> {
        let name = self.name.name.clone();
        let descriptor = ctx.build_child_descriptor(&name);

        let child_ctx = ctx.with_child(name.clone());

        let mut fields = Vec::new();
        let mut messages = Vec::new();
        let mut enums = Vec::new();

        for item in &self.items {
            match item {
                ast::MessageItem::Field(field) => {
                    if let Some(ir_field) = field.lower(&child_ctx) {
                        fields.push(ir_field);
                    }
                }
                ast::MessageItem::Message(msg) => {
                    if let Some(ir_msg) = msg.lower(&child_ctx) {
                        messages.push(ir_msg);
                    }
                }
                ast::MessageItem::Enum(e) => {
                    if let Some(ir_enum) = e.lower(&child_ctx) {
                        enums.push(ir_enum);
                    }
                }
                _ => {}
            }
        }

        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx));

        Some(Message {
            descriptor,
            name,
            fields,
            messages,
            enums,
            doc,
        })
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

    /* --------------------------- Tests: message --------------------------- */

    #[test]
    fn test_message_empty() {
        // Given: A message with no fields or nested types.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Empty".to_string(),
                span: Span::default(),
            },
            items: vec![],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should produce empty message with proper descriptor.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.name, "Empty");
        assert!(ir_message.descriptor.contains("Empty"));
        assert!(ir_message.fields.is_empty());
        assert!(ir_message.messages.is_empty());
        assert!(ir_message.enums.is_empty());
    }

    #[test]
    fn test_message_with_fields() {
        // Given: A message with multiple fields.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Player".to_string(),
                span: Span::default(),
            },
            items: vec![
                ast::MessageItem::Field(ast::Field {
                    comment: None,
                    name: ast::Ident {
                        name: "id".to_string(),
                        span: Span::default(),
                    },
                    index: Some(ast::FieldIndex {
                        value: ast::Uint {
                            value: 0,
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    kind: ast::Type::Scalar(ast::Scalar {
                        kind: ast::ScalarType::Uint32,
                        span: Span::default(),
                    }),
                    encoding: None,
                    span: Span::default(),
                }),
                ast::MessageItem::Field(ast::Field {
                    comment: None,
                    name: ast::Ident {
                        name: "name".to_string(),
                        span: Span::default(),
                    },
                    index: Some(ast::FieldIndex {
                        value: ast::Uint {
                            value: 1,
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    kind: ast::Type::Scalar(ast::Scalar {
                        kind: ast::ScalarType::String,
                        span: Span::default(),
                    }),
                    encoding: None,
                    span: Span::default(),
                }),
            ],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should lower all fields correctly.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.fields.len(), 2);
        assert_eq!(ir_message.fields[0].name, "id");
        assert_eq!(ir_message.fields[1].name, "name");
    }

    #[test]
    fn test_message_with_documentation() {
        // Given: A message with documentation.
        let message = ast::Message {
            comment: Some(ast::CommentBlock {
                comments: vec![ast::Comment {
                    content: "Represents a player in the game".to_string(),
                    span: Span::default(),
                }],
                span: Span::default(),
            }),
            name: ast::Ident {
                name: "Player".to_string(),
                span: Span::default(),
            },
            items: vec![],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should include documentation.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(
            ir_message.doc,
            Some("Represents a player in the game".to_string())
        );
    }

    #[test]
    fn test_message_nested_message() {
        // Given: A message with a nested message.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Outer".to_string(),
                span: Span::default(),
            },
            items: vec![ast::MessageItem::Message(ast::Message {
                comment: None,
                name: ast::Ident {
                    name: "Inner".to_string(),
                    span: Span::default(),
                },
                items: vec![],
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should include nested message.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.messages.len(), 1);
        assert_eq!(ir_message.messages[0].name, "Inner");
        assert!(ir_message.messages[0].descriptor.contains("Inner"));
    }

    #[test]
    fn test_message_nested_enum() {
        // Given: A message with a nested enum.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Config".to_string(),
                span: Span::default(),
            },
            items: vec![ast::MessageItem::Enum(ast::Enum {
                comment: None,
                name: ast::Ident {
                    name: "Mode".to_string(),
                    span: Span::default(),
                },
                items: vec![ast::EnumItem::UnitVariant(ast::UnitVariant {
                    comment: None,
                    name: ast::Ident {
                        name: "Fast".to_string(),
                        span: Span::default(),
                    },
                    index: Some(ast::FieldIndex {
                        value: ast::Uint {
                            value: 0,
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    span: Span::default(),
                })],
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should include nested enum.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.enums.len(), 1);
        assert_eq!(ir_message.enums[0].name, "Mode");
    }

    #[test]
    fn test_message_mixed_items() {
        // Given: A message with fields, nested message, and nested enum.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Complex".to_string(),
                span: Span::default(),
            },
            items: vec![
                ast::MessageItem::Field(ast::Field {
                    comment: None,
                    name: ast::Ident {
                        name: "id".to_string(),
                        span: Span::default(),
                    },
                    index: Some(ast::FieldIndex {
                        value: ast::Uint {
                            value: 0,
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                    kind: ast::Type::Scalar(ast::Scalar {
                        kind: ast::ScalarType::Uint32,
                        span: Span::default(),
                    }),
                    encoding: None,
                    span: Span::default(),
                }),
                ast::MessageItem::Message(ast::Message {
                    comment: None,
                    name: ast::Ident {
                        name: "Nested".to_string(),
                        span: Span::default(),
                    },
                    items: vec![],
                    span: Span::default(),
                }),
                ast::MessageItem::Enum(ast::Enum {
                    comment: None,
                    name: ast::Ident {
                        name: "Status".to_string(),
                        span: Span::default(),
                    },
                    items: vec![],
                    span: Span::default(),
                }),
            ],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should lower all items correctly.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.fields.len(), 1);
        assert_eq!(ir_message.messages.len(), 1);
        assert_eq!(ir_message.enums.len(), 1);
    }

    #[test]
    fn test_message_field_ordering_preserved() {
        // Given: A message with fields in specific order.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Ordered".to_string(),
                span: Span::default(),
            },
            items: vec![
                ast::MessageItem::Field(make_field("first", 0)),
                ast::MessageItem::Field(make_field("second", 1)),
                ast::MessageItem::Field(make_field("third", 2)),
            ],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should preserve field order.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.fields.len(), 3);
        assert_eq!(ir_message.fields[0].name, "first");
        assert_eq!(ir_message.fields[1].name, "second");
        assert_eq!(ir_message.fields[2].name, "third");
    }

    #[test]
    fn test_message_skips_fields_without_index() {
        // Given: A message with a field missing an index.
        let message = ast::Message {
            comment: None,
            name: ast::Ident {
                name: "Partial".to_string(),
                span: Span::default(),
            },
            items: vec![
                ast::MessageItem::Field(make_field("good", 0)),
                ast::MessageItem::Field(ast::Field {
                    comment: None,
                    name: ast::Ident {
                        name: "bad".to_string(),
                        span: Span::default(),
                    },
                    index: None, // Missing index
                    kind: ast::Type::Scalar(ast::Scalar {
                        kind: ast::ScalarType::Uint32,
                        span: Span::default(),
                    }),
                    encoding: None,
                    span: Span::default(),
                }),
            ],
            span: Span::default(),
        };

        // When: Lowering the message.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = message.lower(&ctx);

        // Then: Should skip field without index.
        assert!(result.is_some());
        let ir_message = result.unwrap();
        assert_eq!(ir_message.fields.len(), 1);
        assert_eq!(ir_message.fields[0].name, "good");
    }

    /* --------------------------- Fn: make_field --------------------------- */

    fn make_field(name: &str, index: u64) -> ast::Field {
        ast::Field {
            comment: None,
            name: ast::Ident {
                name: name.to_string(),
                span: Span::default(),
            },
            index: Some(ast::FieldIndex {
                value: ast::Uint {
                    value: index,
                    span: Span::default(),
                },
                span: Span::default(),
            }),
            kind: ast::Type::Scalar(ast::Scalar {
                kind: ast::ScalarType::Uint32,
                span: Span::default(),
            }),
            encoding: None,
            span: Span::default(),
        }
    }
}
