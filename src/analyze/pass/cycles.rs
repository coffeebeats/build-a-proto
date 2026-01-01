//! Cycle detection pass.
//!
//! This pass detects circular dependencies between modules.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::MultiFilePass;
use crate::core::SchemaImport;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                               Struct: Cycles                               */
/* -------------------------------------------------------------------------- */

/// Cycle detection pass.
///
/// Detects circular dependencies between modules using depth-first search.
pub struct Cycles;

/* --------------------------- Impl: MultiFilePass -------------------------- */

impl MultiFilePass for Cycles {
    fn run(&self, ctx: &mut Context) {
        let mut graph: HashMap<SchemaImport, Vec<SchemaImport>> = HashMap::new();

        for (import, entry) in ctx.symbols.iter_modules() {
            graph.insert(import.clone(), entry.deps.clone());
        }

        for (import, deps) in &graph {
            for dep in deps {
                if !graph.contains_key(dep) {
                    let span = find_include_span(ctx, import, dep);
                    ctx.add_error(Error {
                        file: import.clone(),
                        span,
                        kind: ErrorKind::MissingInclude(dep.as_path().display().to_string()),
                    });
                }
            }
        }

        let mut visited = HashSet::new();
        let mut path = Vec::new();

        for node in graph.keys() {
            if let Some(cycle) = detect_cycle(&graph, node, &mut visited, &mut path) {
                let first = &cycle[0];
                let span = ctx
                    .source_files
                    .get(first)
                    .map(|ast| ast.span)
                    .unwrap_or_else(|| Span::from(0..0));

                ctx.add_error(Error {
                    file: first.clone(),
                    span,
                    kind: ErrorKind::CircularDependency(cycle),
                });

                break; // Report only first cycle
            }
        }
    }
}

/* ----------------------------- Fn: detect_cycle --------------------------- */

fn detect_cycle(
    graph: &HashMap<SchemaImport, Vec<SchemaImport>>,
    current: &SchemaImport,
    visited: &mut HashSet<SchemaImport>,
    path: &mut Vec<SchemaImport>,
) -> Option<Vec<SchemaImport>> {
    if let Some(i) = path.iter().position(|s| s == current) {
        let mut cycle = path[i..].to_vec();
        cycle.push(current.clone());

        return Some(cycle);
    }

    if visited.contains(current) {
        return None;
    }

    visited.insert(current.clone());
    path.push(current.clone());

    if let Some(deps) = graph.get(current) {
        for dep in deps {
            if let Some(cycle) = detect_cycle(graph, dep, visited, path) {
                return Some(cycle);
            }
        }
    }

    path.pop();
    None
}

/* -------------------------- Fn: find_include_span ------------------------- */

fn find_include_span(ctx: &Context, import: &SchemaImport, dep: &SchemaImport) -> Span {
    ctx.source_files
        .get(import)
        .and_then(|ast| {
            ast.includes
                .iter()
                .find(|inc| inc.path.file_name() == dep.as_path().file_name())
                .map(|inc| inc.span)
        })
        .unwrap_or_else(|| Span::from(0..0))
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::ModuleEntry;
    use crate::core::PackageName;
    use std::path::PathBuf;

    #[test]
    fn test_detects_simple_cycle() {
        // Given: Two modules with circular dependency (A -> B -> A).
        let mut ctx = Context::default();
        let a = make_import("a.baproto");
        let b = make_import("b.baproto");

        ctx.symbols.register_module(
            a.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![b.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            b.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![a.clone()],
                types: Vec::new(),
            },
        );

        // When: Running the cycle detection pass.
        Cycles.run(&mut ctx);

        // Then: A circular dependency error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::CircularDependency(_)
        ));
    }

    #[test]
    fn test_detects_three_way_cycle() {
        // Given: Three modules with circular dependency (A -> B -> C -> A).
        let mut ctx = Context::default();
        let a = make_import("a.baproto");
        let b = make_import("b.baproto");
        let c = make_import("c.baproto");

        ctx.symbols.register_module(
            a.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![b.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            b.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![c.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            c.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![a.clone()],
                types: Vec::new(),
            },
        );

        // When: Running the cycle detection pass.
        Cycles.run(&mut ctx);

        // Then: A circular dependency error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            ctx.errors[0].kind,
            ErrorKind::CircularDependency(_)
        ));
    }

    #[test]
    fn test_no_error_for_valid_dependency_chain() {
        // Given: A valid linear dependency chain (A -> B -> C).
        let mut ctx = Context::default();
        let a = make_import("a.baproto");
        let b = make_import("b.baproto");
        let c = make_import("c.baproto");

        ctx.symbols.register_module(
            a.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![b.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            b.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![c.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            c,
            ModuleEntry {
                package: make_package(&["test"]),
                deps: Vec::new(),
                types: Vec::new(),
            },
        );

        // When: Running the cycle detection pass.
        Cycles.run(&mut ctx);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_no_error_for_diamond_dependency() {
        // Given: A diamond dependency pattern (A -> B, A -> C, B -> D, C -> D).
        let mut ctx = Context::default();
        let a = make_import("a.baproto");
        let b = make_import("b.baproto");
        let c = make_import("c.baproto");
        let d = make_import("d.baproto");

        ctx.symbols.register_module(
            a.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![b.clone(), c.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            b.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![d.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            c.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![d.clone()],
                types: Vec::new(),
            },
        );
        ctx.symbols.register_module(
            d,
            ModuleEntry {
                package: make_package(&["test"]),
                deps: Vec::new(),
                types: Vec::new(),
            },
        );

        // When: Running the cycle detection pass.
        Cycles.run(&mut ctx);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_detects_missing_include() {
        // Given: A module that depends on a non-existent module.
        let mut ctx = Context::default();
        let a = make_import("a.baproto");
        let missing = make_import("missing.baproto");

        ctx.symbols.register_module(
            a.clone(),
            ModuleEntry {
                package: make_package(&["test"]),
                deps: vec![missing.clone()],
                types: Vec::new(),
            },
        );

        // When: Running the cycle detection pass.
        Cycles.run(&mut ctx);

        // Then: A missing include error should be reported.
        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(ctx.errors[0].kind, ErrorKind::MissingInclude(_)));
    }

    #[test]
    fn test_no_error_for_module_with_no_dependencies() {
        // Given: A module with no dependencies.
        let mut ctx = Context::default();
        let a = make_import("a.baproto");

        ctx.symbols.register_module(
            a,
            ModuleEntry {
                package: make_package(&["test"]),
                deps: Vec::new(),
                types: Vec::new(),
            },
        );

        // When: Running the cycle detection pass.
        Cycles.run(&mut ctx);

        // Then: No errors should be reported.
        assert!(!ctx.has_errors());
    }

    /* --------------------------- Fn: make_import -------------------------- */

    fn make_import(filename: &str) -> SchemaImport {
        let temp = tempfile::Builder::new()
            .prefix(filename)
            .suffix(".baproto")
            .tempfile()
            .unwrap();
        SchemaImport::try_from(temp.path().to_path_buf()).unwrap()
    }

    /* -------------------------- Fn: make_package -------------------------- */

    fn make_package(parts: &[&str]) -> PackageName {
        PackageName::try_from(parts.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }
}