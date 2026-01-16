pub mod golden;

use std::path::{Path, PathBuf};
use tempfile::TempDir;

/* -------------------------------------------------------------------------- */
/*                            Struct: TestContext                             */
/* -------------------------------------------------------------------------- */

/// `TestContext` manages temporary directories and provides helpers for end-to-
/// end compilation testing.
pub struct TestContext {
    /// `input_dir` is a temporary directory for input schemas (cleaned up on
    /// drop).
    pub input_dir: TempDir,
    /// `output_dir` is a temporary directory for generated output (cleaned up
    /// on drop).
    pub output_dir: TempDir,
}

/* ---------------------------- Impl: TestContext --------------------------- */

impl TestContext {
    /// `new` creates a new [`TestContext`], instantiating temporary directories
    /// used during integration testing.
    pub fn new() -> Self {
        Self {
            input_dir: TempDir::new().expect("failed to create temp directory"),
            output_dir: TempDir::new().expect("failed to create temp directory"),
        }
    }

    /// `input_path` returns the input directory path.
    pub fn input_path(&self) -> &Path {
        self.input_dir.path()
    }

    /// `output_path` returns the output directory path.
    pub fn output_path(&self) -> &Path {
        self.output_dir.path()
    }

    /// `copy_testdata` copies a `tests/testdata/` file or directory into the
    /// input directory, preserving any intermediate directories. Returns the
    /// absolute path to the copied file.
    ///
    /// ### Example
    ///
    /// ```rust
    /// copy_testdata_preserve("foo.baproto") // Creates '<temp>/foo.baproto'.
    /// copy_testdata_preserve("foo/bar.baproto") // Creates '<temp>/foo/bar.baproto'.
    /// ```
    pub fn copy_testdata(&self, path: &str) -> PathBuf {
        let src = testdata_path(path);
        let dest = self.input_dir.path().join(path);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|_| panic!("failed to create directory {}", parent.display()));
        }

        std::fs::copy(&src, &dest)
            .unwrap_or_else(|_| panic!("failed to copy {} to {}", src.display(), dest.display()));

        dest
    }

    /// `create_schema` creates a schema file in the input directory with the
    /// given contents. Returns the absolute path to the created file.
    pub fn create_schema(&self, name: &str, content: &str) -> PathBuf {
        let path = self.input_dir.path().join(name);
        std::fs::write(&path, content)
            .unwrap_or_else(|_| panic!("failed to write schema to {}", path.display()));
        path
    }

    /// `read_generated` reads the contents of a generated file by relative path.
    pub fn read_generated(&self, path: &str) -> String {
        let path = self.output_dir.path().join(path);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("failed to read {}", path.display()))
    }
}

/* -------------------------------------------------------------------------- */
/*                              Fn: testdata_path                             */
/* -------------------------------------------------------------------------- */

/// `testdata_path` returns the absolute path to a testdata file.
pub fn testdata_path(relative_path: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    PathBuf::from(manifest_dir)
        .join("tests")
        .join("testdata")
        .join(relative_path)
}
