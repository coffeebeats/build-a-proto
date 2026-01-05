mod common;

use baproto::cmd::compile::{Args, GeneratorSelection, handle};

/* -------------------------------------------------------------------------- */
/*                              Success Test Cases                            */
/* -------------------------------------------------------------------------- */

#[test]
fn test_compile_empty_message() {
    // Given: A schema with an empty message
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("empty_message.baproto");

    // When: Create Args and compile with Rust generator
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Output file is generated
    assert!(ctx.has_generated("test/empty.rs"));
}

#[test]
fn test_compile_simple_types() {
    // Given: A schema with all scalar types
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("simple_types.baproto");

    // When: Compiling
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Generated code contains all field types
    let content = ctx.read_generated("test/types.rs");
    common::assertions::assert_contains_all(
        &content,
        &[
            "pub flag: bool",
            "pub tiny: u8",
            "pub small: u16",
            "pub medium: u32",
            "pub large: u64",
            "pub signed_tiny: i8",
            "pub signed_small: i16",
            "pub signed_medium: i32",
            "pub signed_large: i64",
            "pub float_val: f32",
            "pub double_val: f64",
            "pub text: String",
        ],
    );
}

#[test]
fn test_compile_nested_messages() {
    // Given: A schema with nested message definitions
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("nested_messages.baproto");

    // When: Compiling
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Nested types are generated
    let content = ctx.read_generated("test/nesting.rs");
    common::assertions::assert_contains_all(
        &content,
        &[
            "pub struct Level1",
            "pub struct Level2",
            "pub struct Level3",
        ],
    );
}

#[test]
fn test_compile_enums() {
    // Given: A schema with enum definitions
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("enums.baproto");

    // When: Compiling
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Enum variants are generated
    let content = ctx.read_generated("test/status.rs");
    common::assertions::assert_contains_all(
        &content,
        &[
            "pub enum Status",
            "Unknown",
            "Active",
            "Inactive",
            "pub enum Tagged",
            "None",
            "Number",
            "Text",
        ],
    );
}

#[test]
fn test_compile_arrays_and_maps() {
    // Given: A schema with array and map types
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("arrays_and_maps.baproto");

    // When: Compiling
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Collections are mapped to Vec and HashMap
    let content = ctx.read_generated("test/collections.rs");
    common::assertions::assert_contains_all(
        &content,
        &[
            "pub numbers: Vec<u32>",
            "pub names: Vec<String>",
            "pub counts: HashMap<String, u32>",
        ],
    );
}

#[test]
fn test_compile_cross_file_imports() {
    // Given: Multiple schemas with cross-file references
    let ctx = common::TestContext::new();
    let base = ctx.copy_testdata_preserve("imports/base.baproto");
    let dependent = ctx.copy_testdata_preserve("imports/dependent.baproto");

    // When: Compiling both files
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![base, dependent],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Both types are in the same package file
    let content = ctx.read_generated("test/multi.rs");
    common::assertions::assert_contains_all(
        &content,
        &["pub struct User", "pub struct Post", "pub author: User"],
    );
}

#[test]
fn test_compile_encodings() {
    // Given: A schema with custom encodings
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("encodings.baproto");

    // When: Compiling
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Message is generated (encoding information is in IR but may not be in generated code yet)
    assert!(ctx.has_generated("test/encoded.rs"));
}

#[test]
fn test_compile_doc_comments() {
    // Given: A schema with documentation comments
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("doc_comments.baproto");

    // When: Compiling
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Doc comments are preserved (if the generator supports them)
    assert!(ctx.has_generated("test/docs.rs"));
}

#[test]
fn test_compile_multiple_files_same_package() {
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

    // When: Compiling both
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema1, schema2],
    };
    let result = handle(args);

    // Then: Compilation succeeds
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // Then: Both types are in the same output file
    let content = ctx.read_generated("test/merge.rs");
    common::assertions::assert_contains_all(&content, &["pub struct First", "pub struct Second"]);
}

/* -------------------------------------------------------------------------- */
/*                               Error Test Cases                             */
/* -------------------------------------------------------------------------- */

#[test]
fn test_error_duplicate_field_indices() {
    // Given: A schema with duplicate field indices
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("duplicate_indices.baproto");

    // When: Attempting to compile
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation fails with appropriate error
    assert!(result.is_err(), "Expected compilation to fail");
    let error = result.unwrap_err();
    let error_msg = format!("{:?}", error);
    // The diagnostic system reports errors, resulting in "Compilation failed"
    assert!(
        error_msg.contains("Compilation failed") || error_msg.contains("duplicate"),
        "Expected compilation failure due to duplicate indices, got: {}",
        error_msg
    );
}

#[test]
fn test_error_invalid_type_reference() {
    // Given: A schema referencing a nonexistent type
    let ctx = common::TestContext::new();
    let schema = ctx.copy_testdata("invalid_type_ref.baproto");

    // When: Attempting to compile
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation fails with type resolution error
    assert!(result.is_err(), "Expected compilation to fail");
    let error = result.unwrap_err();
    let error_msg = format!("{:?}", error);
    // Check for either "unresolved", "not found", or "Compilation failed" (which indicates diagnostics were reported)
    assert!(
        error_msg.contains("Compilation failed")
            || error_msg.contains("unresolved")
            || error_msg.contains("not found")
            || error_msg.contains("NonExistent"),
        "Expected type resolution error, got: {}",
        error_msg
    );
}

#[test]
fn test_error_file_not_found() {
    // Given: A nonexistent file path
    let ctx = common::TestContext::new();
    let bad_path = ctx.input_path().join("nonexistent.baproto");

    // When: Attempting to compile
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![bad_path],
    };
    let result = handle(args);

    // Then: Compilation fails with file not found error
    assert!(result.is_err(), "Expected compilation to fail");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("does not exist") || error_msg.contains("not found"),
        "Expected file not found error, got: {}",
        error_msg
    );
}

#[test]
fn test_error_invalid_extension() {
    // Given: A file without .baproto extension
    let ctx = common::TestContext::new();
    let bad_file = ctx.input_dir.path().join("schema.txt");
    std::fs::write(&bad_file, "package test;").unwrap();

    // When: Attempting to compile
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![bad_file],
    };
    let result = handle(args);

    // Then: Compilation fails with extension error
    assert!(result.is_err(), "Expected compilation to fail");
    common::assertions::assert_error_contains(&result.unwrap_err(), ".baproto");
}

#[test]
fn test_error_missing_import_file() {
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

    // When: Attempting to compile
    let args = Args {
        generator: GeneratorSelection {
            rust: true,
            plugin: None,
        },
        out: Some(ctx.output_path().to_path_buf()),
        import_roots: vec![ctx.input_path().to_path_buf()],
        files: vec![schema],
    };
    let result = handle(args);

    // Then: Compilation fails with import resolution error
    assert!(result.is_err(), "Expected compilation to fail");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("import")
            || error_msg.contains("missing.baproto")
            || error_msg.contains("not found")
            || error_msg.contains("Failed to read source file"),
        "Expected import resolution error, got: {}",
        error_msg
    );
}
