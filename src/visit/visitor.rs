use crate::ast;

use super::walk;

/* -------------------------------------------------------------------------- */
/*                             Macro: define_visitor                          */
/* -------------------------------------------------------------------------- */

/// Defines the [`Visitor`] trait with methods for each AST node type. Methods
/// with a walk function delegate to it for traversal; methods without provide
/// an empty default implementation.
macro_rules! define_visitor {
    (
        $(fn $method:ident($ty:ty) $(=> $walk:ident)?;)*
    ) => {
        pub trait Visitor<'ast>: Sized {
            $(
                define_visitor!(@method $method, $ty $(, $walk)?);
            )*
        }
    };

    // Recursive method (with walk function)
    (@method $method:ident, $ty:ty, $walk:ident) => {
        #[allow(unused)]
        #[inline]
        fn $method(&mut self, node: &'ast $ty) {
            walk::$walk(self, node)
        }
    };

    // Leaf method (without walk function)
    (@method $method:ident, $ty:ty) => {
        #[allow(unused)]
        #[inline]
        fn $method(&mut self, _node: &'ast $ty) {}
    };
}

/* -------------------------------------------------------------------------- */
/*                               Trait: Visitor                               */
/* -------------------------------------------------------------------------- */

define_visitor! {
    fn visit_array(ast::Array) => walk_array;
    fn visit_comment_block(ast::CommentBlock) => walk_comment_block;
    fn visit_comment(ast::Comment);
    fn visit_encoding(ast::Encoding);
    fn visit_enum(ast::Enum) => walk_enum;
    fn visit_field_index(ast::FieldIndex);
    fn visit_field(ast::Field) => walk_field;
    fn visit_ident(ast::Ident);
    fn visit_include(ast::Include);
    fn visit_map(ast::Map) => walk_map;
    fn visit_message(ast::Message) => walk_message;
    fn visit_package(ast::Package) => walk_package;
    fn visit_reference(ast::Reference);
    fn visit_scalar(ast::Scalar);
    fn visit_schema(ast::Schema) => walk_schema;
    fn visit_text(ast::Text);
    fn visit_type(ast::Type) => walk_type;
    fn visit_uint(ast::Uint);
    fn visit_unit_variant(ast::UnitVariant) => walk_unit_variant;
}
