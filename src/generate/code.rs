use anyhow::anyhow;
use derive_builder::Builder;

use super::Writer;

/* -------------------------------------------------------------------------- */
/*                             Struct: CodeWriter                             */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Default)]
pub struct CodeWriter {
    pub comment_token: String,
    pub indent_token: String,
    pub newline_token: String,

    #[builder(default)]
    indented: usize,
}

/* ---------------------------- Impl: CodeWriter ---------------------------- */

impl CodeWriter {
    pub fn newline<W: Writer>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write(&self.newline_token)?;
        writer.write(&self.get_indent())?;

        Ok(())
    }

    pub fn get_indent(&self) -> String {
        self.indent_token.repeat(self.indented)
    }

    pub fn indent(&mut self) -> anyhow::Result<()> {
        self.indented += 1;

        Ok(())
    }

    pub fn outdent(&mut self) -> anyhow::Result<()> {
        if self.indented == 0 {
            return Err(anyhow!("cannot outdent further"));
        }

        self.indented -= 1;

        Ok(())
    }

    pub fn comment<W: Writer>(&mut self, writer: &mut W, input: &str) -> anyhow::Result<()> {
        writer.write(&format!("{} {}", self.comment_token, input))?;
        self.newline(writer)?;
        // writer.write(&self.indent_token.repeat(self.indented))?;

        Ok(())
    }

    pub fn write<W: Writer>(&self, writer: &mut W, input: &str) -> anyhow::Result<()> {
        writer.write(input)
    }
}
