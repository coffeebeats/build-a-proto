use crate::ast;

use super::{Lower, LowerContext};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

impl<'a> Lower<'a, String> for ast::CommentBlock {
    fn lower(&'a self, _ctx: &'a LowerContext) -> Option<String> {
        let doc = self
            .comments
            .iter()
            .map(|c| c.content.trim())
            .collect::<Vec<_>>()
            .join("\n");

        if doc.is_empty() {
            None
        } else {
            Some(doc)
        }
    }
}