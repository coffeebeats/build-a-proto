use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use thiserror::Error;

use crate::core::Registry;

/* -------------------------------------------------------------------------- */
/*                              Enum: LinkError                               */
/* -------------------------------------------------------------------------- */

#[derive(Error, Debug)]
pub enum LinkError {
    #[error("missing include: {0:?}")]
    MissingInclude(PathBuf),

    #[error("circular dependency detected: {}", format_cycle(.0))]
    CircularDependency(Vec<PathBuf>),
}

fn format_cycle(cycle: &[PathBuf]) -> String {
    cycle
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

/* -------------------------------- Fn: link -------------------------------- */

/// Validates module dependencies and detects circular dependencies.
///
/// This function should be called after all modules have been prepared and
/// registered in the registry. It performs two checks:
///
/// 1. Validates that all module dependencies exist in the registry
/// 2. Detects circular dependencies using depth-first search
pub fn link(registry: &Registry) -> Result<(), LinkError> {
    // 1. Build dependency graph as adjacency list
    let graph: HashMap<&PathBuf, Vec<&PathBuf>> = registry
        .iter_modules()
        .map(|(_, m)| (&m.path, m.deps.iter().collect()))
        .collect();

    // 2. Validate all dependencies exist in registry
    for (_, deps) in &graph {
        for dep in deps {
            if !graph.contains_key(dep) {
                return Err(LinkError::MissingInclude((*dep).clone()));
            }
        }
    }

    // 3. Detect cycles using DFS
    let mut visited = HashSet::new();
    let mut path = Vec::new();

    for node in graph.keys() {
        detect_cycle(&graph, node, &mut visited, &mut path)?;
    }

    Ok(())
}

/* ----------------------------- Fn: detect_cycle --------------------------- */

fn detect_cycle<'a>(
    graph: &HashMap<&'a PathBuf, Vec<&'a PathBuf>>,
    current: &'a PathBuf,
    visited: &mut HashSet<&'a PathBuf>,
    path: &mut Vec<&'a PathBuf>,
) -> Result<(), LinkError> {
    // If current node is in the path, we found a cycle
    if path.contains(&current) {
        path.push(current);
        return Err(LinkError::CircularDependency(
            path.iter().map(|p| (*p).clone()).collect(),
        ));
    }

    // If already visited in a previous DFS traversal, skip
    if visited.contains(current) {
        return Ok(());
    }

    path.push(current);

    // Traverse dependencies
    if let Some(deps) = graph.get(current) {
        for dep in deps {
            detect_cycle(graph, dep, visited, path)?;
        }
    }

    path.pop();
    visited.insert(current);

    Ok(())
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DescriptorBuilder, Module, ModuleBuilder, registry::Kind};

    fn create_module(path: &str, deps: Vec<&str>) -> Module {
        // Use path stem as the package for uniqueness in tests
        let package_name = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        ModuleBuilder::default()
            .path(PathBuf::from(path))
            .package(vec![package_name])
            .deps(deps.into_iter().map(PathBuf::from).collect())
            .build()
            .unwrap()
    }

    fn register_module(registry: &mut Registry, module: Module) {
        let desc = DescriptorBuilder::default()
            .package(module.package.clone())
            .build()
            .unwrap();
        registry.insert(desc, Kind::Module(module));
    }

    #[test]
    fn test_link_valid_dependencies() {
        // Given: A registry with valid module dependencies (A -> B -> C).
        let mut registry = Registry::default();
        register_module(&mut registry, create_module("/a.baproto", vec!["/b.baproto"]));
        register_module(&mut registry, create_module("/b.baproto", vec!["/c.baproto"]));
        register_module(&mut registry, create_module("/c.baproto", vec![]));

        // When: The registry is linked.
        let result = link(&registry);

        // Then: No error is returned.
        assert!(result.is_ok());
    }

    #[test]
    fn test_link_detects_self_cycle() {
        // Given: A registry where a module depends on itself.
        let mut registry = Registry::default();
        register_module(&mut registry, create_module("/a.baproto", vec!["/a.baproto"]));

        // When: The registry is linked.
        let result = link(&registry);

        // Then: A circular dependency error is returned.
        assert!(result.is_err());
        match result.unwrap_err() {
            LinkError::CircularDependency(cycle) => {
                assert!(cycle.contains(&PathBuf::from("/a.baproto")));
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_link_detects_two_node_cycle() {
        // Given: A registry where A -> B -> A.
        let mut registry = Registry::default();
        register_module(&mut registry, create_module("/a.baproto", vec!["/b.baproto"]));
        register_module(&mut registry, create_module("/b.baproto", vec!["/a.baproto"]));

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
        let mut registry = Registry::default();
        register_module(&mut registry, create_module("/a.baproto", vec!["/b.baproto"]));
        register_module(&mut registry, create_module("/b.baproto", vec!["/c.baproto"]));
        register_module(&mut registry, create_module("/c.baproto", vec!["/a.baproto"]));

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
        let mut registry = Registry::default();
        register_module(&mut registry, create_module("/a.baproto", vec!["/missing.baproto"]));

        // When: The registry is linked.
        let result = link(&registry);

        // Then: A missing include error is returned.
        assert!(result.is_err());
        match result.unwrap_err() {
            LinkError::MissingInclude(path) => {
                assert_eq!(path, PathBuf::from("/missing.baproto"));
            }
            _ => panic!("Expected MissingInclude error"),
        }
    }
}
