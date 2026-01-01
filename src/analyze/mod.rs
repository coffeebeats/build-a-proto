//! Semantic analysis for 'baproto' schema files.
//!
//! This module performs two-stage analysis:
//! 1. **Cross-file passes**: registration, cycle detection.
//! 2. **Per-file passes**: resolution, validation, etc.

mod error;
mod pass;
mod symbol;

use std::collections::HashMap;

use crate::ast::SourceFile;
use crate::core::SchemaImport;

/* ------------------------------- Mod: Error ------------------------------- */

pub use error::*;

/* ------------------------------- Mod: Symbol ------------------------------ */

pub use symbol::*;

/* -------------------------------------------------------------------------- */
/*                               Struct: Context                              */
/* -------------------------------------------------------------------------- */

/// `Context` holds all analysis state during semantic analysis.
#[derive(Default)]
pub struct Context {
    /// `symbols` is a symbol table containing all type definitions.
    pub symbols: SymbolTable,
    /// `errors` is the list of accumulated analysis errors.
    pub errors: Vec<Error>,
    /// `source_files` contains parsed source files keyed by import path.
    pub source_files: HashMap<SchemaImport, SourceFile>,
}

/* ------------------------------ Impl: Context ----------------------------- */

impl Context {
    /// `add_error` adds an error to the context.
    pub fn add_error(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// `has_errors` returns whether any errors have been recorded.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/* -------------------------------------------------------------------------- */
/*                               Fn: analyze                                  */
/* -------------------------------------------------------------------------- */

/// Performs full semantic analysis on a collection of source files.
///
/// This function orchestrates all analysis passes in the correct order:
/// 1. Multi-file passes (registration, cycle detection)
/// 2. Per-file passes (resolution, type checking, validation)
///
/// Returns an analysis context containing the populated symbol table
/// and any errors encountered.
pub fn analyze(files: Vec<(SchemaImport, SourceFile)>) -> Context {
    let mut ctx = Context::default();

    // Store source files
    for (import, ast) in files {
        ctx.source_files.insert(import, ast);
    }

    // Multi-file passes
    pass::Registration.run(&mut ctx);
    pass::Cycles.run(&mut ctx);

    // If critical errors occurred, stop early
    if ctx.has_errors() {
        return ctx;
    }

    // Per-file passes
    let imports: Vec<_> = ctx.source_files.keys().cloned().collect();
    for import in imports {
        pass::Resolution.run(&mut ctx, &import);
        pass::Types.run(&mut ctx, &import);
        pass::Indices.run(&mut ctx, &import);
        pass::Names.run(&mut ctx, &import);
        pass::Values.run(&mut ctx, &import);
        pass::Limits.run(&mut ctx, &import);
        pass::Encoding.run(&mut ctx, &import);
    }

    ctx
}

/* -------------------------------------------------------------------------- */
/*                            Trait: MultiFilePass                            */
/* -------------------------------------------------------------------------- */

/// A pass that runs once across all files.
///
/// Multi-file passes are used for operations that need global knowledge, such
/// as registering all types or detecting circular dependencies.
pub trait MultiFilePass {
    /// `run` runs the pass on the analysis context.
    fn run(&self, ctx: &mut Context);
}

/* -------------------------------------------------------------------------- */
/*                              Trait: FilePass                               */
/* -------------------------------------------------------------------------- */

/// A pass that runs on each file individually.
///
/// Per-file passes are used for operations that can be performed on each
/// file independently, such as name resolution or validation.
pub trait FilePass {
    /// `run` runs a pass on a single file.
    fn run(&self, ctx: &mut Context, file: &SchemaImport);
}
