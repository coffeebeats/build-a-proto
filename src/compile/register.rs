use chumsky::error::Rich;
use std::collections::HashMap;
use std::path::Path;

use crate::ast;
use crate::ast::Item;
use crate::ast::SourceFile;
use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::ImportRoot;
use crate::core::PackageName;
use crate::core::SchemaImport;
use crate::lex::Span;
use crate::parse::ParseError;

use super::symbol::ModuleMetadata;
use super::symbol::Symbols;
use super::symbol::TypeKind;

/* -------------------------------------------------------------------------- */
/*                               Struct: Context                              */
/* -------------------------------------------------------------------------- */

/// `Context` contains compilation context that preserves parsed AST
/// with spans for semantic validation while tracking type definitions in a
/// symbol table.
#[allow(unused)]
#[derive(Default)]
pub struct Context {
    /// `symbols` is a symbol table tracking all type definitions.
    pub symbols: Symbols,
    /// `source_files` are parsed ASTs per schema, preserving spans for
    /// later validation.
    pub source_files: HashMap<SchemaImport, SourceFile>,
}

/* -------------------------------------------------------------------------- */
/*                                Fn: register                                */
/* -------------------------------------------------------------------------- */

/// `register` registers types from a parsed AST into the compilation context.
///
/// This function:
/// 1. Extracts package name, includes, and type definitions from the AST
/// 2. Registers all type names into the symbol table ([`Symbols`])
/// 3. Stores the original parsed AST for later validation
#[allow(unused)]
pub fn register<'src>(
    ctx: &mut Context,
    import_roots: &[ImportRoot],
    schema_import: &SchemaImport,
    ast: SourceFile,
) -> Result<(), ParseError<'src>> {
    let pkg = ast.package.name.clone();

    let mut deps = Vec::new();
    let mut type_descriptors = Vec::new();

    for include in &ast.includes {
        deps.push(resolve_include_path(
            &include.path,
            import_roots,
            include.span,
        )?);
    }

    for item in &ast.items {
        match item {
            Item::Enum(enm) => {
                let descriptor = build_descriptor(&pkg, &[], &enm.name.name);
                ctx.symbols
                    .insert_type(descriptor.clone(), TypeKind::Variant);
                type_descriptors.push(descriptor);
            }
            Item::Message(msg) => {
                let descriptor = build_descriptor(&pkg, &[], &msg.name.name);
                ctx.symbols
                    .insert_type(descriptor.clone(), TypeKind::Message);
                type_descriptors.push(descriptor.clone());

                register_nested_types(&mut ctx.symbols, &pkg, &[&msg.name.name], msg);
            }
        }
    }

    let metadata = ModuleMetadata {
        package: pkg,
        deps,
        types: type_descriptors,
    };

    ctx.symbols.insert_module(schema_import.clone(), metadata);
    ctx.source_files.insert(schema_import.clone(), ast);

    Ok(())
}

/* ----------------------- Fn: register_nested_types ------------------------ */

/// Recursively registers nested messages and enums within a message.
fn register_nested_types(
    symbols: &mut Symbols,
    package: &PackageName,
    path: &[&str],
    msg: &ast::Message,
) {
    for enm in &msg.nested_enums {
        let descriptor = build_descriptor(package, path, &enm.name.name);
        symbols.insert_type(descriptor, TypeKind::Variant);
    }

    for nested_msg in &msg.nested_messages {
        let mut nested_path: Vec<&str> = path.to_vec();
        nested_path.push(&nested_msg.name.name);
        let descriptor = build_descriptor(package, path, &nested_msg.name.name);
        symbols.insert_type(descriptor, TypeKind::Message);

        register_nested_types(symbols, package, &nested_path, nested_msg);
    }
}

/* ------------------------- Fn: build_descriptor --------------------------- */

/// Builds a Descriptor from package, path, and name components.
fn build_descriptor(package: &PackageName, path: &[&str], name: &str) -> Descriptor {
    DescriptorBuilder::default()
        .package(package.clone())
        .path(path.iter().map(|s| s.to_string()).collect())
        .name(name.to_owned())
        .build()
        .expect("descriptor should be valid")
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

    // TODO: Add tests for register() function (Step 8.4 and 8.5)
    // - test_register_populates_symbols
    // - test_register_nested_message_types
    // - test_register_nested_enum_types
    // - test_register_stores_expressions
    // - test_register_extracts_package_name
    // - test_register_resolves_includes
}
