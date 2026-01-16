use std::fmt;

use derive_builder::Builder;
use serde::Deserialize;
use serde::Serialize;

use super::PackageName;

/* -------------------------------------------------------------------------- */
/*                             Struct: Descriptor                             */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Descriptor {
    pub package: PackageName,
    #[builder(default)]
    pub path: Vec<String>,
}

/* ---------------------------- Impl: Descriptor ---------------------------- */

impl Descriptor {
    /// `name` returns the last component of the path (the "name" of this type).
    /// Returns `None` if the path is empty (descriptor represents a package
    /// only).
    pub fn name(&self) -> Option<&str> {
        self.path.last().map(|s| s.as_str())
    }

    /// `parts` returns all components of the [`Descriptor`] as a vector of
    /// `String`s: package components + path components.
    pub fn parts(&self) -> Vec<String> {
        let mut result = Vec::new();

        result.extend(self.package.iter().cloned());
        result.extend(self.path.iter().cloned());

        result
    }

    /// `pop` removes the last item from the [`Descriptor`]'s `path`. This has
    /// no effect if `path` is already empty.
    pub fn pop(&mut self) -> Option<String> {
        self.path.pop()
    }

    /// `push` appends the provided component to `path`.
    ///
    pub fn push<T>(&mut self, component: T)
    where
        T: AsRef<str>,
    {
        self.path.push(component.as_ref().to_string());
    }
}

/* ------------------------------ Impl: Display ----------------------------- */

impl fmt::Display for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.package)?;

        if !self.path.is_empty() {
            write!(f, ".{}", self.path.join("."))?;
        }

        Ok(())
    }
}

/* ------------------------- Impl: From<PackageName> ------------------------ */

impl From<PackageName> for Descriptor {
    fn from(name: PackageName) -> Self {
        DescriptorBuilder::default().package(name).build().unwrap()
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_display_package_only() {
        // Given: A descriptor with only a package.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: Only the package name should be displayed.
        assert_eq!(result, "com.example");
    }

    #[test]
    fn test_descriptor_display_package_with_single_path_element() {
        // Given: A descriptor with a package and single path element.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Message".to_string()])
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: The package and path should be joined with dots.
        assert_eq!(result, "com.example.Message");
    }

    #[test]
    fn test_descriptor_display_package_with_multiple_path_elements() {
        // Given: A descriptor with a package and multiple path elements.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Outer".to_string(), "Inner".to_string()])
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: All path elements should be joined with dots.
        assert_eq!(result, "com.example.Outer.Inner");
    }

    #[test]
    fn test_descriptor_display_package_with_name() {
        // Given: A descriptor with a package and name.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Field".to_string()])
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: The package and name should be joined with a dot.
        assert_eq!(result, "com.example.Field");
    }

    #[test]
    fn test_descriptor_display_package_with_path_and_name() {
        // Given: A descriptor with package, path, and name.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Message".to_string(), "field".to_string()])
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: All parts should be joined with dots.
        assert_eq!(result, "com.example.Message.field");
    }

    #[test]
    fn test_descriptor_display_complex_nested_structure() {
        // Given: A descriptor with complex nested structure.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["org", "proto", "test"]).unwrap())
            .path(vec![
                "Outer".to_string(),
                "Middle".to_string(),
                "Inner".to_string(),
                "nested_field".to_string(),
            ])
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: All components should be properly joined.
        assert_eq!(result, "org.proto.test.Outer.Middle.Inner.nested_field");
    }

    #[test]
    fn test_descriptor_display_empty_path_ignored() {
        // Given: A descriptor with an empty path.
        let desc = DescriptorBuilder::default()
            .package(PackageName::try_from(vec!["com", "example"]).unwrap())
            .path(vec!["Field".to_string()])
            .build()
            .unwrap();

        // When: Converting to string.
        let result = desc.to_string();

        // Then: The empty path should be ignored.
        assert_eq!(result, "com.example.Field");
    }
}
