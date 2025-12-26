use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

/* -------------------------------------------------------------------------- */
/*                          Enum: PathValidationError                         */
/* -------------------------------------------------------------------------- */

/// Errors that can occur during path validation.
#[derive(Error, Debug)]
pub enum PathValidationError {
    /// The path does not exist on the filesystem.
    #[error("path does not exist: {path}")]
    DoesNotExist { path: PathBuf },

    /// The path exists but is not a directory.
    #[error("path is not a directory: {path}")]
    NotADirectory { path: PathBuf },

    /// The path exists but is not a file.
    #[error("path is not a file: {path}")]
    NotAFile { path: PathBuf },

    /// An invalid filepath that could not be canonicalized for one reason or
    /// another.
    #[error("invalid path '{path}': {source}")]
    InvalidPath {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The file does not have the required `.baproto` extension.
    #[error("file must have .baproto extension: {path}")]
    InvalidExtension { path: PathBuf },

    /// The resolved path escapes its import root.
    #[error("path escapes import root: {path} is not within {root}")]
    PathEscapesRoot { path: PathBuf, root: PathBuf },
}

/* -------------------------------------------------------------------------- */
/*                             Struct: ImportRoot                             */
/* -------------------------------------------------------------------------- */

/// A canonicalized directory path used as an import search root.
///
/// This type enforces two invariants at construction:
/// 1. The path must be a valid directory
/// 2. The path is canonicalized (absolute, symlinks resolved)
///
/// These invariants allow functions accepting `ImportRoot` to skip validation
/// and trust the path is ready for use in import resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportRoot(PathBuf);

/* ---------------------------- Impl: ImportRoot ---------------------------- */

impl ImportRoot {
    /// `as_path` returns the path as a `&Path`.
    #[allow(unused)]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// `exists` checks if the import root still exists on the filesystem.
    ///
    /// NOTE: Paths can become invalid after construction if the directory is
    /// deleted or unmounted.
    #[allow(unused)]
    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    /// `resolve_schema_import` resolves the provided relative path within this
    /// import root to a validated schema import.
    ///
    /// This method:
    ///     1. Joins the path to the import root
    ///     2. Validates it's a file
    ///     3. Canonicalizes it
    ///     4. Ensures it doesn't escape the import root (symlink defense)
    ///     5. Returns a validated `SchemaImport`
    /// ```
    pub fn resolve_schema_import<T>(&self, path: T) -> Result<SchemaImport, PathValidationError>
    where
        T: AsRef<Path>,
    {
        let candidate = self.0.join(path);

        if !candidate.exists() {
            return Err(PathValidationError::DoesNotExist { path: candidate });
        }

        if !candidate.is_file() {
            return Err(PathValidationError::NotAFile { path: candidate });
        }

        let canonical = candidate
            .canonicalize()
            .map_err(|e| PathValidationError::InvalidPath {
                path: candidate.clone(),
                source: e,
            })?;

        // Ensure the canonical path doesn't escape the import root. This
        // defends against symlink attacks and relative filepaths that could
        // leak filesystem info.
        if !canonical.starts_with(&self.0) {
            return Err(PathValidationError::PathEscapesRoot {
                path: canonical,
                root: self.0.clone(),
            });
        }

        SchemaImport::try_from(canonical)
    }
}

/* -------------------------- Impl: TryFrom<&Path> -------------------------- */

impl TryFrom<&Path> for ImportRoot {
    type Error = PathValidationError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if !path.exists() {
            return Err(PathValidationError::DoesNotExist {
                path: path.to_path_buf(),
            });
        }

        if !path.is_dir() {
            return Err(PathValidationError::NotADirectory {
                path: path.to_path_buf(),
            });
        }

        let canonical = path
            .canonicalize()
            .map_err(|e| PathValidationError::InvalidPath {
                path: path.to_path_buf(),
                source: e,
            })?;

        Ok(Self(canonical))
    }
}

/* ------------------------- Impl: TryFrom<PathBuf> ------------------------- */

impl TryFrom<PathBuf> for ImportRoot {
    type Error = PathValidationError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(path.as_path())
    }
}

/* --------------------------- Impl: TryFrom<&str> -------------------------- */

impl<'a> TryFrom<&'a str> for ImportRoot {
    type Error = PathValidationError;

    fn try_from(path: &'a str) -> Result<Self, Self::Error> {
        Self::try_from(Path::new(path))
    }
}

/* ---------------------------- Impl: AsRef<Path> --------------------------- */

