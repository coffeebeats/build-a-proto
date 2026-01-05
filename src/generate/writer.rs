//! File writer utility for generator output.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::GeneratorOutput;

/* -------------------------------------------------------------------------- */
/*                              Struct: FileWriter                            */
/* -------------------------------------------------------------------------- */

/// Writes generator output to disk.
///
/// Handles creating parent directories and writing file contents
/// to the specified output directory.
pub struct FileWriter {
    out_dir: PathBuf,
}

impl FileWriter {
    /// Creates a new file writer targeting the given output directory.
    pub fn new(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            out_dir: out_dir.into(),
        }
    }

    /// Writes all files from the generator output to disk.
    ///
    /// Creates parent directories as needed. Returns the list of
    /// absolute paths that were written.
    pub fn write(&self, output: &GeneratorOutput) -> io::Result<Vec<PathBuf>> {
        let mut written = Vec::with_capacity(output.files.len());

        for (relative_path, content) in &output.files {
            let absolute_path = self.out_dir.join(relative_path);

            // Create parent directories if they don't exist
            if let Some(parent) = absolute_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&absolute_path, content)?;
            written.push(absolute_path);
        }

        Ok(written)
    }

    /// Returns the output directory.
    pub fn out_dir(&self) -> &Path {
        &self.out_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let writer = FileWriter::new(temp_dir.path());

        let mut output = GeneratorOutput::new();
        output.add("test.txt", "hello world");

        let written = writer.write(&output).unwrap();

        assert_eq!(written.len(), 1);
        assert!(written[0].exists());
        assert_eq!(fs::read_to_string(&written[0]).unwrap(), "hello world");
    }

    #[test]
    fn test_write_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let writer = FileWriter::new(temp_dir.path());

        let mut output = GeneratorOutput::new();
        output.add("foo/bar/baz.txt", "nested content");

        let written = writer.write(&output).unwrap();

        assert_eq!(written.len(), 1);
        assert!(written[0].exists());
        assert_eq!(
            fs::read_to_string(&written[0]).unwrap(),
            "nested content"
        );
    }

    #[test]
    fn test_write_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let writer = FileWriter::new(temp_dir.path());

        let mut output = GeneratorOutput::new();
        output.add("a.txt", "content a");
        output.add("b/c.txt", "content c");

        let written = writer.write(&output).unwrap();

        assert_eq!(written.len(), 2);
        for path in &written {
            assert!(path.exists());
        }
    }
}