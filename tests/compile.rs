use assert_cmd::cargo::cargo_bin_cmd;

/* ------------------------------- Mod: Common ------------------------------ */

mod common;
use common::golden;

/* -------------------------------------------------------------------------- */
/*                               Tests: compile                               */
/* -------------------------------------------------------------------------- */

#[test]
fn test_compile_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with an empty message
    let schema = ctx.copy_testdata("empty_message.baproto");

    // When: Compiling via CLI.
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

    // Then: Generated code matches the golden file.
    let content = ctx.read_generated("test/empty.rs");
    golden::assert_golden(&content, "tests/testdata/golden/empty_message.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/empty_message.rs");

    Ok(())
}

#[test]
fn test_compile_simple_types() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with all scalar types
    let schema = ctx.copy_testdata("simple_types.baproto");

    // When: Compiling via CLI.
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
    golden::assert_golden(&content, "tests/testdata/golden/simple_types.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/simple_types.rs");

    Ok(())
}

#[test]
fn test_compile_nested_messages() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with nested message definitions
    let schema = ctx.copy_testdata("nested_messages.baproto");

    // When: Compiling via CLI.
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
    golden::assert_golden(&content, "tests/testdata/golden/nested_messages.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/nested_messages.rs");

    Ok(())
}

#[test]
fn test_compile_enums() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with enum definitions
    let schema = ctx.copy_testdata("enums.baproto");

    // When: Compiling via CLI.
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
    golden::assert_golden(&content, "tests/testdata/golden/enums.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/enums.rs");

    Ok(())
}

#[test]
fn test_compile_collections() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with array and map types
    let schema = ctx.copy_testdata("collections.baproto");

    // When: Compiling via CLI.
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
    golden::assert_golden(&content, "tests/testdata/golden/collections.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/collections.rs");

    Ok(())
}

#[test]
fn test_compile_cross_file_imports() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: Multiple schemas with cross-file references
    let base = ctx.copy_testdata("imports_base.baproto");
    let dependent = ctx.copy_testdata("imports_dependent.baproto");

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
    golden::assert_golden(&content, "tests/testdata/golden/cross_file_imports.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/cross_file_imports.rs");

    Ok(())
}

#[test]
fn test_compile_encodings() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with custom encodings
    let schema = ctx.copy_testdata("encodings.baproto");

    // When: Compiling via CLI.
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
    golden::assert_golden(&content, "tests/testdata/golden/encodings.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/encodings.rs");

    Ok(())
}

#[test]
fn test_compile_doc_comments() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with documentation comments
    let schema = ctx.copy_testdata("doc_comments.baproto");

    // When: Compiling via CLI.
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
    golden::assert_golden(&content, "tests/testdata/golden/doc_comments.rs");

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/doc_comments.rs");

    Ok(())
}

#[test]
fn test_compile_multiple_files_same_package() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: Two schemas with the same package.
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
    golden::assert_golden(
        &content,
        "tests/testdata/golden/multiple_files_same_package.rs",
    );

    // Then: The generated file has valid Rust syntax.
    golden::assert_valid_rust_syntax("tests/testdata/golden/multiple_files_same_package.rs");

    Ok(())
}

/* -------------------------------------------------------------------------- */
/*                               Error Test Cases                             */
/* -------------------------------------------------------------------------- */

#[test]
fn test_error_duplicate_field_indices() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema with duplicate field indices
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

    let output = assert.get_output();
    let output = String::from_utf8_lossy(&output.stderr).to_string();
    let output = golden::normalize_paths(&output, ctx.input_path());

    // Then: The generated file matches expectations.
    golden::assert_golden(&output, "tests/testdata/golden/duplicate_field_indices.log");

    Ok(())
}

#[test]
fn test_error_invalid_type_reference() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema referencing a nonexistent type
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

    let output = assert.get_output();
    let output = String::from_utf8_lossy(&output.stderr).to_string();
    let output = golden::normalize_paths(&output, ctx.input_path());

    // Then: Error output matches golden file
    golden::assert_golden(&output, "tests/testdata/golden/invalid_type_reference.log");

    Ok(())
}

#[test]
fn test_error_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A nonexistent file path
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

    let output = assert.get_output();
    let output = String::from_utf8_lossy(&output.stderr).to_string();
    let output = golden::normalize_paths(&output, ctx.input_path());

    // Then: Error output matches golden file
    golden::assert_golden(&output, "tests/testdata/golden/file_not_found.log");

    Ok(())
}

#[test]
fn test_error_invalid_extension() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A file without .baproto extension
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

    let output = assert.get_output();
    let output = String::from_utf8_lossy(&output.stderr).to_string();
    let output = golden::normalize_paths(&output, ctx.input_path());

    // Then: Error output matches golden file
    golden::assert_golden(&output, "tests/testdata/golden/invalid_extension.log");

    Ok(())
}

#[test]
fn test_error_missing_import_file() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = common::TestContext::new();

    // Given: A schema that includes a missing file
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

    let output = assert.get_output();
    let output = String::from_utf8_lossy(&output.stderr).to_string();
    let output = golden::normalize_paths(&output, ctx.input_path());

    // Then: Error output matches golden file
    golden::assert_golden(&output, "tests/testdata/golden/missing_import_file.log");

    Ok(())
}
