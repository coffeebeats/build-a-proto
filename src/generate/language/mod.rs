use std::path::Path;
use std::path::PathBuf;

use crate::core::PackageName;
use crate::ir;

use super::Writer;

/* -------------------------------- Mod: Rust ------------------------------- */

mod rust;
pub use rust::*;

/* -------------------------------------------------------------------------- */
/*                               Trait: Language                              */
/* -------------------------------------------------------------------------- */

pub trait Language<W>
where
    W: Writer,
{
    /// Returns the file path for a package (relative to output directory).
    fn configure_writer(&self, out_dir: &Path, pkg: &ir::Package) -> anyhow::Result<PathBuf>;

    /// Schema-level hooks
    fn gen_begin(&mut self, schema: &ir::Schema, w: Vec<(&PathBuf, &mut W)>) -> anyhow::Result<()>;
    fn gen_end(&mut self, schema: &ir::Schema, w: Vec<(&PathBuf, &mut W)>) -> anyhow::Result<()>;

    /// Package-level hooks
    fn pkg_begin(
        &mut self,
        schema: &ir::Schema,
        pkg: &ir::Package,
        w: &mut W,
    ) -> anyhow::Result<()>;
    fn pkg_end(&mut self, schema: &ir::Schema, pkg: &ir::Package, w: &mut W) -> anyhow::Result<()>;

    /// Cross-package import/include generation
    fn gen_include(
        &mut self,
        schema: &ir::Schema,
        pkg: &ir::Package,
        w: &mut W,
    ) -> anyhow::Result<()>;

    /// Message hooks
    fn gen_msg_begin(
        &mut self,
        schema: &ir::Schema,
        msg: &ir::Message,
        w: &mut W,
    ) -> anyhow::Result<()>;
    fn gen_msg_end(
        &mut self,
        schema: &ir::Schema,
        msg: &ir::Message,
        w: &mut W,
    ) -> anyhow::Result<()>;

    /// Enum hooks
    fn gen_enum_begin(
        &mut self,
        schema: &ir::Schema,
        e: &ir::Enum,
        w: &mut W,
    ) -> anyhow::Result<()>;
    fn gen_enum_end(&mut self, schema: &ir::Schema, e: &ir::Enum, w: &mut W) -> anyhow::Result<()>;

    /// Field/variant hooks
    fn gen_field(
        &mut self,
        schema: &ir::Schema,
        field: &ir::Field,
        current_pkg: &PackageName,
        w: &mut W,
    ) -> anyhow::Result<()>;
    fn gen_variant(
        &mut self,
        schema: &ir::Schema,
        variant: &ir::Variant,
        current_pkg: &PackageName,
        w: &mut W,
    ) -> anyhow::Result<()>;

    /// Generate a complete package (default implementation provided)
    fn gen_pkg(&mut self, schema: &ir::Schema, pkg: &ir::Package, w: &mut W) -> anyhow::Result<()> {
        self.pkg_begin(schema, pkg, w)?;

        // Generate includes for cross-package references
        for dep_pkg in find_package_dependencies(schema, pkg) {
            self.gen_include(schema, dep_pkg, w)?;
        }

        // Generate top-level enums first
        for e in &pkg.enums {
            self.gen_enum(schema, e, &pkg.name, w)?;
        }

        // Generate top-level messages
        for msg in &pkg.messages {
            self.gen_msg(schema, msg, &pkg.name, w)?;
        }

        self.pkg_end(schema, pkg, w)?;
        Ok(())
    }

    /// Generate a message (default implementation provided)
    fn gen_msg(
        &mut self,
        schema: &ir::Schema,
        msg: &ir::Message,
        current_pkg: &PackageName,
        w: &mut W,
    ) -> anyhow::Result<()> {
        // Generate nested types BEFORE parent (for Rust)
        for e in &msg.enums {
            self.gen_enum(schema, e, current_pkg, w)?;
        }

        for nested in &msg.messages {
            self.gen_msg(schema, nested, current_pkg, w)?;
        }

        self.gen_msg_begin(schema, msg, w)?;

        for field in &msg.fields {
            self.gen_field(schema, field, current_pkg, w)?;
        }

        self.gen_msg_end(schema, msg, w)?;
        Ok(())
    }

    /// Generate an enum (default implementation provided)
    fn gen_enum(
        &mut self,
        schema: &ir::Schema,
        e: &ir::Enum,
        current_pkg: &PackageName,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.gen_enum_begin(schema, e, w)?;

        for variant in &e.variants {
            self.gen_variant(schema, variant, current_pkg, w)?;
        }

        self.gen_enum_end(schema, e, w)?;
        Ok(())
    }
}

/* -------------------------------------------------------------------------- */
/*                          Fn: find_package_dependencies                     */
/* -------------------------------------------------------------------------- */

/// Helper to find cross-package dependencies.
/// Scans all messages/enums in pkg for references to other packages.
#[allow(dead_code)]
fn find_package_dependencies<'a>(
    schema: &'a ir::Schema,
    pkg: &ir::Package,
) -> Vec<&'a ir::Package> {
    use std::collections::HashSet;

    let mut deps = HashSet::new();

    fn scan_field(field: &ir::Field, current_pkg: &PackageName, deps: &mut HashSet<PackageName>) {
        scan_native_type(&field.encoding.native, current_pkg, deps);
    }

    fn scan_native_type(
        native: &ir::NativeType,
        _current_pkg: &PackageName,
        deps: &mut HashSet<PackageName>,
    ) {
        match native {
            ir::NativeType::Message { descriptor } | ir::NativeType::Enum { descriptor } => {
                deps.insert(descriptor.package.clone());
            }
            ir::NativeType::Array { element } => {
                scan_native_type(&element.native, _current_pkg, deps);
            }
            ir::NativeType::Map { key, value } => {
                scan_native_type(&key.native, _current_pkg, deps);
                scan_native_type(&value.native, _current_pkg, deps);
            }
            _ => {}
        }
    }

    fn scan_message(msg: &ir::Message, current_pkg: &PackageName, deps: &mut HashSet<PackageName>) {
        for field in &msg.fields {
            scan_field(field, current_pkg, deps);
        }
        for nested in &msg.messages {
            scan_message(nested, current_pkg, deps);
        }
        for enm in &msg.enums {
            scan_enum(enm, current_pkg, deps);
        }
    }

    fn scan_enum(enm: &ir::Enum, current_pkg: &PackageName, deps: &mut HashSet<PackageName>) {
        for variant in &enm.variants {
            if let ir::Variant::Field { field, .. } = variant {
                scan_field(field, current_pkg, deps);
            }
        }
    }

    // Scan all top-level types in the package
    for msg in &pkg.messages {
        scan_message(msg, &pkg.name, &mut deps);
    }

    for enm in &pkg.enums {
        scan_enum(enm, &pkg.name, &mut deps);
    }

    // Look up packages from schema
    deps.into_iter()
        .filter_map(|dep| schema.packages.iter().find(|p| p.name == dep))
        .collect()
}
