use thiserror::Error;

use crate::core::SchemaImport;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                               Struct: Error                                */
/* -------------------------------------------------------------------------- */

/// A semantic analysis error with location information.
#[derive(Clone, Debug)]
pub struct Error {
    /// The source file where the error occurred.
    pub file: SchemaImport,
    /// The span within the source file.
    pub span: Span,
    /// The kind of error.
    pub kind: ErrorKind,
}

/* ------------------------- Impl: std::fmt::Display ------------------------ */

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file.as_path().display(), self.kind)
    }
}

/* ------------------------- Impl: std::error::Error ------------------------ */

impl std::error::Error for Error {}

/* -------------------------------------------------------------------------- */
/*                               Enum: ErrorKind                              */
/* -------------------------------------------------------------------------- */

/// Enumeration of all possible semantic analysis errors.
#[derive(Clone, Debug, Error)]
pub enum ErrorKind {
    #[error("duplicate type definition: '{0}'")]
    DuplicateType(String),

    // Cycle detection errors
    #[error("circular module dependency: {}", format_cycle(.0))]
    CircularDependency(Vec<SchemaImport>),

    #[error("missing include: '{0}'")]
    MissingInclude(String),

    // Name resolution errors
    #[error("unresolved type reference: '{0}'")]
    UnresolvedReference(String),

    // Type checking errors
    #[error("invalid map key type: '{0}' (must be scalar)")]
    InvalidMapKeyType(String),

    #[error("invalid array size: must be positive")]
    InvalidArraySize,

    // Index validation errors
    #[error("duplicate field index: {0}")]
    DuplicateFieldIndex(u64),

    #[error("duplicate variant index: {0}")]
    DuplicateVariantIndex(u64),

    #[error("index {0} exceeds maximum allowed value")]
    IndexOutOfRange(u64),

    // Name validation errors
    #[error("duplicate field name: '{0}'")]
    DuplicateFieldName(String),

    #[error("duplicate variant name: '{0}'")]
    DuplicateVariantName(String),

    #[error("duplicate nested type name: '{0}'")]
    DuplicateNestedTypeName(String),

    #[error("name conflict: '{0}' is used for both a field and a nested type")]
    FieldTypeNameConflict(String),

    // Value validation errors
    #[error("bits({0}) exceeds type width of {1} bits")]
    BitsExceedsTypeWidth(u64, u64),

    #[error("bits({0}) exceeds maximum of 64 bits")]
    BitsExceedsLimit(u64),

    #[error("bits_variable({0}) exceeds maximum of 64")]
    BitsVariableExceedsLimit(u64),

    #[error("fixed_point({0}, {1}) total of {2} bits exceeds maximum of 64")]
    FixedPointExceedsLimit(u64, u64, u64),

    // Resource limit errors
    #[error("message has {0} fields, exceeding limit of {1}")]
    TooManyFields(usize, usize),

    #[error("enum has {0} variants, exceeding limit of {1}")]
    TooManyVariants(usize, usize),

    #[error("nesting depth of {0} exceeds limit of {1}")]
    NestingTooDeep(usize, usize),

    #[error("array size {0} exceeds limit of {1}")]
    ArraySizeTooLarge(u64, u64),

    // Encoding validation errors (placeholder for encoding.rs)
    #[error("encoding '{encoding}' is incompatible with type '{typ}'")]
    IncompatibleEncoding { encoding: String, typ: String },
}

/* ---------------------------- Fn: format_cycle ---------------------------- */

/// `format_cycle` formats a cycle of schema imports for display.
fn format_cycle(cycle: &[SchemaImport]) -> String {
    cycle
        .iter()
        .map(|p| p.as_path().display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}
