use thiserror::Error;

/* -------------------------------------------------------------------------- */
/*                           Enum: PackageNameError                           */
/* -------------------------------------------------------------------------- */

#[derive(Error, Debug, Clone, PartialEq)]
pub enum PackageNameError {
    #[error("package name cannot be empty")]
    Empty,

    #[error("package name missing segment: '{0}'")]
    MissingSegment(String),

    #[error("package segment '{0}' contains invalid characters (only [a-z0-9_] allowed)")]
    InvalidCharacters(String),

    #[error("package segment '{0}' must start with a lowercase letter")]
    InvalidStart(String),
}

/* -------------------------------------------------------------------------- */
/*                             Struct: PackageName                            */
/* -------------------------------------------------------------------------- */

/// `PackageName` is a validated package name consisting of dot-separated
/// segments.
///
/// Each segment must:
/// - Start with a lowercase ASCII letter
/// - Contain only lowercase ASCII letters, digits, and underscores
///
/// # Examples
///
/// Valid package names:
/// - `foo`
/// - `foo.bar`
/// - `my_package.sub_module`
///
/// Invalid package names:
/// - `Foo` (uppercase)
/// - `foo.Bar` (uppercase in segment)
/// - `123.foo` (starts with digit)
/// - `.foo` (missing leading segment)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PackageName(Vec<String>);

/* ---------------------------- Impl: PackageName --------------------------- */

impl PackageName {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/* ------------------------ Impl: TryFrom<Vec<&str>> ------------------------ */

impl<'a> TryFrom<Vec<&'a str>> for PackageName {
    type Error = PackageNameError;

    fn try_from(value: Vec<&'a str>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(PackageNameError::Empty);
        }

        for segment in &value {
            let Some(first) = segment.chars().next() else {
                return Err(PackageNameError::MissingSegment(value.join(".")));
            };

            if !first.is_ascii_lowercase() {
                return Err(PackageNameError::InvalidStart((*segment).to_owned()));
            }

            if !segment
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(PackageNameError::InvalidCharacters((*segment).to_owned()));
            }
        }

        Ok(Self(value.into_iter().map(|s| s.to_owned()).collect()))
    }
}

/* ------------------------------- Impl: Deref ------------------------------ */

impl std::ops::Deref for PackageName {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/* ---------------------------- Impl: Display ----------------------------- */

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_valid_single_segment() {
        let result = PackageName::try_from(vec!["foo"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PackageName(vec!["foo".to_owned()]));
    }

    #[test]
    fn test_package_name_valid_multi_segment() {
        let result = PackageName::try_from(vec!["foo", "bar"]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PackageName(vec!["foo".to_owned(), "bar".to_owned()])
        );
    }

    #[test]
    fn test_package_name_valid_with_underscores() {
        let result = PackageName::try_from(vec!["my_package", "sub_module"]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PackageName(vec!["my_package".to_owned(), "sub_module".to_owned()])
        );
    }

    #[test]
    fn test_package_name_valid_with_digits() {
        let result = PackageName::try_from(vec!["foo2", "bar3"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_package_name_rejects_empty() {
        let result = PackageName::try_from(vec![]);
        assert_eq!(result.unwrap_err(), PackageNameError::Empty);
    }

    #[test]
    fn test_package_name_rejects_uppercase_start() {
        let result = PackageName::try_from(vec!["Foo"]);
        assert_eq!(
            result.unwrap_err(),
            PackageNameError::InvalidStart("Foo".to_string())
        );
    }

    #[test]
    fn test_package_name_rejects_uppercase_anywhere() {
        let result = PackageName::try_from(vec!["fooBar"]);
        assert_eq!(
            result.unwrap_err(),
            PackageNameError::InvalidCharacters("fooBar".to_string())
        );
    }

    #[test]
    fn test_package_name_rejects_digit_start() {
        let result = PackageName::try_from(vec!["123foo"]);
        assert_eq!(
            result.unwrap_err(),
            PackageNameError::InvalidStart("123foo".to_string())
        );
    }

    #[test]
    fn test_package_name_segments() {
        let pkg = PackageName::try_from(vec!["foo", "bar"]);
        assert_eq!(
            pkg,
            Ok(PackageName(vec!["foo".to_owned(), "bar".to_owned()]))
        );
    }

    #[test]
    fn test_package_name_len() {
        let pkg = PackageName::try_from(vec!["foo", "bar"]).unwrap();
        assert_eq!(pkg.len(), 2);
    }
}
