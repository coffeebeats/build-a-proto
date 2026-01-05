use crate::ast;

use super::Visitor;

/* -------------------------------------------------------------------------- */
/*                              Trait: Visitable                              */
/* -------------------------------------------------------------------------- */

/// Implemented by AST nodes - dispatches to appropriate Visitor method.
pub trait Visitable<'ast, V: Visitor<'ast>> {
    fn visit(&'ast self, visitor: &mut V);
}

/* ------------------------- Blanket Impls: Containers ---------------------- */

impl<'ast, V: Visitor<'ast>, T: Visitable<'ast, V>> Visitable<'ast, V> for Box<T> {
    fn visit(&'ast self, visitor: &mut V) {
        (**self).visit(visitor)
    }
}

impl<'ast, V: Visitor<'ast>, T: Visitable<'ast, V>> Visitable<'ast, V> for Option<T> {
    fn visit(&'ast self, visitor: &mut V) {
        if let Some(inner) = self {
            inner.visit(visitor);
        }
    }
}

impl<'ast, V: Visitor<'ast>, T: Visitable<'ast, V>> Visitable<'ast, V> for Vec<T> {
    fn visit(&'ast self, visitor: &mut V) {
        for item in self {
            item.visit(visitor);
        }
    }
}

/* -------------------- Macro: impl_visitable (Structs) --------------------- */

macro_rules! impl_visitable {
    ($($ty:ident => $method:ident),* $(,)?) => {
        $(
            impl<'ast, V: Visitor<'ast>> Visitable<'ast, V> for ast::$ty {
                fn visit(&'ast self, visitor: &mut V) {
                    visitor.$method(self)
                }
            }
        )*
    };
}

impl_visitable! {
    Schema => visit_schema,
    Message => visit_message,
    Enum => visit_enum,
    Field => visit_field,
    UnitVariant => visit_unit_variant,
    Type => visit_type,
    Array => visit_array,
    Map => visit_map,
    Reference => visit_reference,
    Scalar => visit_scalar,
    Package => visit_package,
    Include => visit_include,
    Encoding => visit_encoding,
    CommentBlock => visit_comment_block,
    Comment => visit_comment,
    Ident => visit_ident,
    FieldIndex => visit_field_index,
}

/* --------------------- Macro: impl_visitable_enum (Enums) ----------------- */

macro_rules! impl_visitable_enum {
    ($enum:ident { $($variant:ident),* $(,)? }) => {
        impl<'ast, V: Visitor<'ast>> Visitable<'ast, V> for ast::$enum {
            fn visit(&'ast self, visitor: &mut V) {
                match self {
                    $(Self::$variant(inner) => inner.visit(visitor),)*
                }
            }
        }
    };
}

impl_visitable_enum!(SchemaItem {
    CommentBlock,
    Enum,
    Include,
    Message,
    Package
});
impl_visitable_enum!(MessageItem {
    CommentBlock,
    Enum,
    Field,
    Message
});
impl_visitable_enum!(EnumItem {
    CommentBlock,
    FieldVariant,
    UnitVariant
});
