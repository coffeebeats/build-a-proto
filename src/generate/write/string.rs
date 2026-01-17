use std::path::Path;

use crate::generate::Writer;

/* -------------------------------------------------------------------------- */
/*                            Struct: StringWriter                            */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Debug, Default)]
pub struct StringWriter(String);

/* ---------------------------- Impl: StringWriter -------------------------- */

impl StringWriter {
    /// Consumes the [`Writer`] and returns the accumulated content.
    #[allow(unused)]
    pub fn into_content(self) -> String {
        self.0
    }
}

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

    fn write<T: AsRef<str>>(&mut self, input: T) -> anyhow::Result<()> {
        self.0.push_str(input.as_ref());
        Ok(())
    }
}
