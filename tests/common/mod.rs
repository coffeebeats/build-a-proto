pub mod golden;

use std::path::{Path, PathBuf};
use tempfile::TempDir;

/* -------------------------------------------------------------------------- */
/*                            Struct: TestContext                             */
/* -------------------------------------------------------------------------- */

/// TestContext manages temporary directories and provides helpers
/// for e2e compilation testing.
pub struct TestContext {
    /// Temporary directory for input schemas (cleaned up on drop)
    pub input_dir: TempDir,
    /// Temporary directory for generated output (cleaned up on drop)
    pub output_dir: TempDir,
}

impl TestContext {
    /// Creates a new test context with fresh temporary directories.
    pub fn new() -> Self {
        Self {
            input_dir: TempDir::new().expect("failed to create temp input dir"),
            output_dir: TempDir::new().expect("failed to create temp output dir"),
        }
    }

    /// Returns the input directory path.
    pub fn input_path(&self) -> &Path {
        self.input_dir.path()
    }

    /// Returns the output directory path.
    pub fn output_path(&self) -> &Path {
        self.output_dir.path()
    }

    /// Copies a testdata file into the input directory.
    /// Returns the absolute path to the copied file.
    pub fn copy_testdata(&self, relative_path: &str) -> PathBuf {
        let src = testdata_path(relative_path);
        let filename = Path::new(relative_path)
            .file_name()
            .expect("invalid filename");
        let dest = self.input_dir.path().join(filename);

        std::fs::copy(&src, &dest)
            .unwrap_or_else(|_| panic!("failed to copy {} to {}", src.display(), dest.display()));

        dest
    }

    /// Copies a testdata file, preserving the subdirectory structure.
    /// Returns the absolute path to the copied file.
    ///
    /// Example: copy_testdata_preserve("imports/base.baproto") creates
    /// temp_dir/imports/base.baproto
    pub fn copy_testdata_preserve(&self, relative_path: &str) -> PathBuf {
        let src = testdata_path(relative_path);
        let dest = self.input_dir.path().join(relative_path);

        // Create parent directory if needed
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|_| panic!("failed to create directory {}", parent.display()));
        }

        std::fs::copy(&src, &dest)
            .unwrap_or_else(|_| panic!("failed to copy {} to {}", src.display(), dest.display()));

        dest
    }

    /// Creates a schema file in the input directory with the given content.
    /// Returns the absolute path to the created file.
    pub fn create_schema(&self, name: &str, content: &str) -> PathBuf {
        let path = self.input_dir.path().join(name);
        std::fs::write(&path, content)
            .unwrap_or_else(|_| panic!("failed to write schema to {}", path.display()));
        path
    }

    /// Reads the contents of a generated file by relative path.
    pub fn read_generated(&self, relative_path: &str) -> String {
        let path = self.output_dir.path().join(relative_path);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("failed to read {}", path.display()))
    }
}

/* -------------------------------------------------------------------------- */
/*                          Function: testdata_path                           */
/* -------------------------------------------------------------------------- */

/// Returns the absolute path to a testdata file.
pub fn testdata_path(relative_path: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("tests")
        .join("testdata")
        .join(relative_path)
}

/// Loads the content of a testdata file.
#[allow(dead_code)]
pub fn load_testdata(relative_path: &str) -> String {
    let path = testdata_path(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("failed to read testdata file: {}", path.display()))
}
