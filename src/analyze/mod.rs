mod analyzers;
mod diagnostic;
mod reporter;

/* ----------------------------- Mod: Analyzers ----------------------------- */

pub use analyzers::*;

/* ----------------------------- Mod: Diagnostic ---------------------------- */

pub use diagnostic::*;

/* ------------------------------ Mod: Reporter ----------------------------- */

pub use reporter::*;

/* -------------------------------------------------------------------------- */
/*                              Trait: Analyzer                               */
/* -------------------------------------------------------------------------- */

use crate::visit::Visitor;

/// `Analyzer` defines the interface for semantic analysis passes.
///
/// Analyzers implement the `Visitor` trait to walk the AST and accumulate
/// diagnostics internally. The `drain_diagnostics` method allows the compiler
/// to extract diagnostics without consuming the analyzer, supporting:
/// - Incremental error reporting
/// - Cross-file analysis with maintained state
/// - Flexible diagnostic collection strategies
pub trait Analyzer: for<'ast> Visitor<'ast> {
    /// Drains and returns all diagnostics collected by this analyzer.
    ///
    /// After calling this method, the analyzer's internal diagnostic
    /// collection should be empty, ready to accumulate new diagnostics.
    fn drain_diagnostics(&mut self) -> Vec<Diagnostic>;
}
