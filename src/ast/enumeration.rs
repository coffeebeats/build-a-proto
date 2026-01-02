use crate::ast;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                                 Struct: Enum                               */
/* -------------------------------------------------------------------------- */

/// `Enum` represents an enum definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Enum {
    pub comment: Option<ast::CommentBlock>,
    pub items: Vec<EnumItem>,
    pub name: ast::Ident,
    pub span: Span,
}

/* ----------------------------- Enum: EnumItem ----------------------------- */

/// `EnumItem` represents a top-level item within an [`Enum`].
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum EnumItem {
    CommentBlock(ast::CommentBlock),
    FieldVariant(ast::Field),
    UnitVariant(UnitVariant),
}

/* -------------------------------------------------------------------------- */
/*                             Struct: UnitVariant                            */
/* -------------------------------------------------------------------------- */

/// `UnitVariant` represents a simple [`Enum`] variant which is just an
/// identifier.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitVariant {
    pub comment: Option<ast::CommentBlock>,
    pub index: Option<ast::FieldIndex>,
    pub name: ast::Ident,
    pub span: Span,
}
