use crate::ast;
use crate::core::Descriptor;
use crate::ir::Package;

use super::{Lower, LowerContext};

/* -------------------------------------------------------------------------- */
/*                                 Impl: Lower                                */
/* -------------------------------------------------------------------------- */

/* --------------------------- Struct: ast::Schema -------------------------- */

impl<'a> Lower<'a, Package> for ast::Schema {
    fn lower(&'a self, ctx: &'a LowerContext) -> Option<Package> {
        // Extract package name from schema
        let package = self.get_package_name()?;
        let package_path = package.to_string();

        // Create root context for this package (reuse symbols from parent context)
        let root = Descriptor {
            package,
            path: vec![],
            name: None,
        };
        let pkg_ctx = LowerContext::new(ctx.symbols, root);

        // Lower top-level messages and enums
        let mut messages = Vec::new();
        let mut enums = Vec::new();

        for item in &self.items {
            match item {
                ast::SchemaItem::Message(msg) => {
                    if let Some(ir_msg) = msg.lower(&pkg_ctx) {
                        messages.push(ir_msg);
                    }
                }
                ast::SchemaItem::Enum(e) => {
                    if let Some(ir_enum) = e.lower(&pkg_ctx) {
                        enums.push(ir_enum);
                    }
                }
                _ => {}
            }
        }

        Some(Package {
            path: package_path,
            messages,
            enums,
        })
    }
}
