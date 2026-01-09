mod types;

use derive_builder::Builder;
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
    pub package: Package,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Package                              */
/* -------------------------------------------------------------------------- */

/// `Package` represents a package declaration in a schema file.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub doc: Option<DocComment>,
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
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", name)]
pub struct Ident {
    pub name: String,
    pub span: crate::lex::Span,
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Text                                */
/* -------------------------------------------------------------------------- */

/// `Text` represents a single [`String`] literal with its source location.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("\"{}\"", content)]
pub struct Text {
    pub content: String,
    pub span: crate::lex::Span,
}

/* -------------------------------------------------------------------------- */
/*                                Struct: Uint                                */
/* -------------------------------------------------------------------------- */

/// `Uint` represents an unsigned integer with its source location.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", value)]
pub struct Uint<T = u64> {
    pub value: T,
    pub span: crate::lex::Span,
}
