use crate::ast;
use crate::core::PackageName;

/* -------------------------------------------------------------------------- */
/*                               Struct: Schema                               */
/* -------------------------------------------------------------------------- */

/// `Schema` defines a complete, parsed `.baproto` schema file.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    pub items: Vec<SchemaItem>,
    pub span: crate::lex::Span,
}

/* ---------------------------- Enum: SchemaItem ---------------------------- */

/// `SchemaItem` represents a top-level item within a schema file.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum SchemaItem {
    CommentBlock(ast::CommentBlock),
    Enum(ast::Enum),
    Include(ast::Include),
    Message(ast::Message),
    Package(ast::Package),
}

/* ------------------------------ Impl: Schema ------------------------------ */

impl Schema {
    pub fn get_package_name(&self) -> Option<PackageName> {
        self.items.iter().find_map(|item| match item {
            ast::SchemaItem::Package(pkg) => PackageName::try_from(pkg.clone()).ok(),
            _ => None,
        })
    }

    /// `iter_includes` returns an iterator over [`Include`] items.
    #[allow(unused)]
    pub fn iter_includes(&self) -> impl Iterator<Item = &ast::Include> {
        self.items.iter().filter_map(|item| match item {
            SchemaItem::Include(i) => Some(i),
            _ => None,
        })
    }

    /// `iter_messages` returns an iterator over [`ast::Message`] items. If
    /// `deep` is `true`, then all nested [`ast::Message`] items will be
    /// included as well, using a pre-order traversal (root node then children).
    #[allow(unused)]
    pub fn iter_messages(&self, deep: bool) -> impl Iterator<Item = &ast::Message> {
        let messages = self.items.iter().filter_map(|item| match item {
            SchemaItem::Message(m) => Some(m),
            _ => None,
        });

        if deep { todo!() } else { messages }
    }

    /// `iter_enums` returns an iterator over [`ast::Enum`] items. If `deep` is
    /// `true`, then all nested [`ast::Enum`] items will be included as well,
    /// using a pre-order traversal (root node then children).
    #[allow(unused)]
    pub fn iter_enums(&self, deep: bool) -> impl Iterator<Item = &ast::Enum> {
        let enums = self.items.iter().filter_map(|item| match item {
            SchemaItem::Enum(e) => Some(e),
            _ => None,
        });

        if deep { todo!() } else { enums }
    }
}
