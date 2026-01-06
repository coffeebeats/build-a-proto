//! Writer abstractions for code generation output.
//!
//! The [`Writer`] trait provides a simple abstraction over output destinations,
//! enabling both file I/O and in-memory string building with the same interface.

/* -------------------------------------------------------------------------- */
/*                                Trait: Writer                               */
/* -------------------------------------------------------------------------- */

/// An output destination for generated code.
///
/// Writers accumulate text through [`write`](Writer::write) calls and produce
/// the final output via [`finish`](Writer::finish).
///
/// # Implementations
///
/// - [`StringWriter`] - In-memory buffer for testing
/// - [`crate::generate::FileWriter`] - File I/O for production
pub trait Writer: Default {
    /// Appends text to the output.
    fn write(&mut self, text: &str) -> anyhow::Result<()>;

    /// Consumes the writer and returns the accumulated output.
    fn finish(self) -> anyhow::Result<String>;
}

/* -------------------------------------------------------------------------- */
/*                            Struct: StringWriter                            */
/* -------------------------------------------------------------------------- */

/// An in-memory writer for testing.
///
/// Accumulates all written text into a `String` buffer.
///
/// # Example
///
/// ```rust
/// let mut w = StringWriter::default();
/// w.write("hello").unwrap();
/// w.write(" world").unwrap();
/// assert_eq!(w.finish().unwrap(), "hello world");
/// ```
#[derive(Clone, Debug, Default)]
pub struct StringWriter(String);

impl StringWriter {
    /// Creates a new empty `StringWriter`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the current contents.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the current length of accumulated text.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Writer for StringWriter {
    fn write(&mut self, text: &str) -> anyhow::Result<()> {
        self.0.push_str(text);
        Ok(())
    }

    fn finish(self) -> anyhow::Result<String> {
        Ok(self.0)
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Tests                                    */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_writer_basic() {
        let mut w = StringWriter::default();
        assert!(w.is_empty());
        assert_eq!(w.len(), 0);

        w.write("hello").unwrap();
        assert!(!w.is_empty());
        assert_eq!(w.len(), 5);
        assert_eq!(w.as_str(), "hello");

        w.write(" world").unwrap();
        assert_eq!(w.len(), 11);
        assert_eq!(w.as_str(), "hello world");

        assert_eq!(w.finish().unwrap(), "hello world");
    }

    #[test]
    fn test_string_writer_empty() {
        let w = StringWriter::default();
        assert_eq!(w.finish().unwrap(), "");
    }

    #[test]
    fn test_string_writer_multiline() {
        let mut w = StringWriter::default();
        w.write("line 1\n").unwrap();
        w.write("line 2\n").unwrap();
        assert_eq!(w.finish().unwrap(), "line 1\nline 2\n");
    }
}
