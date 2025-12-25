use chumsky::error::Rich;
use std::path::Path;
use std::path::PathBuf;

use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::EnumBuilder;
use crate::core::Field;
use crate::core::MessageBuilder;
use crate::core::Registry;
use crate::core::VariantKind;
use crate::core::registry;
use crate::parse::Expr;
use crate::parse::ParseError;
use crate::parse::Span;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Prepare                                */
/* -------------------------------------------------------------------------- */

pub fn prepare<'a, P: AsRef<Path>>(
    path: &'a P,
    import_roots: &[PathBuf],
    registry: &'a mut Registry,
    exprs: Vec<(Expr<'a>, Span)>,
) -> Result<(), ParseError<'a>> {
    let mut enums: Vec<crate::parse::Enum> = vec![];
    let mut messages: Vec<crate::parse::Message> = vec![];

    let mut module = crate::core::Module::new(path.as_ref().to_path_buf());

    // First, inspect all expressions so all definitions can be registered.
    for (expr, span) in exprs {
        match expr {
            Expr::Comment(_) => {} // Skip
            Expr::Enum(enm) => enums.push(enm),
            Expr::Message(msg) => messages.push(msg),
            Expr::Package(pkg) => {
                // FIXME: This should happen during parsing.
                module.package = pkg.split(".").map(str::to_owned).collect();
            }
            Expr::Include(include) => {
                // HACK: This is a simple enough way to get around the fact that
                // other modules don't exist yet and we don't have access to a
                // cross-module cache. A second pass will be required to properly
                // link modules in the `Registry`.
                module.deps.push(
                    DescriptorBuilder::default()
                        .name(
                            resolve_include_path(include, import_roots, span)?
                                .to_str()
                                .unwrap(),
                        )
                        .build()
                        .unwrap(),
                )
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
                crate::parse::VariantKind::Field(field) => field.index,
                crate::parse::VariantKind::Variant(variant) => variant.index,
            };
            let r = match r {
                crate::parse::VariantKind::Field(field) => field.index,
                crate::parse::VariantKind::Variant(variant) => variant.index,
            };

            l.cmp(&r)
        });

        let d = DescriptorBuilder::default()
            .package(scope.package)
            .path(scope.path)
            .name(enm.name.to_owned())
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
            .name(enm.name)
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

        msg.fields.sort_by(|l, r| l.index.cmp(&r.index));

        let d = DescriptorBuilder::default()
            .package(scope.package.clone())
            .path(scope.path.clone())
            .name(msg.name.to_owned())
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
            .name(msg.name)
            .fields(msg.fields.into_iter().map(Field::from).collect())
            .build()
            .unwrap();

        let mut scope = scope.clone();
        scope.path.push(msg.name.to_owned());

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
/// For each import root, the function checks if `root/<depenency path>` exists
/// as a file. The first match is returned as a canonicalized path. If no match
/// is found in any root, an error is returned.
fn resolve_include_path<'a>(
    path: &str,
    import_roots: &[PathBuf],
    span: Span,
) -> Result<PathBuf, ParseError<'a>> {
    let path = PathBuf::from(path);

    let (path, path_root) = import_roots
        .iter()
        .find_map(|root| {
            let candidate = root.join(path);

            if candidate.is_file() {
                Some((candidate, root))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            Rich::custom(
                span,
                format!(
                    "include path '{}' not found in any import root: {:?}",
                    path.display(),
                    import_roots
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                ),
            )
        })?;

    let path = path.canonicalize().map_err(|e| {
        Rich::custom(
            span,
            format!("failed to resolve '{}': {}", path.display(), e),
        )
    })?;

    // Ensure the canonical path is still within the import root. This is a
    // defense against symlinks or other filesystem tricks.
    let path_root = path_root.canonicalize().map_err(|e| {
        Rich::custom(
            span,
            format!(
                "failed to canonicalize import root '{}': {}",
                path_root.display(),
                e
            ),
        )
    })?;

    if !path.starts_with(&path_root) {
        return Err(Rich::custom(
            span,
            format!(
                "resolved path '{}' escapes import root '{}'",
                path.display(),
                path_root.display(),
            ),
        ));
    }

    Ok(path)
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

        let import_roots = vec![root.path().canonicalize().unwrap()];
        let span = Span::from(0..10);

        // When: The include path is resolved.
        let result = resolve_include_path(Path::new("dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the file.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_searches_roots_in_order() {
        // Given: Two temp directories, with the file only in the second.
        let root1 = TempDir::new().unwrap();
        let root2 = TempDir::new().unwrap();
        let file_path = root2.path().join("dep.baproto");
        fs::write(&file_path, "").unwrap();

        let import_roots = vec![
            root1.path().canonicalize().unwrap(),
            root2.path().canonicalize().unwrap(),
        ];
        let span = Span::from(0..10);

        // When: The include path is resolved.
        let result = resolve_include_path(Path::new("dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the file in the second root.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file_path.canonicalize().unwrap());
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
            root1.path().canonicalize().unwrap(),
            root2.path().canonicalize().unwrap(),
        ];
        let span = Span::from(0..10);

        // When: The include path is resolved.
        let result = resolve_include_path(Path::new("dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the file in the first root.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file1.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_nested_path() {
        // Given: A temp directory with a nested .baproto file.
        let root = TempDir::new().unwrap();
        let nested_dir = root.path().join("sub").join("dir");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("dep.baproto");
        fs::write(&file_path, "").unwrap();

        let import_roots = vec![root.path().canonicalize().unwrap()];
        let span = Span::from(0..10);

        // When: The include path with subdirectories is resolved.
        let result = resolve_include_path(Path::new("sub/dir/dep.baproto"), &import_roots, span);

        // Then: The resolved path points to the nested file.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_include_path_not_found_returns_error() {
        // Given: A temp directory without the requested file.
        let root = TempDir::new().unwrap();
        let import_roots = vec![root.path().canonicalize().unwrap()];
        let span = Span::from(0..10);

        // When: A non-existent include path is resolved.
        let result = resolve_include_path(Path::new("missing.baproto"), &import_roots, span);

        // Then: An error is returned.
        assert!(result.is_err());
    }
}
