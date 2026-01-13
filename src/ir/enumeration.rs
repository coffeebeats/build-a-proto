use serde::{Deserialize, Serialize};

use super::{Encoding, Field};

/* -------------------------------------------------------------------------- */
/*                                Struct: Enum                                */
/* -------------------------------------------------------------------------- */

/// `Enum` represents a fully resolved enum type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    /// Full descriptor: "foo.bar.MyEnum".
    pub descriptor: String,
    /// Simple name: "MyEnum".
    pub name: String,
    /// How the discriminant is encoded.
    pub discriminant: Encoding,
    /// Enum variants.
    pub variants: Vec<Variant>,
    /// Documentation comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/* -------------------------------------------------------------------------- */
/*                               Enum: Variant                                */
/* -------------------------------------------------------------------------- */

/// `Variant` represents an enum variant (unit or data-carrying).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Variant {
    /// Unit variant with no associated data.
    Unit {
        name: String,
        index: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        doc: Option<String>,
    },
    /// Field variant with an associated field.
    Field {
        name: String,
        index: u32,
        field: Field,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        doc: Option<String>,
    },
}
