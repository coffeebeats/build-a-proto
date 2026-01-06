//! Language-agnostic code generation utilities.
//!
//! This module provides:
//! - [`CodeWriter`] - A formatting helper with configurable tokens and indentation
//! - [`Writer`] trait - An abstraction for output destinations

mod writer;

pub use writer::{StringWriter, Writer};

/* -------------------------------------------------------------------------- */
/*                             Struct: CodeWriter                             */
/* -------------------------------------------------------------------------- */

/// A language-agnostic code formatting helper.
///
/// `CodeWriter` manages indentation and provides configurable tokens for
/// comments, indentation, and newlines. It works with any [`Writer`] implementation.
///
/// # Design
///
/// - **Configurable tokens**: Different languages use different syntax
///   - Rust: `///` comments, 4-space indent
///   - GDScript: `##` comments, 2-space indent (Godot convention)
/// - **Indentation tracking**: `indent()`/`outdent()` manage the current level
/// - **Block helpers**: Closure-based methods ensure balanced indent/outdent
///
/// # Example
///
/// ```rust
/// use crate::generate::code::{CodeWriter, CodeWriterBuilder, StringWriter};
///
/// let mut w = StringWriter::default();
/// let mut cw = CodeWriterBuilder::default()
///     .comment_token("///".to_owned())
///     .indent_token("    ".to_owned())
///     .newline_token("\n".to_owned())
///     .build()
///     .unwrap();
///
/// cw.comment(&mut w, "A struct").unwrap();
/// cw.writeln(&mut w, "pub struct Foo {").unwrap();
/// cw.indent();
/// cw.writeln(&mut w, "value: u32,").unwrap();
/// cw.outdent();
/// cw.writeln(&mut w, "}").unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct CodeWriter {
    /// Token for doc comments (e.g., "///" for Rust, "##" for GDScript).
    pub comment_token: String,
    /// Token for one level of indentation (e.g., "    " or "\t").
    pub indent_token: String,
    /// Token for line endings (typically "\n").
    pub newline_token: String,
    /// Current indentation level.
    indent_level: usize,
}

impl CodeWriter {
    /// Creates a new `CodeWriter` with the given configuration.
    pub fn new(comment_token: String, indent_token: String, newline_token: String) -> Self {
        Self {
            comment_token,
            indent_token,
            newline_token,
            indent_level: 0,
        }
    }

    /// Creates a `CodeWriter` configured for Rust.
    pub fn rust() -> Self {
        Self::new("///".to_owned(), "    ".to_owned(), "\n".to_owned())
    }

    /// Creates a `CodeWriter` configured for GDScript.
    pub fn gdscript() -> Self {
        Self::new("##".to_owned(), "  ".to_owned(), "\n".to_owned())
    }

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
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the indent level would go below zero.
    pub fn outdent(&mut self) -> &mut Self {
        self.indent_level = self.indent_level.saturating_sub(1);
        self
    }

    /// Writes indentation at the current level.
    fn write_indent<W: Writer>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write(&self.get_indent())?;
        Ok(())
    }

    /// Writes a newline (just the newline character, no indentation).
    pub fn newline<W: Writer>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write(&self.newline_token)?;
        Ok(())
    }

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

    /// Writes text followed by a newline only (no indentation before).
    ///
    /// Useful for the first line of a file or when manual indentation control is needed.
    pub fn writeln_no_indent<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        w.write(text)?;
        w.write(&self.newline_token)?;
        Ok(())
    }

    /// Writes a doc comment line.
    ///
    /// For multi-line text, call this multiple times or use [`comment_block`].
    pub fn comment<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        self.write_indent(w)?;
        if text.is_empty() {
            w.write(&self.comment_token)?;
        } else {
            w.write(&format!("{} {}", self.comment_token, text))?;
        }
        w.write(&self.newline_token)?;
        Ok(())
    }

    /// Writes a multi-line doc comment block.
    ///
    /// Splits `text` on newlines and writes each line as a comment.
    pub fn comment_block<W: Writer>(&self, w: &mut W, text: &str) -> anyhow::Result<()> {
        for line in text.lines() {
            self.comment(w, line)?;
        }
        Ok(())
    }

    /// Writes an optional doc comment.
    ///
    /// Does nothing if `text` is `None`.
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

    /// Executes a closure with increased indentation.
    ///
    /// Automatically increments indent before and decrements after.
    /// Guarantees balanced indentation even if the closure returns an error.
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

    /// Writes a block with header and footer lines, indenting the body.
    ///
    /// # Arguments
    ///
    /// * `header` - Text for the opening line (e.g., "pub struct Foo {")
    /// * `footer` - Text for the closing line (e.g., "}")
    /// * `f` - Closure that writes the block body
    ///
    /// # Example
    ///
    /// ```rust
    /// cw.block(&mut w, "impl Foo {", "}", |cw, w| {
    ///     cw.writeln(w, "fn new() -> Self { Self {} }")
    /// })?;
    /// ```
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

    /// Writes a braced block (header + " {" ... "}").
    ///
    /// Convenience wrapper for the common `{ }` block pattern.
    ///
    /// # Example
    ///
    /// ```rust
    /// cw.braced_block(&mut w, "pub struct Foo", |cw, w| {
    ///     cw.writeln(w, "value: u32,")
    /// })?;
    /// // Output: pub struct Foo {\n    value: u32,\n}\n
    /// ```
    pub fn braced_block<W, F>(&mut self, w: &mut W, header: &str, f: F) -> anyhow::Result<()>
    where
        W: Writer,
        F: FnOnce(&mut Self, &mut W) -> anyhow::Result<()>,
    {
        self.block(w, &format!("{} {{", header), "}", f)
    }
}

