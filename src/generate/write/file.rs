use anyhow::anyhow;
use derive_builder::Builder;
use std::path::Path;
use std::path::PathBuf;

use crate::generate::Writer;

/* -------------------------------------------------------------------------- */
/*                             Struct: FileWriter                             */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Default)]
pub struct FileWriter {
    pub path: PathBuf,
    #[builder(default)]
    pub contents: String,
}

/* ------------------------------ Impl: Writer ------------------------------ */

impl Writer for FileWriter {
    fn configured(mut self, module: &crate::core::Module) -> anyhow::Result<Self> {
        self.path = module.path.to_path_buf();
        Ok(self)
    }

    fn close(&mut self) -> anyhow::Result<()> {
        std::fs::write(self.path.as_path(), &self.contents).map_err(|e| anyhow!(e))
    }

    fn open(&mut self, _: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn write(&mut self, input: &str) -> anyhow::Result<()> {
        self.contents.push_str(input);
        Ok(())
    }
}
