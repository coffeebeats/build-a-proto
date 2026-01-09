use std::path::PathBuf;

use crate::generate::generator::Generator;
use crate::generate::{CodeWriter, CodeWriterBuilder, Writer};
use crate::ir;

/* -------------------------------------------------------------------------- */
/*                              Struct: GDScript                              */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Debug)]
pub struct GDScript(CodeWriter);

/* ------------------------------ Impl: Default ----------------------------- */

impl Default for GDScript {
    fn default() -> Self {
        Self(
            CodeWriterBuilder::default()
                .comment_token("##".to_owned())
                .indent_token("  ".to_owned())
                .newline_token("\n".to_owned())
                .build()
                .unwrap(),
        )
    }
}

/* ----------------------------- Impl: Generator ---------------------------- */

impl<W: Writer> Generator<W> for GDScript {
    fn configure_writer(
        &self,
        out_dir: &std::path::Path,
        pkg: &ir::Package,
    ) -> anyhow::Result<PathBuf> {
        // Get the last segment of the package path for the filename
        let parts: Vec<&str> = pkg.path.split('.').collect();
        let filename = parts.last().copied().unwrap_or(&pkg.path);
        let path = out_dir.join(filename).with_extension("gd");
        Ok(path)
    }

    fn gen_begin(
        &mut self,
        _: &ir::Schema,
        _: Vec<(&std::path::PathBuf, &mut W)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn gen_end(
        &mut self,
        _: &ir::Schema,
        _: Vec<(&std::path::PathBuf, &mut W)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn pkg_begin(
        &mut self,
        _: &ir::Schema,
        _: &ir::Package,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.writeln_no_indent(w, "extends RefCounted")?;
        self.0.blank_line(w)?;
        Ok(())
    }

    fn pkg_end(&mut self, _: &ir::Schema, _: &ir::Package, w: &mut W) -> anyhow::Result<()> {
        self.0.blank_line(w)?;
        Ok(())
    }

    fn gen_include(
        &mut self,
        _: &ir::Schema,
        dep_pkg: &ir::Package,
        w: &mut W,
    ) -> anyhow::Result<()> {
        let parts: Vec<&str> = dep_pkg.path.split('.').collect();
        let dep_name = parts.last().copied().unwrap_or(&dep_pkg.path);
        let dep_file = PathBuf::from(dep_name).with_extension("gd");

        self.0.writeln_no_indent(
            w,
            &format!(
                "const {} := preload(\"{}\")",
                dep_name.to_uppercase(),
                dep_file.to_str().unwrap(),
            ),
        )?;
        self.0.blank_line(w)?;
        Ok(())
    }

    fn gen_msg_begin(
        &mut self,
        _: &ir::Schema,
        msg: &ir::Message,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.comment_opt(w, msg.doc.as_deref())?;
        self.0.writeln_no_indent(w, &format!("class {}:", msg.name))?;
        self.0.indent();
        self.0.writeln(w, "extends RefCounted")?;
        self.0.blank_line(w)?;
        Ok(())
    }

    fn gen_msg_end(&mut self, _: &ir::Schema, _: &ir::Message, w: &mut W) -> anyhow::Result<()> {
        self.0.outdent();
        self.0.blank_line(w)?;
        Ok(())
    }

    fn gen_enum_begin(
        &mut self,
        _: &ir::Schema,
        e: &ir::Enum,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.comment_opt(w, e.doc.as_deref())?;
        self.0.writeln_no_indent(w, &format!("enum {}:", e.name))?;
        self.0.indent();
        Ok(())
    }

    fn gen_enum_end(&mut self, _: &ir::Schema, _: &ir::Enum, w: &mut W) -> anyhow::Result<()> {
        self.0.outdent();
        self.0.blank_line(w)?;
        Ok(())
    }

    fn gen_field(
        &mut self,
        _: &ir::Schema,
        field: &ir::Field,
        current_pkg: &str,
        w: &mut W,
    ) -> anyhow::Result<()> {
        let type_name = self.type_name(&field.encoding.native, current_pkg);
        self.0.comment_opt(w, field.doc.as_deref())?;
        self.0.writeln(w, &format!("var {}: {}", field.name, type_name))?;
        Ok(())
    }

    fn gen_variant(
        &mut self,
        _: &ir::Schema,
        variant: &ir::Variant,
        _current_pkg: &str,
        w: &mut W,
    ) -> anyhow::Result<()> {
        match variant {
            ir::Variant::Unit { name, index, doc, .. } => {
                self.0.comment_opt(w, doc.as_deref())?;
                self.0.writeln(w, &format!("{} = {}", name, index))?;
            }
            ir::Variant::Field { name, index, doc, .. } => {
                // GDScript doesn't support variant fields, treat as unit
                self.0.comment_opt(w, doc.as_deref())?;
                self.0.writeln(w, &format!("{} = {}", name, index))?;
            }
        }
        Ok(())
    }
}

/* ----------------------------- Impl: GDScript ----------------------------- */

impl GDScript {
    fn type_name(&self, native: &ir::NativeType, _current_pkg: &str) -> String {
        match native {
            ir::NativeType::Bool => "bool".to_owned(),
            ir::NativeType::Int { .. } => "int".to_owned(),
            ir::NativeType::Float { .. } => "float".to_owned(),
            ir::NativeType::String => "String".to_owned(),
            ir::NativeType::Bytes => "PackedByteArray".to_owned(),
            ir::NativeType::Array { element } => {
                let inner = self.type_name(&element.native, _current_pkg);
                format!("Array[{}]", inner)
            }
            ir::NativeType::Map { .. } => "Dictionary".to_owned(),
            ir::NativeType::Message { descriptor } | ir::NativeType::Enum { descriptor } => {
                // Extract simple name from descriptor
                descriptor.split('.').last().unwrap_or(descriptor).to_owned()
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                              Function: gdscript                            */
/* -------------------------------------------------------------------------- */

pub fn gdscript<W: Writer>() -> impl Generator<W> {
    GDScript::default()
}
