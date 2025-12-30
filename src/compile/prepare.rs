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

use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::EnumBuilder;
use crate::core::Field;
use crate::core::ImportRoot;
use crate::core::MessageBuilder;
use crate::core::Module;
use crate::core::Registry;
use crate::core::SchemaImport;
use crate::core::VariantKind;
use crate::core::registry;
use crate::lex::Span;
use crate::parse::Expr;
use crate::parse::ParseError;
use crate::syntax::PackageName;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Prepare                                */
/* -------------------------------------------------------------------------- */

pub fn prepare<'a>(
    schema_import: &'a SchemaImport,
    import_roots: &[ImportRoot],
    registry: &'a mut Registry,
    exprs: Vec<crate::lex::Spanned<Expr<'a>, Span>>,
) -> Result<(), ParseError<'a>> {
    let mut enums: Vec<crate::parse::Enum> = vec![];
    let mut messages: Vec<crate::parse::Message> = vec![];

    let mut module = Module::new(schema_import.as_path().to_path_buf());

    // First, inspect all expressions so all definitions can be registered.
    for spanned_expr in exprs {
        match spanned_expr.inner {
            Expr::Comment(_) => {} // Skip
            Expr::Enum(enm) => enums.push(enm),
            Expr::Message(msg) => messages.push(msg),
            Expr::Package(segments) => {
                // NOTE: The parser has already validated the package name syntax.
                module.package =
                    PackageName::try_from(segments.into_iter().collect::<Vec<_>>()).unwrap();
            }
            Expr::Include(include) => {
                module.deps.push(resolve_include_path(
                    &include,
                    import_roots,
                    spanned_expr.span,
                )?);
            }
            _ => unreachable!(),
        }
    }

    let package = &module.package;

    fn register_enm(
        registry: &mut Registry,
        scope: Descriptor,
        mut enm: crate::parse::Enum,
    ) -> Descriptor {
        debug_assert!(scope.name.is_none());

        enm.variants.sort_by(|l, r| {
            let l = match l {
                crate::parse::VariantKind::Field(field) => field.index.as_ref().map(|s| s.inner),
                crate::parse::VariantKind::Variant(variant) => {
                    variant.index.as_ref().map(|s| s.inner)
                }
            };
            let r = match r {
                crate::parse::VariantKind::Field(field) => field.index.as_ref().map(|s| s.inner),
                crate::parse::VariantKind::Variant(variant) => {
                    variant.index.as_ref().map(|s| s.inner)
                }
            };

            l.cmp(&r)
        });

        let d = DescriptorBuilder::default()
            .package(scope.package)
            .path(scope.path)
            .name(enm.name.inner.to_owned())
            .build()
            .unwrap();

        let e = EnumBuilder::default()
            .comment(
                enm.comment
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .name(enm.name.inner)
            .variants(enm.variants.into_iter().map(VariantKind::from).collect())
            .build()
            .unwrap();

        registry.insert(d.clone(), registry::Kind::Enum(e));

        d
    }

    fn register_msg(
        registry: &mut Registry,
        scope: Descriptor,
        mut msg: crate::parse::Message,
    ) -> Descriptor {
        debug_assert!(scope.name.is_none());

        msg.fields.sort_by(|l, r| {
            l.index
                .as_ref()
                .map(|s| s.inner)
                .cmp(&r.index.as_ref().map(|s| s.inner))
        });

        let d = DescriptorBuilder::default()
            .package(scope.package.clone())
            .path(scope.path.clone())
            .name(msg.name.inner.to_owned())
            .build()
            .unwrap();

        let mut m = MessageBuilder::default()
            .comment(
                msg.comment
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .name(msg.name.inner)
            .fields(msg.fields.into_iter().map(Field::from).collect())
            .build()
            .unwrap();

        let mut scope = scope.clone();
        scope.path.push(msg.name.inner.to_owned());

        m.enums = msg
            .enums
            .into_iter()
            .map(|enm| register_enm(registry, scope.clone(), enm))
            .collect();

        m.messages = msg
            .messages
            .into_iter()
            .map(|m| register_msg(registry, scope.clone(), m))
            .collect();

        registry.insert(d.clone(), registry::Kind::Message(m));

        d
    }

    let scope = DescriptorBuilder::default()
        .package(package.clone())
        .build()
        .unwrap();

    module.enums = enums
        .into_iter()
        .map(|enm| register_enm(registry, scope.clone(), enm))
        .collect();

    module.messages = messages
        .into_iter()
        .map(|msg| register_msg(registry, scope.clone(), msg))
        .collect();

    registry.insert(scope, registry::Kind::Module(module));

    Ok(())
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
/// TODO: Migrate these tests once [`crate::compile::register`] is utilized.
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
