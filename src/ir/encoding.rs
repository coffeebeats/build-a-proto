use serde::{Deserialize, Serialize};

/* -------------------------------------------------------------------------- */
/*                              Struct: Encoding                              */
/* -------------------------------------------------------------------------- */

/// `Encoding` specifies how a field is serialized to the wire format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    /// Wire format specification (bits, length-prefixed, embedded).
    pub wire: WireFormat,
    /// Native type representation for code generation.
    pub native: NativeType,
    /// Optional transformations applied during encoding/decoding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transforms: Vec<Transform>,
    /// Optional padding bits to add after this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_bits: Option<u64>,
}

/* -------------------------------------------------------------------------- */
/*                             Enum: WireFormat                               */
/* -------------------------------------------------------------------------- */

/// `WireFormat` describes how data is laid out in the binary stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireFormat {
    /// Fixed number of bits.
    Bits { count: u64 },
    /// Variable-length with length prefix.
    LengthPrefixed { prefix_bits: u8 },
    /// Embedded message (recursively encoded).
    Embedded,
}

/* -------------------------------------------------------------------------- */
/*                             Enum: NativeType                               */
/* -------------------------------------------------------------------------- */

/// `NativeType` represents the language-level type for code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeType {
    Bool,
    Int {
        bits: u8,
        signed: bool,
    },
    Float {
        bits: u8,
    },
    String,
    Bytes,
    Array {
        element: Box<Encoding>,
    },
    Map {
        key: Box<Encoding>,
        value: Box<Encoding>,
    },
    /// Reference to message by descriptor string.
    Message {
        descriptor: String,
    },
    /// Reference to enum by descriptor string.
    Enum {
        descriptor: String,
    },
}

/* -------------------------------------------------------------------------- */
/*                              Enum: Transform                               */
/* -------------------------------------------------------------------------- */

/// `Transform` describes encoding transformations applied to values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Transform {
    ZigZag,
    Delta,
    FixedPoint {
        integer_bits: u8,
        fractional_bits: u8,
    },
}
