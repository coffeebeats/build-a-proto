use serde::{Deserialize, Serialize};

use super::Encoding;

/* -------------------------------------------------------------------------- */
/*                               Struct: Field                                */
/* -------------------------------------------------------------------------- */

/// `Field` represents a fully resolved field in a message or enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Field index in serialization order.
    pub index: u32,
    /// Encoding specification for this field.
    pub encoding: Encoding,
    /// Documentation comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}
