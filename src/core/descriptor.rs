use derive_builder::Builder;
use derive_more::Display;
use serde::Deserialize;
use serde::Serialize;

use super::PackageName;

/* -------------------------------------------------------------------------- */
/*                             Struct: Descriptor                             */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Deserialize, Display, PartialEq, Eq, Hash, Serialize)]
#[display("{}", String::from(self))]
pub struct Descriptor {
    pub package: PackageName,
    #[builder(default)]
    pub path: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub name: Option<String>,
}

/* --------------------------- Impl: Into<String> --------------------------- */

impl From<&Descriptor> for String {
    fn from(value: &Descriptor) -> Self {
        let name = value.name.as_deref().unwrap_or("");
        let pkg = value.package.to_string();
        let path = if !value.path.is_empty() {
            value.path.join(".")
        } else {
            "".to_owned()
        };

        vec![pkg.as_str(), path.as_str(), name]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join(".")
    }
}