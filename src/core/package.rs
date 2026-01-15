use derive_more::Display;
use serde::Deserialize;
use serde::Serialize;
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
/// ### Examples
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
#[derive(Clone, Debug, Deserialize, Display, PartialEq, Eq, Hash, Serialize)]
#[display("{}", self.0.join("."))]
pub struct PackageName(Vec<String>);

/* ---------------------------- Impl: PackageName --------------------------- */

impl PackageName {
    /// `suffix` returns the final segment of the package name.
    pub fn suffix(&self) -> &str {
        self.0.last().expect("missing package name")
    }
}

/* ------------------------------- Impl: Deref ------------------------------ */

impl std::ops::Deref for PackageName {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/* ------------------------ Impl: TryFrom<Vec<&str>> ------------------------ */

impl<T> TryFrom<Vec<T>> for PackageName
where
    T: AsRef<str>,
{
    type Error = PackageNameError;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(PackageNameError::Empty);
        }

        for segment in &value {
            let Some(first) = segment.as_ref().chars().next() else {
                return Err(PackageNameError::MissingSegment(
                    value
                        .iter()
                        .map(|s| s.as_ref())
                        .collect::<Vec<_>>()
                        .join("."),
                ));
            };

            if !first.is_ascii_lowercase() {
                return Err(PackageNameError::InvalidStart(segment.as_ref().to_owned()));
            }

            if !segment
                .as_ref()
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(PackageNameError::InvalidCharacters(
                    segment.as_ref().to_owned(),
                ));
            }
        }

        Ok(Self(
            value
                .into_iter()
                .map(|s| s.as_ref().to_owned())
                .collect::<Vec<_>>(),
        ))
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
        assert_eq!(result.unwrap(), PackageName(vec!["foo".into()]));
    }

    #[test]
    fn test_package_name_valid_multi_segment() {
        let result = PackageName::try_from(vec!["foo", "bar"]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PackageName(vec!["foo".into(), "bar".into()])
        );
    }

    #[test]
    fn test_package_name_valid_with_underscores() {
        let result = PackageName::try_from(vec!["my_package", "sub_module"]);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PackageName(vec!["my_package".into(), "sub_module".into()])
        );
    }

    #[test]
    fn test_package_name_valid_with_digits() {
        let result = PackageName::try_from(vec!["foo2", "bar3"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_package_name_rejects_empty() {
        let result = PackageName::try_from(Vec::<String>::default());
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
        assert_eq!(pkg, Ok(PackageName(vec!["foo".into(), "bar".into()])));
    }
}
