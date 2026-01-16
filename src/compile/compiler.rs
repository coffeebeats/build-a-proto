use std::collections::HashSet;
use std::path::PathBuf;

use derive_builder::Builder;

use crate::analyze::Analyzer;
use crate::analyze::Diagnostic;
use crate::analyze::FieldIndexUniqueness;
use crate::analyze::TypeReferenceResolver;
use crate::ast;
use crate::core::{Descriptor, ImportRoot, SchemaImport};
use crate::ir;
use crate::ir::lower::{Lower, LowerContext, TypeKind};
use crate::lex::Span;
use crate::visit::Visitable;

use super::SourceCache;
use super::symbol::Symbols;

/* -------------------------------------------------------------------------- */
/*                              Struct: Compiler                              */
/* -------------------------------------------------------------------------- */

/// `Compiler` orchestrates the compilation process using DFS traversal.
///
/// The compilation process:
/// 1. Parse each schema file
/// 2. Register types in the symbol table
/// 3. Recursively process imports (DFS ensures deps ready before analysis)
/// 4. Run semantic analysis passes
/// 5. Lower to IR and merge incrementally
#[derive(Builder)]
pub struct Compiler {
    /// `diagnostics` contains all reported diagnostics collected during
    /// compilation.
    #[builder(default)]
    pub diagnostics: Vec<Diagnostic>,
    /// Import search paths
    import_roots: Vec<ImportRoot>,
    /// `ir` accumulates the intermediate representation incrementally during
    /// compilation.
    #[builder(default)]
    ir: ir::Schema,
    /// `processed` is the set of already-processed imports (prevents cycles).
    #[builder(default)]
    processed: HashSet<SchemaImport>,
    /// `sources` caches source code by its import path for diagnostic reports.
    #[builder(default)]
    pub sources: SourceCache,
    /// `symbols` tracks types and modules encountered during compilation.
    #[builder(default)]
    pub symbols: Symbols<TypeKind>,
}

/* ---------------------------- Impl: Compiler ------------------------------ */

impl Compiler {
    /// `new` instantiates a new compiler with the provided import search roots.
    pub fn new(import_roots: Vec<ImportRoot>) -> Self {
        CompilerBuilder::default()
            .import_roots(import_roots)
            .build()
            .unwrap()
    }

    /// Compiles a schema import and all its dependencies.
    ///
    /// Returns the symbol table and any diagnostics produced.
    pub fn compile(&mut self, import: SchemaImport) {
        // Skip if already processed (prevents infinite loops).
        if self.processed.contains(&import) {
            return;
        }

        self.processed.insert(import.clone());

        // 1. Parse the schema file
        let ast = match self.parse(&import) {
            Ok(ast) => ast,
            Err(errs) => {
                self.diagnostics.extend(errs);
                return;
            }
        };

        // 2. Register types in symbol table (before processing deps).
        let errors =
            super::TypeCollector::register(&ast, &import, &mut self.symbols, &self.import_roots);
        self.diagnostics.extend(errors);

        // 3. Process imports first (DFS ensures deps ready before analysis).
        for include in ast.iter_includes() {
            match self.resolve_import(&include.path) {
                Ok(dep) => self.compile(dep),
                Err(e) => self.diagnostics.push(e),
            }
        }

        // 4. Run semantic analyzers.
        self.run_analyzers(&ast);

        // 5. Lower to IR and merge.
        self.lower_and_merge(&ast);
    }

    /// Parses a schema file into an AST.
    fn parse(&mut self, import: &SchemaImport) -> Result<ast::Schema, Vec<Diagnostic>> {
        let contents = self.sources.insert(import).map_err(|e| {
            vec![Diagnostic::error(
                Span::default(),
                format!("failed to read file: {}", e),
            )]
        })?;

        // Lex
        let result = crate::lex::lex(&contents, import.clone());
        if !result.errors.is_empty() {
            return Err(result
                .errors
                .into_iter()
                .map(|e| Diagnostic::error(e.span().clone(), e.to_string()))
                .collect());
        }

        let tokens = result.tokens.ok_or_else(|| {
            vec![Diagnostic::error(
                Span::default(),
                "lexing failed with no specific error".to_string(),
            )]
        })?;

        // Parse
        let result = crate::parse::parse(&tokens, import.clone());
        if !result.errors.is_empty() {
            return Err(result
                .errors
                .into_iter()
                .map(|e| Diagnostic::error(e.span().clone(), e.to_string()))
                .collect());
        }

        result.ast.ok_or_else(|| {
            vec![Diagnostic::error(
                Span::default(),
                "parse failed with no specific error".to_string(),
            )]
        })
    }

    /// Resolves an include path to a SchemaImport using import roots.
    fn resolve_import(&self, path: &PathBuf) -> Result<SchemaImport, Diagnostic> {
        for root in &self.import_roots {
            if let Ok(schema) = root.resolve_schema_import(path) {
                return Ok(schema);
            }
        }

        Err(Diagnostic::error(
            Span::default(),
            format!("failed to resolve import: {}", path.display()),
        ))
    }

    /// Helper to run an analyzer and return its diagnostics.
    fn run_analyzer<A>(ast: &ast::Schema, mut analyzer: A) -> Vec<Diagnostic>
    where
        A: Analyzer,
    {
        ast.visit(&mut analyzer);
        analyzer.drain_diagnostics()
    }

    /// Runs all semantic analyzers on the AST.
    fn run_analyzers(&mut self, ast: &ast::Schema) {
        // Declarative list of analyzers to run
        self.diagnostics
            .extend(Self::run_analyzer(ast, FieldIndexUniqueness::default()));

        if let Some(package_name) = ast.get_package_name() {
            self.diagnostics.extend(Self::run_analyzer(
                ast,
                TypeReferenceResolver::new(&self.symbols, Descriptor::from(package_name)),
            ));
        }

        // TODO: Add more analyzers (encoding validation, etc.)
    }

    /// Lowers an AST to IR and merges it into the accumulated schema.
    ///
    /// This handles package merging - if a package already exists in the IR,
    /// the new types are appended to it.
    fn lower_and_merge(&mut self, ast: &ast::Schema) {
        let package_name = match ast.get_package_name() {
            Some(name) => name,
            None => return, // Can't lower without valid package information.
        };

        let ctx = LowerContext::new(&self.symbols, Descriptor::from(package_name));

        // Lower the AST to a Package
        let package = match ast.lower(&ctx) {
            Some(pkg) => pkg,
            None => return, // No package declaration or lowering failed
        };

        if let Some(existing) = self.ir.packages.iter_mut().find(|p| p.name == package.name) {
            existing.messages.extend(package.messages);
            existing.enums.extend(package.enums);
        } else {
            self.ir.packages.push(package);
        }
    }
}

/* ------------------------- Impl: Into<ir::Schema> ------------------------- */

impl From<Compiler> for ir::Schema {
    fn from(value: Compiler) -> Self {
        value.ir
    }
}
