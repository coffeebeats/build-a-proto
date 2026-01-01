use crate::analyze::Context;
use crate::analyze::Error;
use crate::analyze::ErrorKind;
use crate::analyze::ModuleEntry;
use crate::analyze::MultiFilePass;
use crate::analyze::TypeEntry;
use crate::analyze::TypeKind;
use crate::ast::Enum;
use crate::ast::Item;
use crate::ast::Message;
use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::PackageName;
use crate::core::SchemaImport;

/* -------------------------------------------------------------------------- */
/*                            Struct: Registration                            */
/* -------------------------------------------------------------------------- */

/// `Registration` defines the type registration pass.
///
/// Walks all source files and registers types in the symbol table, detecting
/// duplicate type definitions.
pub struct Registration;

/* --------------------------- Impl: MultiFilePass -------------------------- */

impl MultiFilePass for Registration {
    fn run(&self, ctx: &mut Context) {
        let files: Vec<_> = ctx
            .source_files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (import, ast) in files {
            let package = &ast.package.name;
            let mut type_descriptors = Vec::new();

            for item in &ast.items {
                match item {
                    Item::Message(msg) => {
                        let desc = build_descriptor(package, &[], &msg.name.name);
                        register_type(ctx, &import, package, desc.clone(), msg, &mut type_descriptors);
                    }
                    Item::Enum(enm) => {
                        let desc = build_descriptor(package, &[], &enm.name.name);
                        register_enum(ctx, &import, desc.clone(), enm, &mut type_descriptors);
                    }
                }
            }

            ctx.symbols.register_module(
                import.clone(),
                ModuleEntry {
                    package: package.clone(),
                    deps: Vec::new(), // TODO: resolve include paths
                    types: type_descriptors,
                },
            );
        }
    }
}

/* ----------------------------- Fn: register_type -------------------------- */

fn register_type(
    ctx: &mut Context,
    import: &SchemaImport,
    package: &PackageName,
    descriptor: Descriptor,
    msg: &Message,
    type_descriptors: &mut Vec<Descriptor>,
) {
    let entry = TypeEntry {
        descriptor: descriptor.clone(),
        kind: TypeKind::Message {
            fields: Vec::new(), // Populated in resolution pass
            nested: Vec::new(), // Populated below
        },
        span: msg.span,
        source: import.clone(),
    };

    if ctx.symbols.register_type(entry).is_some() {
        ctx.add_error(Error {
            file: import.clone(),
            span: msg.span,
            kind: ErrorKind::DuplicateType(descriptor.to_string()),
        });
    }

    type_descriptors.push(descriptor.clone());

    let path: Vec<&str> = vec![msg.name.name.as_str()];
    register_nested_types(ctx, import, package, &path, msg, type_descriptors);
}

/* --------------------------- Fn: register_enum ---------------------------- */

fn register_enum(
    ctx: &mut Context,
    import: &SchemaImport,
    descriptor: Descriptor,
    enm: &Enum,
    type_descriptors: &mut Vec<Descriptor>,
) {
    let entry = TypeEntry {
        descriptor: descriptor.clone(),
        kind: TypeKind::Enum {
            variants: Vec::new(), // Populated in resolution pass
        },
        span: enm.span,
        source: import.clone(),
    };

    if ctx.symbols.register_type(entry).is_some() {
        ctx.add_error(Error {
            file: import.clone(),
            span: enm.span,
            kind: ErrorKind::DuplicateType(descriptor.to_string()),
        });
    }

    type_descriptors.push(descriptor);
}

/* ----------------------- Fn: register_nested_types ------------------------ */

fn register_nested_types(
    ctx: &mut Context,
    import: &SchemaImport,
    package: &PackageName,
    path: &[&str],
    msg: &Message,
    type_descriptors: &mut Vec<Descriptor>,
) {
    for enm in &msg.nested_enums {
        let desc = build_descriptor(package, path, &enm.name.name);
        register_enum(ctx, import, desc, enm, type_descriptors);
    }

    for nested_msg in &msg.nested_messages {
        let desc = build_descriptor(package, path, &nested_msg.name.name);

        let entry = TypeEntry {
            descriptor: desc.clone(),
            kind: TypeKind::Message {
                fields: Vec::new(),
                nested: Vec::new(),
            },
            span: nested_msg.span,
            source: import.clone(),
        };

        if ctx.symbols.register_type(entry).is_some() {
            ctx.add_error(Error {
                file: import.clone(),
                span: nested_msg.span,
                kind: ErrorKind::DuplicateType(desc.to_string()),
            });
        }

        type_descriptors.push(desc);

        let mut nested_path: Vec<&str> = path.to_vec();
        nested_path.push(&nested_msg.name.name);
        register_nested_types(ctx, import, package, &nested_path, nested_msg, type_descriptors);
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