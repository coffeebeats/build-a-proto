use serde::{Deserialize, Serialize};

use crate::core::Descriptor;

use super::{Enum, Field};

/* -------------------------------------------------------------------------- */
/*                              Struct: Message                               */
/* -------------------------------------------------------------------------- */

/// `Message` represents a fully resolved message type with nested types inline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// `descriptor` uniquely identifies the [`Message`].
    pub descriptor: Descriptor,
    /// `doc` is a doc comment for the [`Message`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// `enums` contains all *direct* nested [`Enum`]s.
    pub enums: Vec<Enum>,
    /// `fields` in serialization order.
    pub fields: Vec<Field>,
    /// `messages` contains all *direct* nested [`Message`]s.
    pub messages: Vec<Message>,
}

/* ------------------------------ Impl: Message ----------------------------- */

impl Message {
    /// `name` returns the name of the [`Message`].
    pub fn name(&self) -> Option<&str> {
        self.descriptor.name()
    }
}
