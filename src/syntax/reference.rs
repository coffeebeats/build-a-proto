use thiserror::Error;

/* -------------------------------------------------------------------------- */
/*                            Enum: ReferenceError                            */
/* -------------------------------------------------------------------------- */

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ReferenceError {
    #[error("reference name cannot be empty")]
    EmptyName,

    #[error("invalid reference name '{0}' (only [a-zA-Z0-9_] allowed)")]
    InvalidNameCharacters(String),

    #[error("invalid path segment '{0}' (only [a-zA-Z0-9_] allowed)")]
    InvalidPathCharacters(String),
}

/* -------------------------------------------------------------------------- */
/*                              Struct: Reference                             */
/* -------------------------------------------------------------------------- */

/// `Reference` represents a validated reference to another type in the schema.
///
/// References can be either absolute (starting with `.`) or relative (resolved
/// from the current scope outward). All references are validated at construction
/// time to ensure they contain only valid identifiers.
///
/// ## Validation Rules
///
/// - Reference names must be non-empty
/// - Names and path segments must contain only ASCII alphanumeric characters
///   and underscores
#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    /// `absolute` denotes whether this is an absolute reference (starts with
    /// `.`).
    absolute: bool,
    /// `path` contains the path segments leading to the type (package and/or
    /// nested scope).
    path: Vec<String>,
    /// `name` is the name of the referenced type.
    name: String,
}

/* ----------------------------- Impl: Reference ---------------------------- */

impl Reference {
    /// `is_absolute` returns whether the reference is an absolute reference
    /// (i.e. it's defined with a leading `.`).
    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// `name` returns the [`Reference`]'s name (final identifier).
    #[allow(unused)]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `path` returns the [`Reference`]'s path segments.
    #[allow(unused)]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// `try_new_absolute` constructs a new absolute reference from the provided
    /// scope `path` and reference `name`, validating that all identifiers are
    /// valid.
    pub fn try_new_absolute<T, U>(path: Vec<T>, name: U) -> Result<Self, ReferenceError>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        Self::try_new(path, name, true)
    }

    /// `try_new_relative` constructs a new relative reference from the provided
    /// scope `path` and reference `name`, validating that all identifiers are
    /// valid.
    pub fn try_new_relative<T, U>(path: Vec<T>, name: U) -> Result<Self, ReferenceError>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        Self::try_new(path, name, false)
    }

    fn try_new<T, U>(path: Vec<T>, name: U, absolute: bool) -> Result<Self, ReferenceError>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        if name.as_ref().is_empty() {
            return Err(ReferenceError::EmptyName);
        }

        if !name
            .as_ref()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(ReferenceError::InvalidNameCharacters(
                name.as_ref().to_owned(),
            ));
        }

        for segment in &path {
            if !segment
                .as_ref()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(ReferenceError::InvalidPathCharacters(
                    segment.as_ref().to_owned(),
                ));
            }
        }

        Ok(Self {
            path: path.into_iter().map(|s| s.as_ref().to_owned()).collect(),
            absolute,
            name: name.as_ref().to_owned(),
        })
    }
}

/* ------------------------------ Impl: Display ----------------------------- */

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            if self.absolute { "." } else { "" },
            self.path.join("."),
            if !self.path.is_empty() { "." } else { "" },
            self.name,
        )
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_valid_absolute() {
        let result = Reference::try_new_absolute(vec!["foo"], "Bar");
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.is_absolute());
        assert_eq!(r.name(), "Bar");
        assert_eq!(r.to_string(), ".foo.Bar");
    }

    #[test]
    fn test_reference_valid_absolute_nested() {
        let result = Reference::try_new_absolute(vec!["foo", "bar"], "Baz");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), ".foo.bar.Baz");
    }

    #[test]
    fn test_reference_valid_relative() {
        let result = Reference::try_new_relative(Vec::<&str>::new(), "Foo");
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(!r.is_absolute());
        assert_eq!(r.name(), "Foo");
        assert_eq!(r.to_string(), "Foo");
    }

    #[test]
    fn test_reference_valid_relative_with_path() {
        let result = Reference::try_new_relative(vec!["foo"], "Bar");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "foo.Bar");
    }

    #[test]
    fn test_reference_rejects_empty_name() {
        let result = Reference::try_new_absolute(Vec::<&str>::new(), "");
        assert_eq!(result.unwrap_err(), ReferenceError::EmptyName);
    }

    #[test]
    fn test_reference_rejects_invalid_name_chars() {
        let result = Reference::try_new_absolute(Vec::<&str>::new(), "Invalid-Name");
        assert!(matches!(
            result,
            Err(ReferenceError::InvalidNameCharacters(_))
        ));
    }

    #[test]
    fn test_reference_rejects_invalid_name_chars_dot() {
        let result = Reference::try_new_relative(Vec::<&str>::new(), "Invalid.Name");
        assert!(matches!(
            result,
            Err(ReferenceError::InvalidNameCharacters(_))
        ));
    }

    #[test]
    fn test_reference_rejects_invalid_path_segment() {
        let result = Reference::try_new_absolute(vec!["invalid.path".to_string()], "Name");
        assert!(matches!(
            result,
            Err(ReferenceError::InvalidPathCharacters(_))
        ));
    }

    #[test]
    fn test_reference_allows_underscores() {
        let result = Reference::try_new_absolute(vec!["foo_bar".to_string()], "Baz_Qux");
        assert!(result.is_ok());
    }

    #[test]
    fn test_reference_allows_alphanumeric() {
        let result = Reference::try_new_absolute(vec!["foo123".to_string()], "Bar456");
        assert!(result.is_ok());
    }
}