impl AsRef<Path> for ImportRoot {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/* -------------------------------------------------------------------------- */
/*                            Struct: SchemaImport                            */
/* -------------------------------------------------------------------------- */

/// A canonicalized path to a validated `.baproto` schema import.
///
/// This type enforces three invariants at construction:
///     1. The path must point to a file (not a directory)
///     2. The path is canonicalized (absolute, symlinks resolved)
///     3. The file has a `.baproto` extension
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SchemaImport(PathBuf);

/* --------------------------- Impl: SchemaImport --------------------------- */

impl SchemaImport {
    /// `as_path` returns the path as a `&Path`.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// `exists` checks if the schema import still exists on the filesystem.
    ///
    /// NOTE: Paths can become invalid after construction if the file is deleted
    /// or the filesystem is unmounted.
    #[allow(unused)]
    pub fn exists(&self) -> bool {
        self.0.exists()
    }
}

/* -------------------------- Impl: TryFrom<&Path> -------------------------- */

impl TryFrom<&Path> for SchemaImport {
    type Error = PathValidationError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if !path.exists() {
            return Err(PathValidationError::DoesNotExist {
                path: path.to_path_buf(),
            });
        }

        if !path.is_file() {
            return Err(PathValidationError::NotAFile {
                path: path.to_path_buf(),
            });
        }

        if path.extension() != Some(std::ffi::OsStr::new("baproto")) {
            return Err(PathValidationError::InvalidExtension {
                path: path.to_path_buf(),
            });
        }

        let canonical = path
            .canonicalize()
            .map_err(|e| PathValidationError::InvalidPath {
                path: path.to_path_buf(),
                source: e,
            })?;

        Ok(Self(canonical))
    }
}

/* ------------------------- Impl: TryFrom<PathBuf> ------------------------- */

impl TryFrom<PathBuf> for SchemaImport {
    type Error = PathValidationError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::try_from(path.as_path())
    }
}

/* --------------------------- Impl: TryFrom<&str> -------------------------- */

impl<'a> TryFrom<&'a str> for SchemaImport {
    type Error = PathValidationError;

    fn try_from(path: &'a str) -> Result<Self, Self::Error> {
        Self::try_from(Path::new(path))
    }
}

/* ---------------------------- Impl: AsRef<Path> --------------------------- */

impl AsRef<Path> for SchemaImport {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /* -------------------------- Tests: ImportRoot ------------------------- */

    #[test]
    fn test_import_root_try_from_valid_directory() {
        // Given: A valid temporary directory.
        let dir = TempDir::new().unwrap();

        // When: Creating an ImportRoot from the directory path.
        let root = ImportRoot::try_from(dir.path());

        // Then: The ImportRoot is successfully created.
        assert!(root.is_ok());
    }

    #[test]
    fn test_import_root_try_from_rejects_file() {
        // Given: A regular file instead of a directory.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "").unwrap();

        // When: Attempting to create an ImportRoot.
        let root = ImportRoot::try_from(file_path.as_path());

        // Then: The operation fails with an error.
        assert!(matches!(
            root,
            Err(PathValidationError::NotADirectory { .. })
        ));
    }

    #[test]
    fn test_import_root_try_from_rejects_nonexistent() {
        // Given: A nonexistent path.
        // When: Attempting to create an ImportRoot from it.
        let root = ImportRoot::try_from("/nonexistent/path/to/nowhere");

        // Then: The operation fails with an error.
        assert!(matches!(
            root,
            Err(PathValidationError::DoesNotExist { .. })
        ));
    }

    #[test]
    fn test_import_root_canonicalizes() {
        // Given: A temporary directory with its canonical path.
        let dir = TempDir::new().unwrap();
        let canonical = dir.path().canonicalize().unwrap();

        // When: Creating an ImportRoot from the directory.
        let root = ImportRoot::try_from(dir.path()).unwrap();

        // Then: The stored path is absolute and matches the canonical path.
        assert!(root.as_path().is_absolute());
        assert_eq!(root.as_path(), canonical);
    }

    #[test]
    fn test_import_root_exists() {
        // Given: A valid ImportRoot for an existing directory.
        let dir = TempDir::new().unwrap();
        let root = ImportRoot::try_from(dir.path()).unwrap();

        // When: Checking if the import root exists.
        // Then: The exists check returns true.
        assert!(root.exists());
    }

