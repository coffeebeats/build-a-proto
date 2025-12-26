use anyhow::anyhow;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::core::Descriptor;
use crate::core::Enum;
use crate::core::Field;
use crate::core::Message;
use crate::core::Module;
use crate::core::Registry;
use crate::core::Variant;

use super::Writer;

/* -------------------------------------------------------------------------- */
/*                                Fn: generate                                */
/* -------------------------------------------------------------------------- */

#[allow(dead_code, unused_variables)]
pub fn generate<P: AsRef<Path>, W: Writer, G: Generator<W>>(
    out_dir: P,
    r: &mut Registry,
    g: &mut G,
) -> anyhow::Result<()> {
    let out_dir = out_dir.as_ref().to_path_buf();
    if !out_dir.is_dir() {
        std::fs::create_dir_all(&out_dir)?;
    }

    let mut writers = HashMap::<PathBuf, W>::default();

    for (d, m) in r.iter_modules_mut() {
        let w = W::default().configured(m)?;

        let (path, mut w) = g.configure_writer(&out_dir, (d, m), w)?;

        w.open(&path)?;

        writers.insert(path.clone(), w);
    }

    fn gen_enum<W: Writer, G: Generator<W>>(
        r: &Registry,
        (desc, enm): Described<&Enum>,
        g: &mut G,
        w: &mut W,
    ) -> anyhow::Result<()> {
        g.gen_enum_begin(r, (desc, enm), w)?;

        for kind in &enm.variants {
            match kind {
                crate::core::VariantKind::Field(f) => g.gen_field(r, f, w)?,
                crate::core::VariantKind::Variant(v) => g.gen_variant(r, v, w)?,
            };
        }

        g.gen_enum_end(r, (desc, enm), w)?;

        Ok(())
    }

    fn gen_msg<W: Writer, G: Generator<W>>(
        r: &Registry,
        (desc, msg): Described<&Message>,
        g: &mut G,
        w: &mut W,
    ) -> anyhow::Result<()> {
        g.gen_msg_begin(r, (desc, msg), w)?;

        for d in &msg.enums {
            let m = r.get_enum(d).ok_or(anyhow!("missing enum: {}", d))?;
            gen_enum(r, (d, m), g, w)?;
        }

        for d in &msg.messages {
            let m = r.get_message(d).ok_or(anyhow!("missing message: {}", d))?;
            gen_msg(r, (d, m), g, w)?;
        }

        for f in &msg.fields {
            g.gen_field(r, f, w)?;
        }

        g.gen_msg_end(r, (desc, msg), w)?;

        Ok(())
    }

    g.gen_begin(r, writers.iter_mut().collect())?;

    for (d, m) in r.iter_modules() {
        let w = writers
            .get_mut(&m.path)
            .ok_or(anyhow!("missing module: {}", d))?;

        g.mod_begin(r, (d, m), w)?;

        for dep_path in &m.deps {
            let (dep_desc, dep) = r
                .get_module_by_path(dep_path.as_path())
                .ok_or(anyhow!("missing module: {:?}", dep_path))?;
            g.gen_include(r, (dep_desc, dep), w)?;
        }

        for d in &m.enums {
            let enm = r.get_enum(d).ok_or(anyhow!("missing enum: {}", d))?;
            gen_enum(r, (d, enm), g, w)?;
        }

        for d in &m.messages {
            let msg = r.get_message(d).ok_or(anyhow!("missing message: {}", d))?;
            gen_msg(r, (d, msg), g, w)?;
        }

        g.mod_end(r, (d, m), w)?;
    }

    g.gen_end(r, writers.iter_mut().collect())?;

    for (_, mut w) in writers {
        w.close()?;
    }

    Ok(())
}

/* ----------------------------- Type: Described ---------------------------- */

pub type Described<'a, T> = (&'a Descriptor, T);

/* -------------------------------------------------------------------------- */
/*                              Trait: Generator                              */
/* -------------------------------------------------------------------------- */

pub trait Generator<W>
where
    W: Writer,
{
    fn configure_writer(
        &self,
        out_dir: &Path,
        module: (&Descriptor, &mut Module),
        writer: W,
    ) -> anyhow::Result<(PathBuf, W)>;

    fn gen_begin(&mut self, r: &Registry, w: Vec<(&PathBuf, &mut W)>) -> anyhow::Result<()>;
    fn gen_end(&mut self, r: &Registry, w: Vec<(&PathBuf, &mut W)>) -> anyhow::Result<()>;

    fn mod_begin(&mut self, r: &Registry, m: Described<&Module>, w: &mut W) -> anyhow::Result<()>;
    fn mod_end(&mut self, r: &Registry, m: Described<&Module>, w: &mut W) -> anyhow::Result<()>;

    fn gen_include(&mut self, r: &Registry, m: Described<&Module>, w: &mut W)
    -> anyhow::Result<()>;

    fn gen_msg_begin(
        &mut self,
        r: &Registry,
        m: Described<&Message>,
        w: &mut W,
    ) -> anyhow::Result<()>;
    fn gen_msg_end(
        &mut self,
        r: &Registry,
        m: Described<&Message>,
        w: &mut W,
    ) -> anyhow::Result<()>;

    fn gen_enum_begin(
        &mut self,
        r: &Registry,
        e: Described<&Enum>,
        w: &mut W,
    ) -> anyhow::Result<()>;
    fn gen_enum_end(&mut self, r: &Registry, e: Described<&Enum>, w: &mut W) -> anyhow::Result<()>;

    fn gen_field(&mut self, r: &Registry, f: &Field, w: &mut W) -> anyhow::Result<()>;
    fn gen_variant(&mut self, r: &Registry, v: &Variant, w: &mut W) -> anyhow::Result<()>;
}
