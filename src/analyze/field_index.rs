use std::collections::HashMap;

use crate::analyze::{Analyzer, Diagnostic};
use crate::ast;
use crate::lex::Span;
use crate::visit::{Visitor, walk};

/* -------------------------------------------------------------------------- */
/*                       Analyzer: FieldIndexUniqueness                       */
/* -------------------------------------------------------------------------- */

/// `FieldIndexUniqueness` validates that field indices within each
/// [`ast::Message`] and [`ast::Enum`]are unique.
#[derive(Default)]
pub struct FieldIndexUniqueness {
    diagnostics: Vec<Diagnostic>,
}

/* ----------------------- Impl: FieldIndexUniqueness ----------------------- */

impl FieldIndexUniqueness {
    fn diagnostic(value: u64, first: &Span, second: &Span) -> Diagnostic {
        Diagnostic::error(
            second.clone(),
            format!(
                "duplicate index {} (previously defined at {:?})",
                value, first
            ),
        )
    }
}

/* ------------------------- Impl: Analyzer --------------------------------- */

impl Analyzer for FieldIndexUniqueness {
    fn drain_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

/* ------------------------------ Impl: Visitor ----------------------------- */

impl<'ast> Visitor<'ast> for FieldIndexUniqueness {
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
                    let error = FieldIndexUniqueness::diagnostic(val, prev_span, &index.span);
                    self.diagnostics.push(error);
                } else {
                    seen.insert(val, index.span.clone());
                }
            }
        }

        walk::walk_enum(self, enum_);
    }

    fn visit_message(&mut self, msg: &'ast ast::Message) {
        let mut seen: HashMap<u64, Span> = HashMap::new();

        for item in &msg.items {
            if let ast::MessageItem::Field(field) = item {
                if let Some(index) = &field.index {
                    let val = index.value.value;
                    if let Some(prev_span) = seen.get(&val) {
                        let error = FieldIndexUniqueness::diagnostic(val, prev_span, &index.span);
                        self.diagnostics.push(error);
                    } else {
                        seen.insert(val, index.span.clone());
                    }
                }
            }
        }

        walk::walk_message(self, msg);
    }
}
