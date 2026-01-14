use crate::visit::Visitor;

/* ----------------------------- Mod: Analyzers ----------------------------- */

mod analyzers;
pub use analyzers::*;

/* ----------------------------- Mod: Diagnostic ---------------------------- */

mod diagnostic;
pub use diagnostic::*;

/* ------------------------------ Mod: Reporter ----------------------------- */

mod reporter;
pub use reporter::*;

/* -------------------------------------------------------------------------- */
/*                              Trait: Analyzer                               */
/* -------------------------------------------------------------------------- */

/// `Analyzer` defines the interface for semantic analysis passes.
///
/// Analyzers implement the `Visitor` trait to walk the AST and accumulate
/// diagnostics internally. The `drain_diagnostics` method allows the compiler
/// to extract diagnostics without consuming the analyzer, supporting:
/// - Incremental error reporting
/// - Cross-file analysis with maintained state
/// - Flexible diagnostic collection strategies
pub trait Analyzer: for<'ast> Visitor<'ast> {
    /// `drain_diagnostics` drains and returns all diagnostics collected by this
    /// analyzer.
    ///
    /// NOTE: After calling this method, the analyzer's internal diagnostic
    /// state should be empty, ready to accumulate new diagnostics.
    fn drain_diagnostics(&mut self) -> Vec<Diagnostic>;
}
