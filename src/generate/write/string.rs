use std::path::Path;

use crate::generate::Writer;

/* -------------------------------------------------------------------------- */
/*                            Struct: StringWriter                            */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Debug, Default)]
#[allow(unused)]
pub struct StringWriter(String);

/* ------------------------------ Impl: Writer ------------------------------ */

impl Writer for StringWriter {
    fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn open<T>(&mut self, _: T) -> anyhow::Result<()>
    where
        T: AsRef<Path>,
    {
        Ok(())
    }

    fn write(&mut self, input: &str) -> anyhow::Result<()> {
        self.0.push_str(input);
        Ok(())
    }
}
