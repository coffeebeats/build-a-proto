use anyhow::anyhow;
use std::io::Write;
use std::path::Path;

use crate::generate::Writer;

/* -------------------------------------------------------------------------- */
/*                             Struct: FileWriter                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Default)]
pub struct FileWriter {
    file: Option<std::fs::File>,
}

/* ------------------------------ Impl: Writer ------------------------------ */

impl Writer for FileWriter {
    fn close(&mut self) -> anyhow::Result<()> {
        if let Some(f) = self.file.take() {
            f.sync_all().map_err(|e| anyhow!(e))?;
        }

        Ok(())
    }

    fn open<T>(&mut self, path: T) -> anyhow::Result<()>
    where
        T: AsRef<Path>,
    {
        if self.file.is_some() {
            return Err(anyhow!("file already open; try calling 'close' first"));
        }

        self.file = std::fs::File::create(path)
            .map(Some)
            .map_err(|e| anyhow!(e))?;

        Ok(())
    }

    fn write(&mut self, input: &str) -> anyhow::Result<()> {
        match &mut self.file {
            None => Err(anyhow!("missing file; try calling 'open' first")),
            Some(f) => {
                f.write_all(input.as_bytes()).map_err(|e| anyhow!(e))?;
                Ok(())
            }
        }
    }
}
