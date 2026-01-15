use serde::{Deserialize, Serialize};

use crate::core::PackageName;

use super::{Enum, Message};

/* -------------------------------------------------------------------------- */
/*                              Struct: Package                               */
/* -------------------------------------------------------------------------- */

/// `Package` represents a logical namespace that may be contributed to by
/// multiple source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Full package path: "foo.bar".
    pub name: PackageName,
    /// Top-level messages (nested types inline).
    pub messages: Vec<Message>,
    /// Top-level enums.
    pub enums: Vec<Enum>,
}
