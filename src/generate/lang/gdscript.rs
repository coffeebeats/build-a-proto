use std::path::PathBuf;

use crate::core::Type;
use crate::generate::CodeWriter;
use crate::generate::CodeWriterBuilder;
use crate::generate::Described;
use crate::generate::Generator;
use crate::generate::Writer;

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
        (_, module): (&crate::core::Descriptor, &mut crate::core::Module),
        w: W,
    ) -> anyhow::Result<(PathBuf, W)> {
        let path = out_dir.join(module.package[0].clone()).with_extension("gd");

        // HACK: This path is used to look up a module's writer; update it
        // here so that it matches the target path.
        module.path = path.to_owned();
        let w = w.configured(module)?;

        Ok((path, w))
    }

    fn gen_begin(
        &mut self,
        _: &crate::core::Registry,
        _: Vec<(&std::path::PathBuf, &mut W)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn gen_end(
        &mut self,
        _registry: &crate::core::Registry,
        _writers: Vec<(&std::path::PathBuf, &mut W)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn mod_begin(
        &mut self,
        _: &crate::core::Registry,
        _: Described<&crate::core::Module>,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.write(w, "extends RefCounted")?;
        self.0.newline(w)?;
        self.0.newline(w)?;

        Ok(())
    }

    fn mod_end(
        &mut self,
        _: &crate::core::Registry,
        _: Described<&crate::core::Module>,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.newline(w)?;

        Ok(())
    }

    fn gen_include(
        &mut self,
        _: &crate::core::Registry,
        (desc, m): Described<&crate::core::Module>,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.write(
            w,
            &format!(
                "const {} := preload(\"{}\")",
                desc.package.first().unwrap(),
                PathBuf::from(m.package[0].clone())
                    .with_extension("gd")
                    .to_str()
                    .unwrap(),
            ),
        )?;
        self.0.newline(w)?;
        self.0.newline(w)?;

        Ok(())
    }

    fn gen_msg_begin(
        &mut self,
        _: &crate::core::Registry,
        (_, msg): Described<&crate::core::Message>,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.write(w, &format!("class {}:", msg.name))?;
        self.0.newline(w)?;

        self.0.indent()?;
        w.write(&self.0.get_indent())?;

        self.0.write(w, "extends RefCounted")?;
        self.0.newline(w)?;

        Ok(())
    }

    fn gen_msg_end(
        &mut self,
        _: &crate::core::Registry,
        _: Described<&crate::core::Message>,
        _: &mut W,
    ) -> anyhow::Result<()> {
        self.0.outdent()?;

        Ok(())
    }

    fn gen_enum_begin(
        &mut self,
        _: &crate::core::Registry,
        (_, enm): Described<&crate::core::Enum>,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.write(w, &format!("class {}:", enm.name))?;
        self.0.newline(w)?;

        self.0.indent()?;
        w.write(&self.0.get_indent())?;

        self.0.write(w, "extends RefCounted")?;
        self.0.newline(w)?;

        Ok(())
    }

    fn gen_enum_end(
        &mut self,
        _: &crate::core::Registry,
        _: Described<&crate::core::Enum>,
        _: &mut W,
    ) -> anyhow::Result<()> {
        self.0.outdent()?;

        Ok(())
    }

    fn gen_field(
        &mut self,
        _: &crate::core::Registry,
        f: &crate::core::Field,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.newline(w)?;

        for line in &f.comment {
            self.0.comment(w, line)?;
        }

        self.0.write(w, &format!("var {}: ", f.name))?;

        fn render_type(typ: &Type) -> String {
            match typ {
                crate::core::Type::Array(t, _) => format!("Array[{}]", &render_type(t)),
                crate::core::Type::Map(k, v) => format!("Dictionary[{}, {}]", k, v),
                crate::core::Type::Reference(s) => s.to_owned(),
                crate::core::Type::Scalar(scalar) => match scalar {
                    crate::core::Scalar::Bit => "bool",
                    crate::core::Scalar::Bool => "bool",
                    crate::core::Scalar::Byte => "int",
                    crate::core::Scalar::Float32 => "float",
                    crate::core::Scalar::Float64 => "float",
                    crate::core::Scalar::SignedInt16 => "int",
                    crate::core::Scalar::SignedInt32 => "int",
                    crate::core::Scalar::SignedInt64 => "int",
                    crate::core::Scalar::SignedInt8 => "int",
                    crate::core::Scalar::String => "String",
                    crate::core::Scalar::UnsignedInt16 => "int",
                    crate::core::Scalar::UnsignedInt32 => "int",
                    crate::core::Scalar::UnsignedInt64 => "int",
                    crate::core::Scalar::UnsignedInt8 => "int",
                }
                .to_owned(),
            }
        }

        self.0.write(w, &format!("{}", render_type(&f.typ)))?;
        self.0.newline(w)?;

        Ok(())
    }

    fn gen_variant(
        &mut self,
        _: &crate::core::Registry,
        v: &crate::core::Variant,
        w: &mut W,
    ) -> anyhow::Result<()> {
        self.0.newline(w)?;

        for line in &v.comment {
            self.0.comment(w, line)?;
        }
        self.0.write(w, &format!("var {}: bool", v.name))?;
        self.0.newline(w)?;

        Ok(())
    }
}
