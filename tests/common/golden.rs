use std::env;
use std::fs;
use std::path::Path;

/* -------------------------------------------------------------------------- */
/*                        Fn: assert_valid_rust_syntax                        */
/* -------------------------------------------------------------------------- */

/// `assert_valid_rust_syntax` validates that a golden Rust file has valid
/// syntax.
///
/// This parses the file using `syn` to ensure the generated code is
/// syntactically valid Rust. This catches any codegen bugs that might produce
/// invalid Rust.
pub fn assert_valid_rust_syntax(path: impl AsRef<Path>) {
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

/* -------------------------------------------------------------------------- */
/*                              Fn: assert_golden                             */
/* -------------------------------------------------------------------------- */

/// `assert_golden` compares actual output against a golden file.
///
/// If the `BAPROTO_UPDATE_GOLDENS` environment variable is enabled (see
/// [`should_update_goldens`] for valid "truthy" values), this function will
/// update the golden file with the actual content instead of comparing. This is
/// useful when regenerating golden files after intentional changes to the
/// output.
pub fn assert_golden<T: AsRef<Path>>(actual: &str, golden_path: T) {
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
/*                          Fn: should_update_goldens                         */
/* -------------------------------------------------------------------------- */

/// Checks if golden files should be updated based on the
/// `BAPROTO_UPDATE_GOLDENS` environment variable.
fn should_update_goldens() -> bool {
    env::var("BAPROTO_UPDATE_GOLDENS")
        .map(|v| {
            let v = v.to_ascii_lowercase();
            vec!["1", "y", "yes", "true"].contains(&v.as_str())
        })
        .unwrap_or(false)
}

/* -------------------------------------------------------------------------- */
/*                               Fn: read_golden                              */
/* -------------------------------------------------------------------------- */

/// `read_golden` reads a golden file and returns its contents.
fn read_golden<T: AsRef<Path>>(path: T) -> std::io::Result<String> {
    fs::read_to_string(path)
}

/* -------------------------------------------------------------------------- */
/*                              Fn: write_golden                              */
/* -------------------------------------------------------------------------- */

/// `write_golden` writes content to a golden file, creating parent directories
/// as needed.
fn write_golden<T: AsRef<Path>>(path: T, content: &str) {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent).expect("Failed to create golden file directory");
    }

    fs::write(path, content).expect("Failed to write golden file");
}

/* -------------------------------------------------------------------------- */
/*                             Fn: normalize_paths                            */
/* -------------------------------------------------------------------------- */

/// `normalize_paths` normalizes temporary directory paths in text for
/// consistent golden file comparison.
///
/// This replaces OS-specific temporary directory paths with the placeholder
/// `<temp>` to ensure golden file comparisons work across different test runs.
pub fn normalize_paths<T: AsRef<Path>>(text: &str, path: T) -> String {
    text.replace(&path.as_ref().display().to_string(), "<temp>")
}
