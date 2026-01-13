use serde::{Deserialize, Serialize};

use super::{Enum, Field};

/* -------------------------------------------------------------------------- */
/*                              Struct: Message                               */
/* -------------------------------------------------------------------------- */

/// `Message` represents a fully resolved message type with nested types inline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Full descriptor: "foo.bar.Outer.Inner".
    pub descriptor: String,
    /// Simple name: "Inner".
    pub name: String,
    /// Fields in serialization order.
    pub fields: Vec<Field>,
    /// Nested messages (inline).
    pub messages: Vec<Message>,
    /// Nested enums (inline).
    pub enums: Vec<Enum>,
    /// Documentation comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}
