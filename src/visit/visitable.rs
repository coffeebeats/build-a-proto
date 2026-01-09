use crate::ast;

use super::Visitor;

/* -------------------------------------------------------------------------- */
/*                              Trait: Visitable                              */
/* -------------------------------------------------------------------------- */

/// `Visitable` defines an AST node that can be "visited" to traverse the node's
/// contents. Each type's implementation dispatches to the appropriate "walk"
/// functions.
#[allow(unused)]
pub trait Visitable<'ast, V: Visitor<'ast>> {
    fn visit(&'ast self, visitor: &mut V);
}

/* ----------------------------- Struct: Box<T> ----------------------------- */

impl<'ast, V: Visitor<'ast>, T: Visitable<'ast, V>> Visitable<'ast, V> for Box<T> {
    #[inline]
    fn visit(&'ast self, visitor: &mut V) {
        self.as_ref().visit(visitor)
    }
}

/* ----------------------------- Enum: Option<T> ---------------------------- */

impl<'ast, V: Visitor<'ast>, T: Visitable<'ast, V>> Visitable<'ast, V> for Option<T> {
    #[inline]
    fn visit(&'ast self, visitor: &mut V) {
        if let Some(inner) = self {
            inner.visit(visitor);
        }
    }
}

/* ----------------------------- Struct: Vec<T> ----------------------------- */

impl<'ast, V: Visitor<'ast>, T: Visitable<'ast, V>> Visitable<'ast, V> for Vec<T> {
    #[inline]
    fn visit(&'ast self, visitor: &mut V) {
        for item in self {
            item.visit(visitor);
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                        Macro: impl_visitable_struct                        */
/* -------------------------------------------------------------------------- */

/// Implements [`Visitable`] for struct types by delegating to the corresponding
/// visitor method.
macro_rules! impl_visitable_struct {
    ($($ty:ident => $method:ident),* $(,)?) => {
        $(
            impl<'ast, V: Visitor<'ast>> Visitable<'ast, V> for ast::$ty {
                #[inline]
                fn visit(&'ast self, visitor: &mut V) {
                    visitor.$method(self)
                }
            }
        )*
    };
}

/* ----------------------- Impl: impl_visitable_struct ---------------------- */

impl_visitable_struct! {
    Array => visit_array,
    Comment => visit_comment,
    CommentBlock => visit_comment_block,
    Encoding => visit_encoding,
    Enum => visit_enum,
    Field => visit_field,
    FieldIndex => visit_field_index,
    Ident => visit_ident,
    Include => visit_include,
    Map => visit_map,
    Message => visit_message,
    Package => visit_package,
    Reference => visit_reference,
    Scalar => visit_scalar,
    Schema => visit_schema,
    Text => visit_text,
    Type => visit_type,
    Uint => visit_uint,
    UnitVariant => visit_unit_variant,
}

/* -------------------------------------------------------------------------- */
/*                         Macro: impl_visitable_enum                         */
/* -------------------------------------------------------------------------- */

/// Implements [`Visitable`] for enum types by matching on variants and
/// delegating to the inner type's [`Visitable`] implementation.
macro_rules! impl_visitable_enum {
    ($enum:ident { $($variant:ident),* $(,)? }) => {
        impl<'ast, V: Visitor<'ast>> Visitable<'ast, V> for ast::$enum {
            #[inline]
            fn visit(&'ast self, visitor: &mut V) {
                match self {
                    $(Self::$variant(inner) => inner.visit(visitor),)*
                }
            }
        }
    };
}

/* ------------------------ Impl: impl_visitable_enum ----------------------- */

impl_visitable_enum!(EnumItem {
    CommentBlock,
    FieldVariant,
    UnitVariant
});

impl_visitable_enum!(MessageItem {
    CommentBlock,
    Enum,
    Field,
    Message
});

impl_visitable_enum!(SchemaItem {
    CommentBlock,
    Enum,
    Include,
    Message,
    Package
});
