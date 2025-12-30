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

use crate::ast;
use crate::ast::Item;
use crate::ast::SourceFile;
use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::EnumBuilder;
use crate::core::ImportRoot;
use crate::core::MessageBuilder;
use crate::core::Module;
use crate::core::Registry;
use crate::core::SchemaImport;
use crate::core::registry;
use crate::lex::Span;
use crate::parse::ParseError;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Prepare                                */
/* -------------------------------------------------------------------------- */

pub fn prepare<'a>(
    schema_import: &'a SchemaImport,
    import_roots: &[ImportRoot],
    registry: &'a mut Registry,
    ast: &'a SourceFile,
) -> Result<(), ParseError<'a>> {
    let mut module = Module::new(schema_import.as_path().to_path_buf());

    let package = ast.package.name.clone();
    module.package = package.clone();

    for include in &ast.includes {
        module.deps.push(resolve_include_path(
            &include.path,
            import_roots,
            include.span,
        )?);
    }

    fn convert_field(field: &ast::Field) -> crate::core::Field {
        crate::core::FieldBuilder::default()
            .comment(
                field
                    .doc
                    .as_ref()
                    .map(|doc| doc.lines.clone())
                    .unwrap_or_default(),
            )
            .encoding(field.encoding.as_ref().map(|enc| {
                enc.encodings
                    .iter()
                    .map(|e| match e {
                        ast::Encoding::Bits(n) => crate::core::Encoding::Bits(*n),
                        ast::Encoding::BitsVariable(n) => crate::core::Encoding::BitsVariable(*n),
                        ast::Encoding::Delta => crate::core::Encoding::Delta,
                        ast::Encoding::FixedPoint(i, f) => {
                            crate::core::Encoding::FixedPoint(*i, *f)
                        }
                        ast::Encoding::Pad(n) => crate::core::Encoding::Pad(*n),
                        ast::Encoding::ZigZag => crate::core::Encoding::ZigZag,
                    })
                    .collect()
            }))
            .name(field.name.name.clone())
            .index(field.index.value)
            .typ(convert_type(&field.typ))
            .build()
            .unwrap()
    }

    fn convert_type(typ: &ast::Type) -> crate::core::Type {
        match &typ.kind {
            ast::TypeKind::Invalid => panic!("Invalid type in AST"),
            ast::TypeKind::Scalar(s) => crate::core::Type::Scalar(convert_scalar(s)),
            ast::TypeKind::Reference(r) => crate::core::Type::Reference(r.clone()),
            ast::TypeKind::Array { element, size } => {
                crate::core::Type::Array(Box::new(convert_type(element)), size.map(|s| s as usize))
            }
            ast::TypeKind::Map { key, value } => {
                crate::core::Type::Map(Box::new(convert_type(key)), Box::new(convert_type(value)))
            }
        }
    }

    fn convert_scalar(s: &ast::ScalarType) -> crate::core::Scalar {
        match s {
            ast::ScalarType::Bit => crate::core::Scalar::Bit,
            ast::ScalarType::Bool => crate::core::Scalar::Bool,
            ast::ScalarType::Byte => crate::core::Scalar::Byte,
            ast::ScalarType::Float32 => crate::core::Scalar::Float32,
            ast::ScalarType::Float64 => crate::core::Scalar::Float64,
            ast::ScalarType::Int8 => crate::core::Scalar::SignedInt8,
            ast::ScalarType::Int16 => crate::core::Scalar::SignedInt16,
            ast::ScalarType::Int32 => crate::core::Scalar::SignedInt32,
            ast::ScalarType::Int64 => crate::core::Scalar::SignedInt64,
            ast::ScalarType::Uint8 => crate::core::Scalar::UnsignedInt8,
            ast::ScalarType::Uint16 => crate::core::Scalar::UnsignedInt16,
            ast::ScalarType::Uint32 => crate::core::Scalar::UnsignedInt32,
            ast::ScalarType::Uint64 => crate::core::Scalar::UnsignedInt64,
            ast::ScalarType::String => crate::core::Scalar::String,
        }
    }

    fn register_enm(registry: &mut Registry, scope: Descriptor, enm: &ast::Enum) -> Descriptor {
        debug_assert!(scope.name.is_none());

        let mut variants: Vec<_> = enm.variants.iter().collect();
        variants.sort_by_key(|v| v.index.value);

        let d = DescriptorBuilder::default()
            .package(scope.package)
            .path(scope.path)
            .name(enm.name.name.clone())
            .build()
            .unwrap();

        let e = EnumBuilder::default()
            .comment(
                enm.doc
                    .as_ref()
                    .map(|doc| doc.lines.clone())
                    .unwrap_or_default(),
            )
            .name(enm.name.name.clone())
            .variants(
                variants
                    .into_iter()
                    .map(|v| match &v.kind {
                        ast::VariantKind::Field(f) => {
                            crate::core::VariantKind::Field(convert_field(f))
                        }
                        ast::VariantKind::Unit(ident) => crate::core::VariantKind::Variant(
                            crate::core::VariantBuilder::default()
                                .comment(
                                    v.doc
                                        .as_ref()
                                        .map(|doc| doc.lines.clone())
                                        .unwrap_or_default(),
                                )
                                .name(ident.name.clone())
                                .build()
                                .unwrap(),
                        ),
                    })
                    .collect(),
            )
            .build()
            .unwrap();

        registry.insert(d.clone(), registry::Kind::Enum(e));

        d
    }

    fn register_msg(registry: &mut Registry, scope: Descriptor, msg: &ast::Message) -> Descriptor {
        debug_assert!(scope.name.is_none());

        let mut fields: Vec<_> = msg.fields.iter().collect();
        fields.sort_by_key(|f| f.index.value);

        let d = DescriptorBuilder::default()
            .package(scope.package.clone())
            .path(scope.path.clone())
            .name(msg.name.name.clone())
            .build()
            .unwrap();

        let mut m = MessageBuilder::default()
            .comment(
                msg.doc
                    .as_ref()
                    .map(|doc| doc.lines.clone())
                    .unwrap_or_default(),
            )
            .name(msg.name.name.clone())
            .fields(fields.into_iter().map(convert_field).collect())
            .build()
            .unwrap();

        let mut scope = scope.clone();
        scope.path.push(msg.name.name.clone());

        m.enums = msg
            .nested_enums
            .iter()
            .map(|enm| register_enm(registry, scope.clone(), enm))
            .collect();

        m.messages = msg
            .nested_messages
            .iter()
            .map(|m| register_msg(registry, scope.clone(), m))
            .collect();

        registry.insert(d.clone(), registry::Kind::Message(m));

        d
    }

    let scope = DescriptorBuilder::default()
        .package(package.clone())
        .build()
        .unwrap();

    for item in &ast.items {
        match item {
            Item::Enum(enm) => {
                let d = register_enm(registry, scope.clone(), enm);
                module.enums.push(d);
            }
            Item::Message(msg) => {
                let d = register_msg(registry, scope.clone(), msg);
                module.messages.push(d);
            }
        }
    }

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
