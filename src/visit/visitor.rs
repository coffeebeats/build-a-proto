use crate::ast;

use super::walk;

/* -------------------------------------------------------------------------- */
/*                             Macro: define_visitor                          */
/* -------------------------------------------------------------------------- */

macro_rules! define_visitor {
    (
        // Methods with walk functions (have children)
        $(fn $method:ident($ty:ty) => $walk:ident;)*
        ---
        // Leaf methods (no children to walk)
        $(fn $leaf:ident($leaf_ty:ty);)*
    ) => {
        pub trait Visitor<'ast>: Sized {
            $(
                fn $method(&mut self, node: &'ast $ty) {
                    walk::$walk(self, node)
                }
            )*

            $(
                #[allow(unused_variables)]
                fn $leaf(&mut self, node: &'ast $leaf_ty) {}
            )*
        }
    };
}

/* -------------------------------------------------------------------------- */
/*                               Trait: Visitor                               */
/* -------------------------------------------------------------------------- */

define_visitor! {
    fn visit_schema(ast::Schema) => walk_schema;
    fn visit_message(ast::Message) => walk_message;
    fn visit_enum(ast::Enum) => walk_enum;
    fn visit_field(ast::Field) => walk_field;
    fn visit_unit_variant(ast::UnitVariant) => walk_unit_variant;
    fn visit_type(ast::Type) => walk_type;
    fn visit_array(ast::Array) => walk_array;
    fn visit_map(ast::Map) => walk_map;
    fn visit_package(ast::Package) => walk_package;
    fn visit_comment_block(ast::CommentBlock) => walk_comment_block;
    ---
    fn visit_include(ast::Include);
    fn visit_reference(ast::Reference);
    fn visit_scalar(ast::Scalar);
    fn visit_encoding(ast::Encoding);
    fn visit_ident(ast::Ident);
    fn visit_comment(ast::Comment);
    fn visit_field_index(ast::FieldIndex);
}
