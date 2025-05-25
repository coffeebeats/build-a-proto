use chumsky::error::Rich;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use crate::core::Descriptor;
use crate::core::DescriptorBuilder;
use crate::core::EnumBuilder;
use crate::core::Field;
use crate::core::MessageBuilder;
use crate::core::Registry;
use crate::core::VariantKind;
use crate::core::registry;
use crate::parse::Expr;
use crate::parse::ParseError;
use crate::parse::Span;

/* -------------------------------------------------------------------------- */
/*                                 Fn: Prepare                                */
/* -------------------------------------------------------------------------- */

pub fn prepare<'a, P: AsRef<Path>>(
    path: &'a P,
    registry: &'a mut Registry,
    exprs: Vec<(Expr<'a>, Span)>,
) -> Result<(), ParseError<'a>> {
    let mut enums: Vec<crate::parse::Enum> = vec![];
    let mut messages: Vec<crate::parse::Message> = vec![];

    let mut module = crate::core::Module::new(path.as_ref().to_path_buf());

    // First, inspect all expressions so all definitions can be registered.
    for (expr, span) in exprs {
        match expr {
            Expr::Comment(_) => {} // Skip
            Expr::Enum(enm) => enums.push(enm),
            Expr::Message(msg) => messages.push(msg),
            Expr::Package(pkg) => {
                // FIXME: This should happen during parsing.
                module.package = pkg.split(".").map(str::to_owned).collect();
            }
            Expr::Include(include) => {
                // HACK: This is a simple enough way to get around the fact that
                // other modules don't exist yet and we don't have access to a
                // cross-module cache. A second pass will be required to properly
                // link modules in the [`Registry`].
                module.deps.push(
                    DescriptorBuilder::default()
                        .name(
                            parse_include_path(path.as_ref().to_str().unwrap(), include, span)?
                                .to_str()
                                .unwrap(),
                        )
                        .build()
                        .unwrap(),
                )
            }
            _ => unreachable!(),
        }
    }

    let package = &module.package;

    fn register_enm(
        registry: &mut Registry,
        scope: Descriptor,
        mut enm: crate::parse::Enum,
    ) -> Descriptor {
        debug_assert!(scope.name.is_none());

        enm.variants.sort_by(|l, r| {
            let l = match l {
                crate::parse::VariantKind::Field(field) => field.index,
                crate::parse::VariantKind::Variant(variant) => variant.index,
            };
            let r = match r {
                crate::parse::VariantKind::Field(field) => field.index,
                crate::parse::VariantKind::Variant(variant) => variant.index,
            };

            l.cmp(&r)
        });

        let d = DescriptorBuilder::default()
            .package(scope.package)
            .path(scope.path)
            .name(enm.name.to_owned())
            .build()
            .unwrap();

        let e = EnumBuilder::default()
            .comment(
                enm.comment
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .name(enm.name)
            .variants(enm.variants.into_iter().map(VariantKind::from).collect())
            .build()
            .unwrap();

        registry.insert(d.clone(), registry::Kind::Enum(e));

        d
    }

    fn register_msg(
        registry: &mut Registry,
        scope: Descriptor,
        mut msg: crate::parse::Message,
    ) -> Descriptor {
        debug_assert!(scope.name.is_none());

        msg.fields.sort_by(|l, r| l.index.cmp(&r.index));

        let d = DescriptorBuilder::default()
            .package(scope.package.clone())
            .path(scope.path.clone())
            .name(msg.name.to_owned())
            .build()
            .unwrap();

        let mut m = MessageBuilder::default()
            .comment(
                msg.comment
                    .unwrap_or_default()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .name(msg.name)
            .fields(msg.fields.into_iter().map(Field::from).collect())
            .build()
            .unwrap();

        let mut scope = scope.clone();
        scope.path.push(msg.name.to_owned());

        m.enums = msg
            .enums
            .into_iter()
            .map(|enm| register_enm(registry, scope.clone(), enm))
            .collect();

        m.messages = msg
            .messages
            .into_iter()
            .map(|m| register_msg(registry, scope.clone(), m))
            .collect();

        registry.insert(d.clone(), registry::Kind::Message(m));

        d
    }

    let scope = DescriptorBuilder::default()
        .package(package.clone())
        .build()
        .unwrap();

    module.enums = enums
        .into_iter()
        .map(|enm| register_enm(registry, scope.clone(), enm))
        .collect();

    module.messages = messages
        .into_iter()
        .map(|msg| register_msg(registry, scope.clone(), msg))
        .collect();

    registry.insert(scope, registry::Kind::Module(module));

    Ok(())
}

/* ------------------------- Fn: parse_include_path ------------------------- */

fn parse_include_path<'a>(
    src_path: &str,
    dep_path: &str,
    span: Span,
) -> Result<PathBuf, ParseError<'a>> {
    match PathBuf::from_str(dep_path) {
        Err(err) => {
            Err(Rich::custom(
                span,
                format!("{}: invalid include path: {}", err, dep_path),
            ))
        }
        Ok(mut p) => {
            debug_assert!(p.to_str().is_some());

            if p.is_relative() {
                debug_assert!(PathBuf::from_str(src_path).is_ok());

                if let Some(parent) = PathBuf::from_str(src_path).unwrap().parent() {
                    let mut path = PathBuf::default();
                    path.push(parent);
                    path.push(p);
                    p = path.to_path_buf();
                }
            }

            if p.to_str() == Some(src_path) {
                return Err(Rich::custom(
                    span,
                    format!(
                        "dependency cycle: invalid include path '{:?}' from file: {:?}",
                        dep_path, src_path,
                    ),
                ));
            }

            Ok(p)
        }
    }
}
