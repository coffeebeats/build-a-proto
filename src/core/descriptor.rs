use std::fmt;

use derive_builder::Builder;
use serde::Deserialize;
use serde::Serialize;

use super::PackageName;

/* -------------------------------------------------------------------------- */
/*                             Struct: Descriptor                             */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct Descriptor {
    pub package: PackageName,
    #[builder(default)]
    pub path: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub name: Option<String>,
}

/* ------------------------------ Impl: Display ----------------------------- */

impl fmt::Display for Descriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.package)?;

        if !self.path.is_empty() {
            write!(f, ".{}", self.path.join("."))?;
        }

        if let Some(name) = &self.name {
            write!(f, ".{}", name)?;
        }

        Ok(())
    }
}
