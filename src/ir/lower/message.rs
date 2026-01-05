use crate::ast;
use crate::ir::Message;

use super::{Lower, LowerContext};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

impl<'a> Lower<'a, Message> for ast::Message {
    fn lower(&'a self, ctx: &'a LowerContext) -> Option<Message> {
        let name = self.name.name.clone();
        let descriptor = ctx.build_child_descriptor(&name);

        // Create child context for nested items
        let child_ctx = ctx.with_child(name.clone());

        // Lower message items
        let mut fields = Vec::new();
        let mut messages = Vec::new();
        let mut enums = Vec::new();

        for item in &self.items {
            match item {
                ast::MessageItem::Field(field) => {
                    if let Some(ir_field) = field.lower(&child_ctx) {
                        fields.push(ir_field);
                    }
                }
                ast::MessageItem::Message(msg) => {
                    if let Some(ir_msg) = msg.lower(&child_ctx) {
                        messages.push(ir_msg);
                    }
                }
                ast::MessageItem::Enum(e) => {
                    if let Some(ir_enum) = e.lower(&child_ctx) {
                        enums.push(ir_enum);
                    }
                }
                _ => {}
            }
        }

        let doc = self.comment.as_ref().and_then(|c| c.lower(ctx));

        Some(Message {
            descriptor,
            name,
            fields,
            messages,
            enums,
            doc,
        })
    }
}