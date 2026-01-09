//! File writer utility for generator output.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::api::GeneratorOutput;

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
