use crate::core;

use super::Field;
use super::Type;
use super::TypeKind;
use super::Variant;
use super::VariantKind;

impl From<&Type> for core::Type {
    fn from(value: &Type) -> Self {
        match &value.kind {
            TypeKind::Invalid => unreachable!(),
            TypeKind::Reference(r) => core::Type::Reference(r.clone()),
            TypeKind::Bit => core::Type::Scalar(core::Scalar::Bit),
            TypeKind::Bool => core::Type::Scalar(core::Scalar::Bool),
            TypeKind::Byte => core::Type::Scalar(core::Scalar::Byte),
            TypeKind::Float32 => core::Type::Scalar(core::Scalar::Float32),
            TypeKind::Float64 => core::Type::Scalar(core::Scalar::Float64),
            TypeKind::SignedInt16 => core::Type::Scalar(core::Scalar::SignedInt16),
            TypeKind::SignedInt32 => core::Type::Scalar(core::Scalar::SignedInt32),
            TypeKind::SignedInt64 => core::Type::Scalar(core::Scalar::SignedInt64),
            TypeKind::SignedInt8 => core::Type::Scalar(core::Scalar::SignedInt8),
            TypeKind::String => core::Type::Scalar(core::Scalar::String),
            TypeKind::UnsignedInt16 => core::Type::Scalar(core::Scalar::UnsignedInt16),
            TypeKind::UnsignedInt32 => core::Type::Scalar(core::Scalar::UnsignedInt32),
            TypeKind::UnsignedInt64 => core::Type::Scalar(core::Scalar::UnsignedInt64),
            TypeKind::UnsignedInt8 => core::Type::Scalar(core::Scalar::UnsignedInt8),
            TypeKind::Array(typ, size) => {
                core::Type::Array(Box::new(core::Type::from(typ.as_ref())), *size)
            }
            TypeKind::Map(k, v) => core::Type::Map(
                Box::new(core::Type::from(k.as_ref())),
                Box::new(core::Type::from(v.as_ref())),
            ),
        }
    }
}

impl<'a> From<Field<'a>> for core::Field {
    fn from(value: Field<'a>) -> Self {
        core::FieldBuilder::default()
            .comment(
                value
                    .comment
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .encoding(value.encoding.map(|s| s.inner))
            .name(value.name.inner)
            .index(value.index.unwrap().inner)
            .typ(core::Type::from(&value.typ))
            .build()
            .unwrap()
    }
}

impl<'a> From<Variant<'a>> for core::Variant {
    fn from(value: Variant<'a>) -> Self {
        core::VariantBuilder::default()
            .comment(
                value
                    .comment
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .name(value.name.inner)
            .build()
            .unwrap()
    }
}

impl<'a> From<VariantKind<'a>> for core::VariantKind {
    fn from(value: VariantKind<'a>) -> Self {
        match value {
            VariantKind::Field(f) => core::VariantKind::Field(core::Field::from(f)),
            VariantKind::Variant(v) => core::VariantKind::Variant(core::Variant::from(v)),
        }
    }
}
