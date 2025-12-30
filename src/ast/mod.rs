mod types;

use std::path::PathBuf;

use crate::core::PackageName;
use crate::lex::Span;

#[allow(unused_imports)]
pub use types::ScalarType;
pub use types::Type;
#[allow(unused_imports)]
pub use types::TypeKind;

/* -------------------------------------------------------------------------- */
/*                             Struct: SourceFile                             */
/* -------------------------------------------------------------------------- */

/// `SourceFile` represents a complete `.baproto` schema file.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    pub includes: Vec<Include>,
    pub items: Vec<Item>,
    pub package: Option<Package>,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Package                              */
/* -------------------------------------------------------------------------- */

/// `Package` represents a package declaration in a schema file.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub name: PackageName,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Include                              */
/* -------------------------------------------------------------------------- */

/// `Include` represents an include statement for importing other schema files.
#[derive(Clone, Debug, PartialEq)]
pub struct Include {
    pub path: PathBuf,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                                 Enum: Item                                 */
/* -------------------------------------------------------------------------- */

/// `Item` represents a top-level item in a schema file.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Message(Message),
    Enum(Enum),
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Message                              */
/* -------------------------------------------------------------------------- */

/// `Message` represents a message definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub doc: Option<DocComment>,
    pub fields: Vec<Field>,
    pub name: Ident,
    pub nested_enums: Vec<Enum>,
    pub nested_messages: Vec<Message>,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Field                               */
/* -------------------------------------------------------------------------- */

/// `Field` represents a field within a message.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub doc: Option<DocComment>,
    pub encoding: Option<EncodingSpec>,
    pub index: FieldIndex,
    pub name: Ident,
    pub span: Span,
    pub typ: Type,
}

/* -------------------------------------------------------------------------- */
/*                                 Struct: Enum                               */
/* -------------------------------------------------------------------------- */

/// `Enum` represents an enum definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Enum {
    pub doc: Option<DocComment>,
    pub name: Ident,
    pub span: Span,
    pub variants: Vec<Variant>,
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Variant                              */
/* -------------------------------------------------------------------------- */

/// `Variant` represents a variant within an enum.
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub doc: Option<DocComment>,
    pub index: FieldIndex,
    pub kind: VariantKind,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                              Enum: VariantKind                             */
/* -------------------------------------------------------------------------- */

/// `VariantKind` specifies whether an enum variant is a simple unit variant
/// or a variant with associated data (like a field).
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum VariantKind {
    /// A variant with associated field data.
    Field(Field),

    /// A unit variant with just a name.
    Unit(Ident),
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Ident                               */
/* -------------------------------------------------------------------------- */

/// `Ident` represents an identifier with its source location.
#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                             Struct: DocComment                             */
/* -------------------------------------------------------------------------- */

/// `DocComment` represents documentation comments attached to a declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct DocComment {
    pub lines: Vec<String>,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                             Struct: FieldIndex                             */
/* -------------------------------------------------------------------------- */

/// `FieldIndex` represents a field or variant index with its source location.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldIndex {
    pub span: Span,
    pub value: u64,
}

/* -------------------------------------------------------------------------- */
/*                             Struct: EncodingSpec                           */
/* -------------------------------------------------------------------------- */

/// `EncodingSpec` represents encoding specifications for a field.
#[derive(Clone, Debug, PartialEq)]
pub struct EncodingSpec {
    pub encodings: Vec<Encoding>,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                              Enum: Encoding                                */
/* -------------------------------------------------------------------------- */

/// `Encoding` specifies how a field should be encoded in the wire format.
#[allow(unused)]
#[derive(Clone, Debug, PartialEq)]
pub enum Encoding {
    /// Fixed-size bit encoding.
    Bits(usize),

    /// Variable-length bit encoding with a maximum size.
    BitsVariable(usize),

    /// Delta encoding (difference from previous value).
    Delta,

    /// Fixed-point encoding with integer and fractional bits.
    FixedPoint(usize, usize),

    /// Padding bits.
    Pad(usize),

    /// ZigZag encoding for signed integers.
    ZigZag,
}