impl Default for CodeWriter {
    fn default() -> Self {
        Self::rust()
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Tests                                    */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_defaults() {
        let cw = CodeWriter::rust();
        assert_eq!(cw.comment_token, "///");
        assert_eq!(cw.indent_token, "    ");
        assert_eq!(cw.newline_token, "\n");
    }

    #[test]
    fn test_gdscript_defaults() {
        let cw = CodeWriter::gdscript();
        assert_eq!(cw.comment_token, "##");
        assert_eq!(cw.indent_token, "  ");
        assert_eq!(cw.newline_token, "\n");
    }

    #[test]
    fn test_indent_outdent() {
        let mut cw = CodeWriter::rust();
        assert_eq!(cw.indent_level(), 0);
        assert_eq!(cw.get_indent(), "");

        cw.indent();
        assert_eq!(cw.indent_level(), 1);
        assert_eq!(cw.get_indent(), "    ");

        cw.indent();
        assert_eq!(cw.indent_level(), 2);
        assert_eq!(cw.get_indent(), "        ");

        cw.outdent();
        assert_eq!(cw.indent_level(), 1);
        assert_eq!(cw.get_indent(), "    ");

        cw.outdent();
        assert_eq!(cw.indent_level(), 0);
        assert_eq!(cw.get_indent(), "");
    }

    #[test]
    fn test_outdent_saturates() {
        let mut cw = CodeWriter::rust();
        cw.outdent();
        assert_eq!(cw.indent_level(), 0);
    }

    #[test]
    fn test_write_and_writeln() {
        let cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.write(&mut w, "hello").unwrap();
        cw.write(&mut w, " world").unwrap();
        assert_eq!(w.finish().unwrap(), "hello world");
    }

    #[test]
    fn test_writeln() {
        let cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.writeln(&mut w, "line 1").unwrap();
        cw.writeln(&mut w, "line 2").unwrap();
        assert_eq!(w.finish().unwrap(), "line 1\nline 2\n");
    }

    #[test]
    fn test_writeln_with_indent() {
        let mut cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.writeln(&mut w, "outer").unwrap();
        cw.indent();
        cw.writeln(&mut w, "inner").unwrap();
        cw.outdent();
        cw.writeln(&mut w, "outer again").unwrap();

        assert_eq!(w.finish().unwrap(), "outer\n    inner\nouter again\n");
    }

    #[test]
    fn test_comment() {
        let cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.comment(&mut w, "A comment").unwrap();
        assert_eq!(w.finish().unwrap(), "/// A comment\n");
    }

    #[test]
    fn test_comment_empty() {
        let cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.comment(&mut w, "").unwrap();
        assert_eq!(w.finish().unwrap(), "///\n");
    }

    #[test]
    fn test_comment_block() {
        let cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.comment_block(&mut w, "Line 1\nLine 2").unwrap();
        assert_eq!(w.finish().unwrap(), "/// Line 1\n/// Line 2\n");
    }

    #[test]
    fn test_indented() {
        let mut cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.writeln(&mut w, "outer").unwrap();
        cw.indented(&mut w, |cw, w| cw.writeln(w, "inner")).unwrap();
        cw.writeln_no_indent(&mut w, "outer").unwrap();

        assert_eq!(w.finish().unwrap(), "outer\n    inner\nouter\n");
    }

    #[test]
    fn test_block() {
        let mut cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.block(&mut w, "if true {", "}", |cw, w| {
            cw.writeln(w, "do_something();")
        })
        .unwrap();

        assert_eq!(w.finish().unwrap(), "if true {\n    do_something();\n}\n");
    }

    #[test]
    fn test_braced_block() {
        let mut cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.braced_block(&mut w, "struct Foo", |cw, w| cw.writeln(w, "x: u32,"))
            .unwrap();

        assert_eq!(w.finish().unwrap(), "struct Foo {\n    x: u32,\n}\n");
    }

    #[test]
    fn test_nested_blocks() {
        let mut cw = CodeWriter::rust();
        let mut w = StringWriter::default();

        cw.braced_block(&mut w, "mod outer", |cw, w| {
            cw.braced_block(w, "mod inner", |cw, w| cw.writeln(w, "const X: u32 = 1;"))
        })
        .unwrap();

        let expected = "mod outer {\n    mod inner {\n        const X: u32 = 1;\n    }\n}\n";
        assert_eq!(w.finish().unwrap(), expected);
    }

    #[test]
    fn test_gdscript_output() {
        let mut cw = CodeWriter::gdscript();
        let mut w = StringWriter::default();

        cw.comment(&mut w, "A class").unwrap();
        cw.writeln(&mut w, "class Foo:").unwrap();
        cw.indent();
        cw.writeln(&mut w, "extends RefCounted").unwrap();
        cw.writeln(&mut w, "var x: int").unwrap();
        cw.outdent();

        let expected = "## A class\nclass Foo:\n  extends RefCounted\n  var x: int\n";
        assert_eq!(w.finish().unwrap(), expected);
    }
}
