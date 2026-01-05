use std::env;
use std::fs;
use std::path::Path;

/* -------------------------------------------------------------------------- */
/*                         Function: normalize_paths                          */
/* -------------------------------------------------------------------------- */

/// Normalizes temporary directory paths in text for consistent golden file comparison.
///
/// This replaces OS-specific temporary directory paths with the placeholder `<temp>`
/// to ensure golden file comparisons work across different test runs.
pub fn normalize_paths(text: &str, temp_path: &Path) -> String {
    text.replace(&temp_path.display().to_string(), "<temp>")
}

/* -------------------------------------------------------------------------- */
/*                            Function: assert_golden                         */
/* -------------------------------------------------------------------------- */

/// `assert_golden` compares actual output against a golden file.
///
/// If the `BAPROTO_UPDATE_GOLDENS` environment variable is set to "1", this
/// function will update the golden file with the actual content instead of
/// comparing. This is useful when regenerating golden files after intentional
/// changes to the output.
///
/// # Arguments
///
/// * `actual` - The actual output to compare
/// * `golden_path` - Path to the golden file
///
/// # Panics
///
/// Panics if:
/// * The golden file doesn't exist (suggests running with `BAPROTO_UPDATE_GOLDENS=1`)
/// * The actual content doesn't match the golden content
/// * File I/O operations fail
pub fn assert_golden(actual: &str, golden_path: impl AsRef<Path>) {
    let golden_path = golden_path.as_ref();

    if should_update_goldens() {
        write_golden(golden_path, actual);
        eprintln!("Updated golden file: {}", golden_path.display());
        return;
    }

    match read_golden(golden_path) {
        Ok(expected) => {
            if actual != expected {
                panic!(
                    "\nGolden file mismatch: {}\n\
                    \n\
                    Expected:\n\
                    {}\n\
                    \n\
                    Actual:\n\
                    {}\n\
                    \n\
                    To update golden files, run:\n\
                    BAPROTO_UPDATE_GOLDENS=1 cargo test\n",
                    golden_path.display(),
                    expected,
                    actual
                );
            }
        }
        Err(e) => {
            panic!(
                "\nFailed to read golden file: {}\n\
                Error: {}\n\
                \n\
                Golden file may not exist. To create it, run:\n\
                BAPROTO_UPDATE_GOLDENS=1 cargo test\n",
                golden_path.display(),
                e
            );
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                         Function: should_update_goldens                    */
/* -------------------------------------------------------------------------- */

/// Checks if golden files should be updated based on the `BAPROTO_UPDATE_GOLDENS`
/// environment variable.
fn should_update_goldens() -> bool {
    env::var("BAPROTO_UPDATE_GOLDENS")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/* -------------------------------------------------------------------------- */
/*                            Function: read_golden                           */
/* -------------------------------------------------------------------------- */

/// Reads a golden file and returns its contents.
fn read_golden(path: &Path) -> std::io::Result<String> {
    fs::read_to_string(path)
}

/* -------------------------------------------------------------------------- */
/*                            Function: write_golden                          */
/* -------------------------------------------------------------------------- */

/// Writes content to a golden file, creating parent directories as needed.
fn write_golden(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create golden file directory");
    }

    fs::write(path, content).expect("Failed to write golden file");
}

/* -------------------------------------------------------------------------- */
/*                         Function: check_rust_syntax                        */
/* -------------------------------------------------------------------------- */

/// Validates that a golden Rust file has valid syntax.
///
/// This parses the file using `syn` to ensure the generated code is syntactically
/// valid Rust. This catches any codegen bugs that might produce invalid Rust.
///
/// # Arguments
///
/// * `golden_path` - Path to the golden .rs file to validate
///
/// # Panics
///
/// Panics if:
/// * The file cannot be read
/// * The file contains invalid Rust syntax
pub fn check_rust_syntax(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read golden file {}: {}", path.display(), e));

    syn::parse_file(&content).unwrap_or_else(|e| {
        panic!(
            "Golden file {} contains invalid Rust syntax: {}",
            path.display(),
            e
        )
    });
}
