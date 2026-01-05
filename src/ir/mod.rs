//! Intermediate Representation for baproto schemas.
//!
//! The IR is a hierarchical, self-describing representation designed for
//! serialization to JSON and consumption by generator plugins in any language.
//!
//! ## Design Principles
//!
//! - **Hierarchical structure** - Mirrors code organization (packages contain
//!   messages, messages contain nested types)
//! - **Self-describing** - Every type includes its full descriptor string
//! - **Serialization-first** - JSON structure is the primary interface for plugins
//! - **No validation** - IR assumes semantic correctness; validation happens before
//!   lowering
//! - **Package-oriented** - Multiple source files merge into package structure

use serde::{Deserialize, Serialize};

mod encoding;
mod enumeration;
mod field;
pub mod lower;
mod message;
mod package;

pub use encoding::{Encoding, NativeType, Transform, WireFormat};
pub use enumeration::{Enum, Variant};
pub use field::Field;
pub use message::Message;
pub use package::Package;

/* -------------------------------------------------------------------------- */
/*                               Struct: Schema                               */
/* -------------------------------------------------------------------------- */

/// `Schema` is the root of the IR, containing all packages with their types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    /// All packages (source files merged by package path).
    pub packages: Vec<Package>,
}
