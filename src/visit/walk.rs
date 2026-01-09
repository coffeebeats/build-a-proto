use crate::ast;

use super::Visitable;
use super::Visitor;

/* -------------------------------------------------------------------------- */
/*                                  Fn: Walk                                  */
/* -------------------------------------------------------------------------- */

/* ----------------------------- Struct: Schema ----------------------------- */

/// Walks a [`ast::Schema`], visiting all items.
#[inline]
pub fn walk_schema<'ast, V: Visitor<'ast>>(visitor: &mut V, schema: &'ast ast::Schema) {
    schema.items.visit(visitor);
}

/* ----------------------------- Struct: Message ---------------------------- */

/// Walks a [`ast::Message`], visiting comment, name, and all items.
#[inline]
pub fn walk_message<'ast, V: Visitor<'ast>>(visitor: &mut V, message: &'ast ast::Message) {
    message.comment.visit(visitor);
    message.name.visit(visitor);
    message.items.visit(visitor);
}

/* ------------------------------ Struct: Enum ------------------------------ */

/// Walks a [`ast::Enum`], visiting comment, name, and all items.
#[inline]
pub fn walk_enum<'ast, V: Visitor<'ast>>(visitor: &mut V, enum_: &'ast ast::Enum) {
    enum_.comment.visit(visitor);
    enum_.name.visit(visitor);
    enum_.items.visit(visitor);
}

/* ------------------------------ Struct: Field ----------------------------- */

/// Walks a [`ast::Field`], visiting comment, name, type, encoding, and index.
#[inline]
pub fn walk_field<'ast, V: Visitor<'ast>>(visitor: &mut V, field: &'ast ast::Field) {
    field.comment.visit(visitor);
    field.index.visit(visitor);
    field.kind.visit(visitor);
    field.name.visit(visitor);
    field.encoding.visit(visitor);
}

/* --------------------------- Struct: UnitVariant -------------------------- */

/// Walks a [`ast::UnitVariant`], visiting comment, name, and index.
#[inline]
pub fn walk_unit_variant<'ast, V: Visitor<'ast>>(visitor: &mut V, variant: &'ast ast::UnitVariant) {
    variant.comment.visit(visitor);
    variant.index.visit(visitor);
    variant.name.visit(visitor);
}

/* ------------------------------- Enum: Type ------------------------------- */

/// Walks a [`ast::Type`], dispatching to the appropriate variant.
#[inline]
pub fn walk_type<'ast, V: Visitor<'ast>>(visitor: &mut V, kind: &'ast ast::Type) {
    match kind {
        ast::Type::Array(arr) => arr.visit(visitor),
        ast::Type::Map(map) => map.visit(visitor),
        ast::Type::Reference(ref_) => ref_.visit(visitor),
        ast::Type::Scalar(scalar) => scalar.visit(visitor),
    }
}

/* ------------------------------ Struct: Array ----------------------------- */

/// Walks a [`ast::Array`], visiting the element type.
#[inline]
pub fn walk_array<'ast, V: Visitor<'ast>>(visitor: &mut V, array: &'ast ast::Array) {
    array.size.visit(visitor);
    array.element.visit(visitor);
}

/* ------------------------------- Struct: Map ------------------------------ */

/// Walks a [`ast::Map`], visiting key and value types.
#[inline]
pub fn walk_map<'ast, V: Visitor<'ast>>(visitor: &mut V, map: &'ast ast::Map) {
    map.key.visit(visitor);
    map.value.visit(visitor);
}

/* ----------------------------- Struct: Package ---------------------------- */

/// Walks a [`ast::Package`], visiting comment and all component identifiers.
#[inline]
pub fn walk_package<'ast, V: Visitor<'ast>>(visitor: &mut V, package: &'ast ast::Package) {
    package.comment.visit(visitor);
    package.components.visit(visitor);
}

/* -------------------------- Struct: CommentBlock -------------------------- */

/// Walks a [`ast::CommentBlock`], visiting all comments.
#[inline]
pub fn walk_comment_block<'ast, V: Visitor<'ast>>(visitor: &mut V, block: &'ast ast::CommentBlock) {
    block.comments.visit(visitor);
}
