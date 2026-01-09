use derive_more::Display;

use crate::ast;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                               Struct: Message                              */
/* -------------------------------------------------------------------------- */

/// `Message` represents a message definition.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("message {}", name)]
pub struct Message {
    pub comment: Option<ast::CommentBlock>,
    pub items: Vec<MessageItem>,
    pub name: ast::Ident,
    pub span: Span,
}

/* ---------------------------- Enum: MessageItem --------------------------- */

/// `MessageItem` represents a top-level item within a [`Message`].
#[allow(unused)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageItem {
    CommentBlock(ast::CommentBlock),
    Enum(ast::Enum),
    Field(Field),
    Message(Message),
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Field                               */
/* -------------------------------------------------------------------------- */

/// `Field` represents a field within a [`ast::Message`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub comment: Option<ast::CommentBlock>,
    pub encoding: Option<ast::Encoding>,
    pub index: Option<FieldIndex>,
    pub kind: ast::Type,
    pub name: ast::Ident,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                             Struct: FieldIndex                             */
/* -------------------------------------------------------------------------- */

/// `FieldIndex` represents a field or variant index within a [`ast::Message`]
/// or [`ast::Enum`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldIndex {
    pub span: Span,
    pub value: ast::Uint,
}
