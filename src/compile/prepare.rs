//! DEPRECATED: This module will be replaced by `register.rs` as part of the
//! semantic validation refactoring.
//!
//! The new `register` phase:
//! - Preserves spans for better error reporting
//! - Separates type registration from type lowering
//! - Uses the `Symbols` table instead of directly building the `Registry`
//!
//! This module is kept temporarily until the command pipeline is updated (Step 6).

use chumsky::error::Rich;
use std::path::Path;

use crate::ast::Schema;
use crate::core::ImportRoot;
use crate::core::Registry;
use crate::core::SchemaImport;
use crate::lex::Span;
use crate::parse::ParseError;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Prepare                                */
/* -------------------------------------------------------------------------- */

pub fn prepare<'a>(
    _: &'a SchemaImport,
    _: &[ImportRoot],
    _: &'a mut Registry,
    _: &'a Schema,
) -> Result<(), ParseError<'a>> {
    todo!() // TODO: Replace this with new parsing implementation.
}

/* ------------------------ Fn: resolve_include_path ------------------------ */

/// Resolves an include path by searching through import roots in order.
///
/// Returns a validated SchemaImport for the first matching .baproto file.
fn resolve_include_path<'a>(
    path: &Path,
    import_roots: &[ImportRoot],
    span: Span,
) -> Result<SchemaImport, ParseError<'a>> {
    import_roots
        .iter()
        .find_map(|root| root.resolve_schema_import(path).ok())
        .ok_or_else(|| {
            Rich::custom(
                span,
                format!(
                    "include path '{}' not found in any import root",
                    path.display(),
                ),
            )
        })
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_include_path_finds_file_in_first_root() {
        // Given: A temp directory with a .baproto file.
        let root = TempDir::new().unwrap();
        let file_path = root.path().join("dep.baproto");
        fs::write(&file_path, "").unwrap();

        let import_roots = vec![ImportRoot::try_from(root.path()).unwrap()];
        let span = Span::from(0..10);

        // When: The include path is resolved.
        let result = resolve_include_path(Path::new("dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the file.
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.as_path(), &file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_searches_roots_in_order() {
        // Given: Two temp directories, with the file only in the second.
        let root1 = TempDir::new().unwrap();
        let root2 = TempDir::new().unwrap();
        let file_path = root2.path().join("dep.baproto");
        fs::write(&file_path, "").unwrap();

        let import_roots = vec![
            ImportRoot::try_from(root1.path()).unwrap(),
            ImportRoot::try_from(root2.path()).unwrap(),
        ];
        let span = Span::from(0..10);

        // When: The include path is resolved.
        let result = resolve_include_path(Path::new("dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the file in the second root.
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.as_path(), &file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_prefers_first_root() {
        // Given: Two temp directories, both with the same file name.
        let root1 = TempDir::new().unwrap();
        let root2 = TempDir::new().unwrap();
        let file1 = root1.path().join("dep.baproto");
        let file2 = root2.path().join("dep.baproto");
        fs::write(&file1, "first").unwrap();
        fs::write(&file2, "second").unwrap();

        let import_roots = vec![
            ImportRoot::try_from(root1.path()).unwrap(),
            ImportRoot::try_from(root2.path()).unwrap(),
        ];
        let span = Span::from(0..10);

        // When: The include path is resolved.
        let result = resolve_include_path(Path::new("dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the file in the first root.
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.as_path(), &file1.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_nested_path() {
        // Given: A temp directory with a nested .baproto file.
        let root = TempDir::new().unwrap();
        let nested_dir = root.path().join("sub").join("dir");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("dep.baproto");
        fs::write(&file_path, "").unwrap();

        let import_roots = vec![ImportRoot::try_from(root.path()).unwrap()];
        let span = Span::from(0..10);

        // When: The include path with subdirectories is resolved.
        let result = resolve_include_path(Path::new("sub/dir/dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the nested file.
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.as_path(), &file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_not_found_returns_error() {
        // Given: A temp directory without the requested file.
        let root = TempDir::new().unwrap();
        let import_roots = vec![ImportRoot::try_from(root.path()).unwrap()];
        let span = Span::from(0..10);

        // When: A non-existent include path is resolved.
        let result = resolve_include_path(Path::new("missing.baproto"), &import_roots, span);

        // Then: An error is returned.
        assert!(result.is_err());
    }
}
