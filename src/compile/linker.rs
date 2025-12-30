use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::core::Registry;
use crate::core::SchemaImport;
use crate::core::path::PathValidationError;

/* -------------------------------------------------------------------------- */
/*                              Enum: LinkError                               */
/* -------------------------------------------------------------------------- */

#[derive(Error, Debug)]
pub enum LinkError {
    #[error("circular dependency detected: {}", format_cycle(.0))]
    CircularDependency(Vec<SchemaImport>),

    #[error("invalid module path: {0}")]
    InvalidModule(#[from] PathValidationError),

    #[error("missing import: {0:?}")]
    MissingInclude(SchemaImport),
}

/* ---------------------------- Fn: format_cycle ---------------------------- */

fn format_cycle(cycle: &[SchemaImport]) -> String {
    cycle
        .iter()
        .map(|p| p.as_path().display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

/* -------------------------------------------------------------------------- */
/*                                  Fn: link                                  */
/* -------------------------------------------------------------------------- */

/// Validates module dependencies and detects circular dependencies.
///
/// This function should be called after all modules have been prepared and
/// registered in the registry. It performs two checks:
///
/// 1. Validates that all module dependencies exist in the registry.
/// 2. Detects circular dependencies using depth-first search.
pub fn link(registry: &Registry) -> Result<(), LinkError> {
    let mut graph: HashMap<SchemaImport, Vec<SchemaImport>> = HashMap::new();

    // 1. Build a dependency graph for the registered modules.
    for (_, m) in registry.iter_modules() {
        let schema_import = SchemaImport::try_from(m.path.as_path())?;
        graph.insert(schema_import, m.deps.clone());
    }

    // 2. Validate that all module dependencies were registered.
    for deps in graph.values() {
        for dep in deps {
            if !graph.contains_key(dep) {
                return Err(LinkError::MissingInclude(dep.clone()));
            }
        }
    }

    let mut path = Vec::new();
    let mut visited = HashSet::new();

    // 3. Verify that there are no dependency cycles.
    for node in graph.keys() {
        detect_cycle(&graph, node, &mut visited, &mut path)?;
    }

    Ok(())
}

/* ----------------------------- Fn: detect_cycle --------------------------- */

fn detect_cycle(
    graph: &HashMap<SchemaImport, Vec<SchemaImport>>,
    current: &SchemaImport,
    visited: &mut HashSet<SchemaImport>,
    path: &mut Vec<SchemaImport>,
) -> Result<(), LinkError> {
    if let Some(i) = path.iter().position(|s| s == current) {
        let mut cycle = path[i..].to_vec();
        cycle.push(current.clone());

        return Err(LinkError::CircularDependency(cycle));
    }

    if visited.contains(current) {
        return Ok(());
    }

    visited.insert(current.clone());

    path.push(current.clone());

    if let Some(deps) = graph.get(current) {
        for dep in deps {
            detect_cycle(graph, dep, visited, path)?;
        }
    }

    path.pop();

    Ok(())
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use std::path::Path;
    use tempfile::TempDir;

    use crate::core::DescriptorBuilder;
    use crate::core::Module;
    use crate::core::ModuleBuilder;
    use crate::core::PackageName;
    use crate::core::registry::Kind;

    use super::*;

    #[test]
    fn test_link_valid_dependencies() {
        // Given: A registry with valid module dependencies (A -> B -> C).
        let dir = TempDir::new().unwrap();
        let mut schema_cache = std::collections::HashMap::new();
        let mut registry = Registry::default();
        register_module(
            &mut registry,
            create_module(&dir, "a.baproto", vec!["b.baproto"], &mut schema_cache),
        );
        register_module(
            &mut registry,
            create_module(&dir, "b.baproto", vec!["c.baproto"], &mut schema_cache),
        );
        register_module(
            &mut registry,
            create_module(&dir, "c.baproto", vec![], &mut schema_cache),
        );

        // When: The registry is linked.
        let result = link(&registry);

        // Then: No error is returned.
        if let Err(e) = &result {
            eprintln!("Unexpected error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_link_detects_self_cycle() {
        // Given: A registry where a module depends on itself.
        let dir = TempDir::new().unwrap();
        let mut schema_cache = std::collections::HashMap::new();
        let mut registry = Registry::default();
        register_module(
            &mut registry,
            create_module(&dir, "a.baproto", vec!["a.baproto"], &mut schema_cache),
        );

        // When: The registry is linked.
        let result = link(&registry);

        // Then: A circular dependency error is returned.
        assert!(result.is_err());
        match result.unwrap_err() {
            LinkError::CircularDependency(cycle) => {
                assert!(!cycle.is_empty());
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_link_detects_two_node_cycle() {
        // Given: A registry where A -> B -> A.
        let dir = TempDir::new().unwrap();
        let mut schema_cache = std::collections::HashMap::new();
        let mut registry = Registry::default();
        register_module(
            &mut registry,
            create_module(&dir, "a.baproto", vec!["b.baproto"], &mut schema_cache),
        );
        register_module(
            &mut registry,
            create_module(&dir, "b.baproto", vec!["a.baproto"], &mut schema_cache),
        );

        // When: The registry is linked.
        let result = link(&registry);

        // Then: A circular dependency error is returned.
        assert!(result.is_err());
        match result.unwrap_err() {
            LinkError::CircularDependency(cycle) => {
                assert!(cycle.len() >= 2);
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_link_detects_three_node_cycle() {
        // Given: A registry where A -> B -> C -> A.
        let dir = TempDir::new().unwrap();
        let mut schema_cache = std::collections::HashMap::new();
        let mut registry = Registry::default();
        register_module(
            &mut registry,
            create_module(&dir, "a.baproto", vec!["b.baproto"], &mut schema_cache),
        );
        register_module(
            &mut registry,
            create_module(&dir, "b.baproto", vec!["c.baproto"], &mut schema_cache),
        );
        register_module(
            &mut registry,
            create_module(&dir, "c.baproto", vec!["a.baproto"], &mut schema_cache),
        );

        // When: The registry is linked.
        let result = link(&registry);

        // Then: A circular dependency error is returned.
        assert!(result.is_err());
        match result.unwrap_err() {
            LinkError::CircularDependency(cycle) => {
                assert!(cycle.len() >= 3);
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_link_missing_include_error() {
        // Given: A registry where a module depends on a non-existent module.
        let dir = TempDir::new().unwrap();
        let mut schema_cache = std::collections::HashMap::new();
        let mut registry = Registry::default();
        register_module(
            &mut registry,
            create_module(
                &dir,
                "a.baproto",
                vec!["missing.baproto"],
                &mut schema_cache,
            ),
        );

        // When: The registry is linked.
        let result = link(&registry);

        // Then: A missing include error is returned.
        assert!(result.is_err());
        match result.unwrap_err() {
            LinkError::MissingInclude(schema) => {
                assert!(schema.as_path().ends_with("missing.baproto"));
            }
            _ => panic!("Expected MissingInclude error"),
        }
    }

    /* -------------------------- Fn: create_module ------------------------- */

    fn create_module(
        dir: &TempDir,
        name: &str,
        deps: Vec<&str>,
        schema_cache: &mut std::collections::HashMap<String, SchemaImport>,
    ) -> Module {
        if !schema_cache.contains_key(name) {
            let file_path = dir.path().join(name);
            std::fs::write(&file_path, "").unwrap();
            schema_cache.insert(name.to_string(), SchemaImport::try_from(file_path).unwrap());
        }

        let package_name = Path::new(name)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let dep_imports: Vec<SchemaImport> = deps
            .into_iter()
            .map(|d| {
                if !schema_cache.contains_key(d) {
                    let dep_path = dir.path().join(d);
                    std::fs::write(&dep_path, "").unwrap();
                    schema_cache.insert(d.to_string(), SchemaImport::try_from(dep_path).unwrap());
                }
                schema_cache.get(d).unwrap().clone()
            })
            .collect();

        let path = schema_cache.get(name).unwrap().as_path().to_path_buf();

        ModuleBuilder::default()
            .path(path)
            .package(PackageName::try_from(vec![package_name]).unwrap())
            .deps(dep_imports)
            .build()
            .unwrap()
    }

    /* ------------------------- Fn: register_module ------------------------ */

    fn register_module(registry: &mut Registry, module: Module) {
        let desc = DescriptorBuilder::default()
            .package(module.package.clone())
            .build()
            .unwrap();
        registry.insert(desc, Kind::Module(module));
    }
}
