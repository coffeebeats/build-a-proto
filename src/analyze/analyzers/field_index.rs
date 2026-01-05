use std::collections::HashMap;

use crate::analyze::{Analyzer, Diagnostic};
use crate::ast;
use crate::lex::Span;
use crate::visit::{Visitor, walk};

/* -------------------------------------------------------------------------- */
/*                       Analyzer: FieldIndexUniqueness                       */
/* -------------------------------------------------------------------------- */

/// `FieldIndexUniqueness` validates that field indices within each message and
/// enum variant indices within each enum are unique.
///
/// This analyzer implements the `Visitor` trait and walks the AST to check
/// for duplicate field/variant indices. When duplicates are found, an error
/// diagnostic is generated.
pub struct FieldIndexUniqueness {
    diagnostics: Vec<Diagnostic>,
}

/* ------------------------ Impl: FieldIndexUniqueness ---------------------- */

impl FieldIndexUniqueness {
    /// Creates a new field index uniqueness analyzer.
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
}

impl Default for FieldIndexUniqueness {
    fn default() -> Self {
        Self::new()
    }
}

/* ------------------------- Impl: Analyzer --------------------------------- */

impl Analyzer for FieldIndexUniqueness {
    fn drain_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

/* ----------------------- Impl: Visitor (Messages) ------------------------- */

impl<'ast> Visitor<'ast> for FieldIndexUniqueness {
    fn visit_message(&mut self, msg: &'ast ast::Message) {
        let mut seen: HashMap<u64, Span> = HashMap::new();

        for item in &msg.items {
            if let ast::MessageItem::Field(field) = item {
                if let Some(index) = &field.index {
                    let val = index.value.value;
                    if let Some(prev_span) = seen.get(&val) {
                        self.diagnostics.push(Diagnostic::error(
                            index.span.clone(),
                            format!(
                                "duplicate field index {} (previously defined at {:?})",
                                val, prev_span
                            ),
                        ));
                    } else {
                        seen.insert(val, index.span.clone());
                    }
                }
            }
        }

        walk::walk_message(self, msg);
    }

    fn visit_enum(&mut self, enum_: &'ast ast::Enum) {
        let mut seen: HashMap<u64, Span> = HashMap::new();

        for item in &enum_.items {
            let index_opt = match item {
                ast::EnumItem::UnitVariant(variant) => variant.index.as_ref(),
                ast::EnumItem::FieldVariant(field) => field.index.as_ref(),
                ast::EnumItem::CommentBlock(_) => None,
            };

            if let Some(index) = index_opt {
                let val = index.value.value;
                if let Some(prev_span) = seen.get(&val) {
                    self.diagnostics.push(Diagnostic::error(
                        index.span.clone(),
                        format!(
                            "duplicate variant index {} (previously defined at {:?})",
                            val, prev_span
                        ),
                    ));
                } else {
                    seen.insert(val, index.span.clone());
                }
            }
        }

        walk::walk_enum(self, enum_);
    }
}
