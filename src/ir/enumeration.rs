use serde::{Deserialize, Serialize};

use crate::core::Descriptor;

use super::{Encoding, Field};

/* -------------------------------------------------------------------------- */
/*                                Struct: Enum                                */
/* -------------------------------------------------------------------------- */

/// `Enum` represents a fully resolved enum type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enum {
    /// `descriptor` uniquely identifies the [`Enum`].
    pub descriptor: Descriptor,
    /// `discriminant` describes the [`Enum`]'s discriminant encoding.
    pub discriminant: Encoding,
    /// `doc` is a doc comment for the [`Enum`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,

    /// `variants` in serialization order.
    pub variants: Vec<Variant>,
}

/* ------------------------------- Impl: Enum ------------------------------- */

impl Enum {
    /// `name` returns the name of the [`Enum`].
    pub fn name(&self) -> Option<&str> {
        self.descriptor.name()
    }
}

/* -------------------------------------------------------------------------- */
/*                                Enum: Variant                               */
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
