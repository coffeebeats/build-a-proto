use derive_builder::Builder;

use super::Writer;

/* -------------------------------------------------------------------------- */
/*                             Struct: CodeWriter                             */
/* -------------------------------------------------------------------------- */

/// `CodeWriter` is a utility that assists with generating source code of an
/// arbitrary syntax.
#[derive(Builder, Clone, Debug, Default)]
pub struct CodeWriter {
    pub comment_token: String,
    pub indent_token: String,
    pub newline_token: String,

    #[builder(default)]
    indent_level: usize,
}

/* ---------------------------- Impl: CodeWriter ---------------------------- */

impl CodeWriter {
    #[allow(unused)]
    /// Returns the current indentation level.
    pub fn indent_level(&self) -> usize {
        self.indent_level
    }

    /// Returns the current indentation string.
    pub fn get_indent(&self) -> String {
        self.indent_token.repeat(self.indent_level)
    }

    /// Increases the indentation level by one.
    pub fn indent(&mut self) -> &mut Self {
        self.indent_level += 1;
        self
    }

    /// Decreases the indentation level by one.
    pub fn outdent(&mut self) -> &mut Self {
        debug_assert!(self.indent_level > 0, "cannot outdent below zero");
        self.indent_level = self.indent_level.saturating_sub(1);
        self
    }

    /// Writes indentation at the current level.
    fn write_indent<W: Writer>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write(self.get_indent())?;
        Ok(())
    }

    #[allow(unused)]
    /// Writes a newline (just the newline character, no indentation).
    pub fn newline<W: Writer>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write(&self.newline_token)?;
        Ok(())
    }

    #[allow(unused)]
    /// Writes text without a trailing newline.
    pub fn write<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        w.write(text)
    }

    /// Writes indentation, then text, then a newline.
    pub fn writeln<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        self.write_indent(w)?;
        w.write(text)?;
        w.write(&self.newline_token)?;
        Ok(())
    }

    #[allow(unused)]
    /// Writes text followed by a newline only (no indentation before).
    pub fn writeln_no_indent<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        w.write(text)?;
        w.write(&self.newline_token)?;
        Ok(())
    }

    /// Writes a doc comment line.
    pub fn comment<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        self.write_indent(w)?;
        if text.is_empty() {
            w.write(&self.comment_token)?;
        } else {
            w.write(format!("{} {}", self.comment_token, text))?;
        }
        w.write(&self.newline_token)?;
        Ok(())
    }

    /// Writes a multi-line doc comment block.
    pub fn comment_block<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        for line in text.lines() {
            self.comment(w, line)?;
        }
        Ok(())
    }

    /// Writes an optional doc comment.
    pub fn comment_opt<W: Writer>(&self, w: &mut W, text: Option<&str>) -> anyhow::Result<()> {
        if let Some(text) = text {
            self.comment_block(w, text)?;
        }
        Ok(())
    }

    /// Writes a blank line (newline without indentation).
    pub fn blank_line<W: Writer>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write(&self.newline_token)?;
        Ok(())
    }

    #[allow(unused)]
    /// Executes a closure with increased indentation.
    pub fn indented<W, F>(&mut self, w: &mut W, f: F) -> anyhow::Result<()>
    where
        W: Writer,
        F: FnOnce(&mut Self, &mut W) -> anyhow::Result<()>,
    {
        self.indent();
        let result = f(self, w);
        self.outdent();
        result
    }

    #[allow(unused)]
    /// Writes a block with header and footer lines, indenting the body.
    pub fn block<W, F>(&mut self, w: &mut W, header: &str, footer: &str, f: F) -> anyhow::Result<()>
    where
        W: Writer,
        F: FnOnce(&mut Self, &mut W) -> anyhow::Result<()>,
    {
        self.writeln(w, header)?;
        self.indented(w, f)?;
        self.writeln(w, footer)?;
        Ok(())
    }

    #[allow(unused)]
    /// Writes a braced block (header + " {" ... "}").
    pub fn braced_block<W, F>(&mut self, w: &mut W, header: &str, f: F) -> anyhow::Result<()>
    where
        W: Writer,
        F: FnOnce(&mut Self, &mut W) -> anyhow::Result<()>,
    {
        self.block(w, &format!("{} {{", header), "}", f)
    }
}
