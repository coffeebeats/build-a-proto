//! Name resolution pass.
//!
//! This pass resolves type references to descriptors.

use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::FieldEntry;
use crate::analyze::FilePass;
use crate::analyze::ResolvedType;
use crate::analyze::TypeKind;
use crate::analyze::VariantEntry;
use crate::analyze::VariantKind as ResolvedVariantKind;
use crate::ast::Enum;
use crate::ast::Item;
use crate::ast::Message;
use crate::ast::Type;
use crate::ast::TypeKind as AstTypeKind;
use crate::ast::VariantKind as AstVariantKind;
use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::PackageName;
use crate::core::SchemaImport;

/* -------------------------------------------------------------------------- */
/*                             Struct: Resolution                             */
/* -------------------------------------------------------------------------- */

/// Name resolution pass.
///
/// Resolves type references to descriptors and populates field/variant entries
/// in the symbol table.
pub struct Resolution;

/* ----------------------------- Impl: FilePass ----------------------------- */

impl FilePass for Resolution {
    fn run(&self, ctx: &mut Context, file: &SchemaImport) {
        let ast = match ctx.source_files.get(file) {
            Some(ast) => ast.clone(),
            None => return,
        };

        let package = ast.package.name.clone();

        for item in &ast.items {
            match item {
                Item::Message(msg) => {
                    let desc = build_descriptor(&package, &[], &msg.name.name);
                    resolve_message(ctx, file, &package, &[], msg, &desc);
                }
                Item::Enum(enm) => {
                    let desc = build_descriptor(&package, &[], &enm.name.name);
                    resolve_enum(ctx, file, enm, &desc);
                }
            }
        }
    }
}

/* --------------------------- Fn: resolve_message -------------------------- */

fn resolve_message(
    ctx: &mut Context,
    file: &SchemaImport,
    package: &PackageName,
    path: &[&str],
    msg: &Message,
    msg_desc: &Descriptor,
) {
    let mut fields = Vec::new();
    let mut nested_descs = Vec::new();

    for field in &msg.fields {
        let resolved_type = resolve_type(ctx, file, msg_desc, &field.typ);
        let encoding = field
            .encoding
            .as_ref()
            .map(|enc| enc.encodings.clone());

        fields.push(FieldEntry {
            name: field.name.name.clone(),
            index: field.index.value,
            resolved_type,
            encoding,
            span: field.span,
        });
    }

    for nested_msg in &msg.nested_messages {
        let desc = build_descriptor(package, path, &nested_msg.name.name);
        nested_descs.push(desc.clone());

        let mut nested_path: Vec<&str> = path.to_vec();
        nested_path.push(&msg.name.name);
        resolve_message(ctx, file, package, &nested_path, nested_msg, &desc);
    }

    for nested_enm in &msg.nested_enums {
        let desc = build_descriptor(package, path, &nested_enm.name.name);
        nested_descs.push(desc.clone());
        resolve_enum(ctx, file, nested_enm, &desc);
    }

    if let Some(entry) = ctx.symbols.get_type_mut(msg_desc) {
        entry.kind = TypeKind::Message {
            fields,
            nested: nested_descs,
        };
    }
}

/* ---------------------------- Fn: resolve_enum ---------------------------- */

fn resolve_enum(ctx: &mut Context, file: &SchemaImport, enm: &Enum, enm_desc: &Descriptor) {
    let mut variants = Vec::new();

    for variant in &enm.variants {
        let (name, kind) = match &variant.kind {
            AstVariantKind::Unit(ident) => (ident.name.clone(), ResolvedVariantKind::Unit),
            AstVariantKind::Field(field) => {
                let resolved_type = resolve_type(ctx, file, enm_desc, &field.typ);
                let encoding = field
                    .encoding
                    .as_ref()
                    .map(|enc| enc.encodings.clone());

                let field_entry = FieldEntry {
                    name: field.name.name.clone(),
                    index: field.index.value,
                    resolved_type,
                    encoding,
                    span: field.span,
                };

                (field.name.name.clone(), ResolvedVariantKind::Field(field_entry))
            }
        };

        variants.push(VariantEntry {
            name,
            index: variant.index.value,
            kind,
            span: variant.span,
        });
    }

    if let Some(entry) = ctx.symbols.get_type_mut(enm_desc) {
        entry.kind = TypeKind::Enum { variants };
    }
}

/* ----------------------------- Fn: resolve_type --------------------------- */

fn resolve_type(
    ctx: &mut Context,
    file: &SchemaImport,
    scope: &Descriptor,
    typ: &Type,
) -> ResolvedType {
    match &typ.kind {
        AstTypeKind::Invalid => ResolvedType::Unresolved("invalid".to_string()),

        AstTypeKind::Scalar(scalar) => ResolvedType::Scalar(scalar.clone()),

        AstTypeKind::Reference(reference) => match ctx.symbols.find(scope, reference) {
            Some(entry) => ResolvedType::Named(entry.descriptor.clone()),
            None => {
                ctx.add_error(Error {
                    file: file.clone(),
                    span: typ.span,
                    kind: ErrorKind::UnresolvedReference(reference.to_string()),
                });
                ResolvedType::Unresolved(reference.to_string())
            }
        },

        AstTypeKind::Array { element, size } => {
            let resolved_element = resolve_type(ctx, file, scope, element);
            ResolvedType::Array {
                element: Box::new(resolved_element),
                size: *size,
            }
        }

        AstTypeKind::Map { key, value } => {
            let resolved_key = resolve_type(ctx, file, scope, key);
            let resolved_value = resolve_type(ctx, file, scope, value);
            ResolvedType::Map {
                key: Box::new(resolved_key),
                value: Box::new(resolved_value),
            }
        }
    }
}

/* -------------------------- Fn: build_descriptor -------------------------- */

fn build_descriptor(package: &PackageName, path: &[&str], name: &str) -> Descriptor {
    DescriptorBuilder::default()
        .package(package.clone())
        .path(path.iter().map(|s| s.to_string()).collect())
        .name(name.to_owned())
        .build()
        .expect("descriptor should be valid")
}