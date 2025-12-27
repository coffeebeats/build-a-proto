use derive_more::Display;

/* -------------------------------------------------------------------------- */
/*                              Struct: Reference                             */
/* -------------------------------------------------------------------------- */

/// `Reference` represents a reference to another type in the schema.
///
/// References can be either absolute (starting with `.`) or relative (resolved
/// from the current scope outward).
#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    /// Whether this is an absolute reference (starts with `.`).
    absolute: bool,
    /// The path segments leading to the type (package and/or nested scope).
    path: Vec<String>,
    /// The name of the referenced type.
    name: String,
}

/* ----------------------------- Impl: Reference ---------------------------- */

impl Reference {
    /// `is_absolute` returns whether the reference is an absolute reference
    /// (i.e. it's defined with a leading `.`).
    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// `name` returns the [`Reference`]'s name (final identifier).
    #[allow(unused)]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `new_absolute` constructs a new absolute reference from the provided
    /// scope `path` and reference `name`.
    pub fn new_absolute(path: Vec<String>, name: &str) -> Self {
        debug_assert!(!name.is_empty());
        debug_assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));

        Self {
            absolute: true,
            path: path,
            name: name.to_owned(),
        }
    }

    /// `new_relative` constructs a new relative reference from the provided
    /// scope `path` and reference `name`.
    pub fn new_relative(path: Vec<String>, name: &str) -> Self {
        Self {
            absolute: false,
            path: path,
            name: name.to_owned(),
        }
    }
}

/* ------------------------------ Impl: Display ----------------------------- */

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            if self.absolute { "." } else { "" },
            self.path.join("."),
            if !self.path.is_empty() { "." } else { "" },
            self.name,
        )
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Enum: Type                                 */
/* -------------------------------------------------------------------------- */

#[allow(dead_code)]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Type {
    #[display("[{}]{_0}", _1.map(|n| n.to_string()).unwrap_or("".to_owned()))]
    Array(Box<Type>, Option<usize>),
    #[display("[{_0}]{_1}")]
    Map(Box<Type>, Box<Type>),
    #[display("{_0}")]
    Reference(Reference),
    Scalar(Scalar),
}

/* --------------------- Impl: From<crate::parse::Type> --------------------- */

impl<'a> From<&crate::parse::Type<'a>> for Type {
    fn from(value: &crate::parse::Type<'a>) -> Self {
        match &value.kind {
            crate::parse::TypeKind::Reference {
                absolute,
                path,
                name,
            } => {
                let path = path.into_iter().map(|&s| s.to_owned()).collect();

                Type::Reference(if *absolute {
                    Reference::new_absolute(path, name)
                } else {
                    Reference::new_relative(path, name)
                })
            }
            crate::parse::TypeKind::Bit => Type::Scalar(Scalar::Bit),
            crate::parse::TypeKind::Bool => Type::Scalar(Scalar::Bool),
            crate::parse::TypeKind::Byte => Type::Scalar(Scalar::Byte),
            crate::parse::TypeKind::Float32 => Type::Scalar(Scalar::Float32),
            crate::parse::TypeKind::Float64 => Type::Scalar(Scalar::Float64),
            crate::parse::TypeKind::SignedInt16 => Type::Scalar(Scalar::SignedInt16),
            crate::parse::TypeKind::SignedInt32 => Type::Scalar(Scalar::SignedInt32),
            crate::parse::TypeKind::SignedInt64 => Type::Scalar(Scalar::SignedInt64),
            crate::parse::TypeKind::SignedInt8 => Type::Scalar(Scalar::SignedInt8),
            crate::parse::TypeKind::String => Type::Scalar(Scalar::String),
            crate::parse::TypeKind::UnsignedInt16 => Type::Scalar(Scalar::UnsignedInt16),
            crate::parse::TypeKind::UnsignedInt32 => Type::Scalar(Scalar::UnsignedInt32),
            crate::parse::TypeKind::UnsignedInt64 => Type::Scalar(Scalar::UnsignedInt64),
            crate::parse::TypeKind::UnsignedInt8 => Type::Scalar(Scalar::UnsignedInt8),
            crate::parse::TypeKind::Array(typ, size) => {
                Type::Array(Box::new(Type::from(typ.as_ref())), *size)
            }
            crate::parse::TypeKind::Map(k, v) => Type::Map(
                Box::new(Type::from(k.as_ref())),
                Box::new(Type::from(v.as_ref())),
            ),
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                Enum: Scalar                                */
/* -------------------------------------------------------------------------- */

#[allow(dead_code)]
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Scalar {
    #[display("bit")]
    Bit,
    #[display("bool")]
    Bool,
    #[display("byte")]
    Byte,
    #[display("f32")]
    Float32,
    #[display("f64")]
    Float64,
    #[display("i16")]
    SignedInt16,
    #[display("i32")]
    SignedInt32,
    #[display("i64")]
    SignedInt64,
    #[display("i8")]
    SignedInt8,
    #[display("string")]
    String,
    #[display("u16")]
    UnsignedInt16,
    #[display("u32")]
    UnsignedInt32,
    #[display("u64")]
    UnsignedInt64,
    #[display("u8")]
    UnsignedInt8,
}
