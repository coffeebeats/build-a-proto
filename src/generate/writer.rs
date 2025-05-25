use anyhow::anyhow;
use derive_builder::Builder;

#[allow(dead_code)]
pub trait Writer {
    fn newline(&mut self) -> anyhow::Result<()>;
    fn indent(&mut self) -> anyhow::Result<()>;
    fn outdent(&mut self) -> anyhow::Result<()>;
    fn write(&mut self, input: &str) -> anyhow::Result<()>;

    fn comment(&mut self, input: &str) -> anyhow::Result<()>;
}

#[derive(Builder, Clone, Default)]
pub struct StringWriter {
    #[builder(default = format!("//"))]
    pub comment_token: String,
    #[builder(default = format!("  "))]
    pub indent_token: String,
    #[builder(default = format!("\n"))]
    pub newline_token: String,

    contents: String,
    indented: usize,
}

impl Writer for StringWriter {
    fn newline(&mut self) -> anyhow::Result<()> {
        self.write(&self.newline_token.clone())?;
        self.write(&self.indent_token.repeat(self.indented))?;

        Ok(())
    }

    fn indent(&mut self) -> anyhow::Result<()> {
        self.indented += 1;

        Ok(())
    }

    fn outdent(&mut self) -> anyhow::Result<()> {
        if self.indented == 0 {
            return Err(anyhow!("cannot outdent further"));
        }

        self.indented -= 1;

        Ok(())
    }

    fn write(&mut self, input: &str) -> anyhow::Result<()> {
        self.contents.push_str(input);
        Ok(())
    }

    fn comment(&mut self, input: &str) -> anyhow::Result<()> {
        self.write(&format!("{} {}", self.comment_token, input))?;
        self.newline()?;
        self.write(&self.indent_token.repeat(self.indented))?;

        Ok(())
    }
}
