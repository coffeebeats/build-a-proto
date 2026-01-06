//! External generator wrapper for binary plugins.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::ir;

use super::{Generator, GeneratorError, GeneratorOutput};

/* -------------------------------------------------------------------------- */
/*                           Struct: ExternalGenerator                        */
/* -------------------------------------------------------------------------- */

/// Wrapper that invokes an external binary as a code generator.
///
/// ## Protocol
///
/// External generators follow a simple stdin/stdout JSON protocol:
///
/// **Input (stdin):** JSON-serialized `ir::Schema`
///
/// **Output (stdout):** JSON object with "files" key:
/// ```json
/// {"files": {"path/to/file.ext": "file contents..."}}
/// ```
///
/// **Exit codes:**
/// - `0` = success
/// - non-zero = failure (stderr contains error message)
pub struct ExternalGenerator {
    binary_path: PathBuf,
    name: String,
}

impl ExternalGenerator {
    /// Creates a new external generator wrapper.
    ///
    /// The binary path must exist and be executable.
    pub fn new(binary_path: impl Into<PathBuf>) -> Result<Self, GeneratorError> {
        let binary_path = binary_path.into();

        if !binary_path.exists() {
            return Err(GeneratorError::Process(format!(
                "generator binary not found: {}",
                binary_path.display()
            )));
        }

        // Derive name from binary filename
        let name = binary_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("external")
            .to_string();

        Ok(Self { binary_path, name })
    }

    /// Returns the path to the generator binary.
    pub fn binary_path(&self) -> &Path {
        &self.binary_path
    }
}

impl Generator for ExternalGenerator {
    fn name(&self) -> &'static str {
        // SAFETY: We leak the string to get a 'static lifetime.
        // This is acceptable since ExternalGenerator instances are typically
        // created once and live for the duration of the program.
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn extension(&self) -> &'static str {
        // External generators control their own file naming
        ""
    }

    fn generate(&self, schema: &ir::Schema) -> Result<GeneratorOutput, GeneratorError> {
        // Serialize schema to JSON
        let input = serde_json::to_string(schema)
            .map_err(|e| GeneratorError::Serialization(e.to_string()))?;

        // Spawn the generator process
        let mut child = Command::new(&self.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                GeneratorError::Process(format!(
                    "failed to spawn {}: {}",
                    self.binary_path.display(),
                    e
                ))
            })?;

        // Write JSON to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(|e| {
                GeneratorError::Process(format!("failed to write to stdin: {}", e))
            })?;
        }

        // Wait for process to complete
        let output = child.wait_with_output().map_err(|e| {
            GeneratorError::Process(format!("failed to wait for process: {}", e))
        })?;

        // Check exit status
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeneratorError::Process(format!(
                "generator exited with status {}: {}",
                output.status,
                stderr.trim()
            )));
        }

        // Parse stdout as GeneratorOutput
        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            GeneratorError::InvalidOutput(format!("invalid UTF-8 in output: {}", e))
        })?;

        serde_json::from_str(&stdout).map_err(|e| {
            GeneratorError::InvalidOutput(format!(
                "failed to parse generator output as JSON: {}",
                e
            ))
        })
    }
}