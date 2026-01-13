use crate::ast;
use crate::core::Descriptor;
use crate::ir::Package;

use super::{Lower, LowerContext, TypeResolver};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* --------------------------- Struct: ast::Schema -------------------------- */

impl<'a, R: TypeResolver> Lower<'a, Package, LowerContext<'a, R>> for ast::Schema {
    fn lower(&'a self, ctx: &'a LowerContext<'a, R>) -> Option<Package> {
        // Extract package name from schema
        let package = self.get_package_name()?;
        let package_path = package.to_string();

        // Create root context for this package (reuse resolver from parent context)
        let root = Descriptor {
            package,
            path: vec![],
            name: None,
        };
        let pkg_ctx = LowerContext::new(ctx.resolver, root);

        // Lower top-level messages and enums
        let mut messages = Vec::new();
        let mut enums = Vec::new();

        for item in &self.items {
            match item {
                ast::SchemaItem::Message(msg) => {
                    if let Some(ir_msg) = msg.lower(&pkg_ctx) {
                        messages.push(ir_msg);
                    }
                }
                ast::SchemaItem::Enum(e) => {
                    if let Some(ir_enum) = e.lower(&pkg_ctx) {
                        enums.push(ir_enum);
                    }
                }
                _ => {}
            }
        }

        Some(Package {
            path: package_path,
            messages,
            enums,
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

    /* ---------------------------- Tests: schema --------------------------- */

    #[test]
    fn test_schema_empty() {
        // Given: A schema with only a package declaration.
        let schema = ast::Schema {
            items: vec![ast::SchemaItem::Package(ast::Package {
                comment: None,
                components: vec![ast::Ident {
                    name: "mypackage".to_string(),
                    span: Span::default(),
                }],
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should produce package with no types.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.path, "mypackage");
        assert!(package.messages.is_empty());
        assert!(package.enums.is_empty());
    }

    #[test]
    fn test_schema_with_message() {
        // Given: A schema with a top-level message.
        let schema = ast::Schema {
            items: vec![
                ast::SchemaItem::Package(ast::Package {
                    comment: None,
                    components: vec![ast::Ident {
                        name: "test".to_string(),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                }),
                ast::SchemaItem::Message(ast::Message {
                    comment: None,
                    name: ast::Ident {
                        name: "Player".to_string(),
                        span: Span::default(),
                    },
                    items: vec![],
                    span: Span::default(),
                }),
            ],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should include message in package.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.messages.len(), 1);
        assert_eq!(package.messages[0].name, "Player");
    }

    #[test]
    fn test_schema_with_enum() {
        // Given: A schema with a top-level enum.
        let schema = ast::Schema {
            items: vec![
                ast::SchemaItem::Package(ast::Package {
                    comment: None,
                    components: vec![ast::Ident {
                        name: "test".to_string(),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                }),
                ast::SchemaItem::Enum(ast::Enum {
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

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should include enum in package.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.enums.len(), 1);
        assert_eq!(package.enums[0].name, "Status");
    }

    #[test]
    fn test_schema_multiple_messages() {
        // Given: A schema with multiple top-level messages.
        let schema = ast::Schema {
            items: vec![
                ast::SchemaItem::Package(ast::Package {
                    comment: None,
                    components: vec![ast::Ident {
                        name: "game".to_string(),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                }),
                ast::SchemaItem::Message(make_message("Player")),
                ast::SchemaItem::Message(make_message("Enemy")),
                ast::SchemaItem::Message(make_message("Item")),
            ],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should include all messages.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.messages.len(), 3);
        assert_eq!(package.messages[0].name, "Player");
        assert_eq!(package.messages[1].name, "Enemy");
        assert_eq!(package.messages[2].name, "Item");
    }

    #[test]
    fn test_schema_mixed_types() {
        // Given: A schema with both messages and enums.
        let schema = ast::Schema {
            items: vec![
                ast::SchemaItem::Package(ast::Package {
                    comment: None,
                    components: vec![ast::Ident {
                        name: "mixed".to_string(),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                }),
                ast::SchemaItem::Message(make_message("Config")),
                ast::SchemaItem::Enum(make_enum("Mode")),
                ast::SchemaItem::Message(make_message("State")),
                ast::SchemaItem::Enum(make_enum("Status")),
            ],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should include both messages and enums.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.messages.len(), 2);
        assert_eq!(package.enums.len(), 2);
        assert_eq!(package.messages[0].name, "Config");
        assert_eq!(package.enums[0].name, "Mode");
    }

    #[test]
    fn test_schema_multipart_package() {
        // Given: A schema with a multi-component package name.
        let schema = ast::Schema {
            items: vec![ast::SchemaItem::Package(ast::Package {
                comment: None,
                components: vec![
                    ast::Ident {
                        name: "com".to_string(),
                        span: Span::default(),
                    },
                    ast::Ident {
                        name: "example".to_string(),
                        span: Span::default(),
                    },
                    ast::Ident {
                        name: "game".to_string(),
                        span: Span::default(),
                    },
                ],
                span: Span::default(),
            })],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should correctly format package path.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.path, "com.example.game");
    }

    #[test]
    fn test_schema_without_package() {
        // Given: A schema without a package declaration.
        let schema = ast::Schema {
            items: vec![ast::SchemaItem::Message(make_message("Test"))],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should return None (package is required).
        assert!(result.is_none());
    }

    #[test]
    fn test_schema_preserves_order() {
        // Given: A schema with items in specific order.
        let schema = ast::Schema {
            items: vec![
                ast::SchemaItem::Package(ast::Package {
                    comment: None,
                    components: vec![ast::Ident {
                        name: "ordered".to_string(),
                        span: Span::default(),
                    }],
                    span: Span::default(),
                }),
                ast::SchemaItem::Message(make_message("First")),
                ast::SchemaItem::Enum(make_enum("Second")),
                ast::SchemaItem::Message(make_message("Third")),
            ],
            span: Span::default(),
        };

        // When: Lowering the schema.
        let resolver = lower::MockResolver::new();
        let ctx = lower::make_context(&resolver);
        let result = schema.lower(&ctx);

        // Then: Should preserve declaration order.
        assert!(result.is_some());
        let package = result.unwrap();
        assert_eq!(package.messages.len(), 2);
        assert_eq!(package.enums.len(), 1);
        assert_eq!(package.messages[0].name, "First");
        assert_eq!(package.messages[1].name, "Third");
        assert_eq!(package.enums[0].name, "Second");
    }

    /* ---------------------------- Fn: make_enum --------------------------- */

    fn make_enum(name: &str) -> ast::Enum {
        ast::Enum {
            comment: None,
            name: ast::Ident {
                name: name.to_string(),
                span: Span::default(),
            },
            items: vec![],
            span: Span::default(),
        }
    }

    /* -------------------------- Fn: make_message -------------------------- */

    fn make_message(name: &str) -> ast::Message {
        ast::Message {
            comment: None,
            name: ast::Ident {
                name: name.to_string(),
                span: Span::default(),
            },
            items: vec![],
            span: Span::default(),
        }
    }
}