    #[test]
    fn test_import_root_resolve_schema_import_success() {
        // Given: An ImportRoot with a valid .baproto file.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("schema.baproto");
        fs::write(&file_path, "").unwrap();
        let root = ImportRoot::try_from(dir.path()).unwrap();

        // When: Resolving the schema import path.
        let schema = root.resolve_schema_import(Path::new("schema.baproto"));

        // Then: The schema import is successfully resolved and canonicalized.
        assert!(schema.is_ok());
        assert_eq!(schema.unwrap().as_path(), file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_import_root_resolve_schema_import_not_found() {
        // Given: An ImportRoot without the requested file.
        let dir = TempDir::new().unwrap();
        let root = ImportRoot::try_from(dir.path()).unwrap();

        // When: Attempting to resolve a nonexistent schema import.
        let schema = root.resolve_schema_import(Path::new("missing.baproto"));

        // Then: The operation fails with an error.
        assert!(matches!(
            schema,
            Err(PathValidationError::DoesNotExist { .. })
        ));
    }

    #[test]
    fn test_import_root_resolve_schema_import_rejects_directory() {
        // Given: An ImportRoot containing a subdirectory.
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let root = ImportRoot::try_from(dir.path()).unwrap();

        // When: Attempting to resolve a directory as a schema import.
        let schema = root.resolve_schema_import(Path::new("subdir"));

        // Then: The operation fails with an error.
        assert!(matches!(schema, Err(PathValidationError::NotAFile { .. })));
    }

    #[test]
    fn test_import_root_resolve_schema_import_validates_extension() {
        // Given: An ImportRoot with a file lacking the .baproto extension.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("schema.txt");
        fs::write(&file_path, "").unwrap();
        let root = ImportRoot::try_from(dir.path()).unwrap();

        // When: Attempting to resolve the file as a schema import.
        let schema = root.resolve_schema_import(Path::new("schema.txt"));

        // Then: The operation fails with an error.
        assert!(matches!(
            schema,
            Err(PathValidationError::InvalidExtension { .. })
        ));
    }

    #[test]
    fn test_import_root_resolve_schema_import_prevents_escape_via_symlink() {
        // Given: An ImportRoot with a symlink to a file outside the root.
        let dir = TempDir::new().unwrap();
        let outside_dir = TempDir::new().unwrap();
        let outside_file = outside_dir.path().join("outside.baproto");
        fs::write(&outside_file, "").unwrap();
        let symlink_path = dir.path().join("link.baproto");

        #[cfg(unix)]
        let symlink_result = std::os::unix::fs::symlink(&outside_file, &symlink_path);
        #[cfg(windows)]
        let symlink_result = std::os::windows::fs::symlink_file(&outside_file, &symlink_path);
        assert!(symlink_result.is_ok());

        let root = ImportRoot::try_from(dir.path()).unwrap();

        // When: Attempting to resolve the symlink as a schema import.
        let schema = root.resolve_schema_import(Path::new("link.baproto"));

        // Then: The operation fails with PathEscapesRoot error.
        assert!(matches!(
            schema,
            Err(PathValidationError::PathEscapesRoot { .. })
        ));
    }

    /* ------------------------- Tests: SchemaImport ------------------------ */

    #[test]
    fn test_schema_import_try_from_valid_file() {
        // Given: A valid .baproto file.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("schema.baproto");
        fs::write(&file_path, "").unwrap();

        // When: Creating a SchemaImport from the file path.
        let schema = SchemaImport::try_from(file_path.as_path());

        // Then: The SchemaImport is successfully created.
        assert!(schema.is_ok());
    }

    #[test]
    fn test_schema_import_try_from_rejects_directory() {
        // Given: A directory instead of a file.
        let dir = TempDir::new().unwrap();

        // When: Attempting to create a SchemaImport from the directory.
        let schema = SchemaImport::try_from(dir.path());

        // Then: The operation fails with an error.
        assert!(matches!(schema, Err(PathValidationError::NotAFile { .. })));
    }

    #[test]
    fn test_schema_import_try_from_rejects_nonexistent() {
        // Given: A nonexistent file path.
        // When: Attempting to create a SchemaImport from it.
        let schema = SchemaImport::try_from("/nonexistent/schema.baproto");

        // Then: The operation fails with DoesNotExist error.
        assert!(matches!(
            schema,
            Err(PathValidationError::DoesNotExist { .. })
        ));
    }

    #[test]
    fn test_schema_import_try_from_validates_extension() {
        // Given: A file without the .baproto extension.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("schema.txt");
        fs::write(&file_path, "").unwrap();

        // When: Attempting to create a SchemaImport from the file.
        let schema = SchemaImport::try_from(file_path.as_path());

        // Then: The operation fails with an error.
        assert!(matches!(
            schema,
            Err(PathValidationError::InvalidExtension { .. })
        ));
    }

    #[test]
    fn test_schema_import_canonicalizes() {
        // Given: A .baproto file with its canonical path.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("schema.baproto");
        fs::write(&file_path, "").unwrap();
        let canonical = file_path.canonicalize().unwrap();

        // When: Creating a SchemaImport from the file.
        let schema = SchemaImport::try_from(file_path.as_path()).unwrap();

        // Then: The stored path is absolute and matches the canonical path.
        assert!(schema.as_path().is_absolute());
        assert_eq!(schema.as_path(), canonical);
    }

    #[test]
    fn test_schema_import_exists() {
        // Given: A valid SchemaImport for an existing file.
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("schema.baproto");
        fs::write(&file_path, "").unwrap();
        let schema = SchemaImport::try_from(file_path.as_path()).unwrap();

        // When: Checking if the schema import exists.
        // Then: The exists check returns true.
        assert!(schema.exists());
    }
}
