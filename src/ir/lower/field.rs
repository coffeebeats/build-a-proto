use crate::ast;
use crate::ir::Field;

use super::{Lower, LowerContext};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* --------------------------- Struct: ast::Field --------------------------- */

impl<'a> Lower<'a, Field> for ast::Field {
    fn lower(&'a self, ctx: &'a LowerContext) -> Option<Field> {
        // Fields must have an index
        let index = self.index.as_ref()?.value.value as u32;

        // Lower the field type with optional encoding annotation
        let encoding = self.kind.lower(&FieldTypeContext {
            ctx,
            encoding: self.encoding.as_ref(),
        })?;

        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx));

        Some(Field {
            name: self.name.name.clone(),
            index,
            encoding,
            doc,
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                        Struct: FieldTypeContext                            */
/* -------------------------------------------------------------------------- */

/// Context for lowering field types, including optional encoding annotations.
pub struct FieldTypeContext<'a, 'b> {
    pub ctx: &'a LowerContext<'b>,
    pub encoding: Option<&'a ast::Encoding>,
}