use std::path::Path;

use crate::core::SchemaImport;
use crate::generate::Writer;

/* -------------------------------------------------------------------------- */
/*                            Struct: StringWriter                            */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Debug, Default)]
#[allow(unused)]
pub struct StringWriter(String);

/* ------------------------------ Impl: Writer ------------------------------ */

impl Writer for StringWriter {
    fn configured(self, _: &SchemaImport) -> anyhow::Result<Self> {
        Ok(self)
    }

    fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn open(&mut self, _: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn write(&mut self, input: &str) -> anyhow::Result<()> {
        self.0.push_str(input);
        Ok(())
    }
}
