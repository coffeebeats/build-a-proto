mod common;

use assert_cmd::cargo::cargo_bin_cmd;

/* -------------------------------------------------------------------------- */
/*                              Success Test Cases                            */
/* -------------------------------------------------------------------------- */

#[test]
fn test_compile_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with an empty message
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("empty_message.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/empty.rs");
    common::golden::assert_golden(&content, "tests/golden/empty_message.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/empty_message.rs");

    Ok(())
}

#[test]
fn test_compile_simple_types() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with all scalar types
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("simple_types.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/types.rs");
    common::golden::assert_golden(&content, "tests/golden/simple_types.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/simple_types.rs");

    Ok(())
}

#[test]
fn test_compile_nested_messages() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with nested message definitions
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("nested_messages.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/nesting.rs");
    common::golden::assert_golden(&content, "tests/golden/nested_messages.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/nested_messages.rs");

    Ok(())
}

#[test]
fn test_compile_enums() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with enum definitions
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("enums.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/status.rs");
    common::golden::assert_golden(&content, "tests/golden/enums.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/enums.rs");

    Ok(())
}

#[test]
fn test_compile_arrays_and_maps() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with array and map types
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("arrays_and_maps.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/collections.rs");
    common::golden::assert_golden(&content, "tests/golden/arrays_and_maps.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/arrays_and_maps.rs");

    Ok(())
}

#[test]
fn test_compile_cross_file_imports() -> Result<(), Box<dyn std::error::Error>> {
    // Given: Multiple schemas with cross-file references
    let ctx = common::TestContext::new();
    let base = ctx.copy_testdata_preserve("imports/base.baproto");
    let dependent = ctx.copy_testdata_preserve("imports/dependent.baproto");

    // When: Compiling both files via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&base)
        .arg(&dependent)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/multi.rs");
    common::golden::assert_golden(&content, "tests/golden/cross_file_imports.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/cross_file_imports.rs");

    Ok(())
}

#[test]
fn test_compile_encodings() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with custom encodings
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("encodings.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/encoded.rs");
    common::golden::assert_golden(&content, "tests/golden/encodings.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/encodings.rs");

    Ok(())
}

#[test]
fn test_compile_doc_comments() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with documentation comments
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("doc_comments.baproto");

    // When: Compiling via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/docs.rs");
    common::golden::assert_golden(&content, "tests/golden/doc_comments.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/doc_comments.rs");

    Ok(())
}

#[test]
fn test_compile_multiple_files_same_package() -> Result<(), Box<dyn std::error::Error>> {
    // Given: Two schemas with the same package
    let ctx = common::TestContext::new();
    let schema1 = ctx.create_schema(
        "first.baproto",
        r#"
package test.merge;

message First {
    0: u32 id;
}
"#,
    );
    let schema2 = ctx.create_schema(
        "second.baproto",
        r#"
package test.merge;

message Second {
    0: string name;
}
"#,
    );

    // When: Compiling both via CLI
    cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema1)
        .arg(&schema2)
        .assert()
        .success();

    // Then: Generated code matches golden file
    let content = ctx.read_generated("test/merge.rs");
    common::golden::assert_golden(&content, "tests/golden/multiple_files_same_package.rs");

    // Then: The generated file has valid Rust syntax.
    common::golden::check_rust_syntax("tests/golden/multiple_files_same_package.rs");

    Ok(())
}

/* -------------------------------------------------------------------------- */
/*                               Error Test Cases                             */
/* -------------------------------------------------------------------------- */

#[test]
fn test_error_duplicate_field_indices() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema with duplicate field indices
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("duplicate_indices.baproto");

    // When: Compiling via CLI (expecting failure)
    let assert = cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .failure();

    // Then: Error output matches golden file
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let normalized = common::golden::normalize_paths(&stderr, ctx.input_path());
    common::golden::assert_golden(&normalized, "tests/golden/duplicate_field_indices.log");

    Ok(())
}

#[test]
fn test_error_invalid_type_reference() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema referencing a nonexistent type
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("invalid_type_ref.baproto");

    // When: Compiling via CLI (expecting failure)
    let assert = cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .failure();

    // Then: Error output matches golden file
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let normalized = common::golden::normalize_paths(&stderr, ctx.input_path());
    common::golden::assert_golden(&normalized, "tests/golden/invalid_type_reference.log");

    Ok(())
}

#[test]
fn test_error_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A nonexistent file path
    let ctx = common::TestContext::new();
    let bad_path = ctx.input_path().join("nonexistent.baproto");

    // When: Compiling via CLI (expecting failure)
    let assert = cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&bad_path)
        .assert()
        .failure();

    // Then: Error output matches golden file
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let normalized = common::golden::normalize_paths(&stderr, ctx.input_path());
    common::golden::assert_golden(&normalized, "tests/golden/file_not_found.log");

    Ok(())
}

#[test]
fn test_error_invalid_extension() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A file without .baproto extension
    let ctx = common::TestContext::new();
    let bad_file = ctx.input_path().join("schema.txt");
    std::fs::write(&bad_file, "package test;").unwrap();

    // When: Compiling via CLI (expecting failure)
    let assert = cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&bad_file)
        .assert()
        .failure();

    // Then: Error output matches golden file
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let normalized = common::golden::normalize_paths(&stderr, ctx.input_path());
    common::golden::assert_golden(&normalized, "tests/golden/invalid_extension.log");

    Ok(())
}

#[test]
fn test_error_missing_import_file() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A schema that includes a missing file
    let ctx = common::TestContext::new();
    let schema = ctx.create_schema(
        "test.baproto",
        r#"
package test;

include "missing.baproto";

message Foo {
    0: u32 id;
}
"#,
    );

    // When: Compiling via CLI (expecting failure)
    let assert = cargo_bin_cmd!("baproto")
        .arg("compile")
        .arg("--rust")
        .arg("-o")
        .arg(ctx.output_path())
        .arg("-I")
        .arg(ctx.input_path())
        .arg(&schema)
        .assert()
        .failure();

    // Then: Error output matches golden file
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let normalized = common::golden::normalize_paths(&stderr, ctx.input_path());
    common::golden::assert_golden(&normalized, "tests/golden/missing_import_file.log");

    Ok(())
}
