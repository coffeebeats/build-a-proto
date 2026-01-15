use crate::analyze::Diagnostic;
use crate::compile::SourceCache;

/* -------------------------------------------------------------------------- */
/*                         Struct: DiagnosticReporter                         */
/* -------------------------------------------------------------------------- */

/// `DiagnosticReporter` renders diagnostics as rich, colorized error messages
/// with source code context using the ariadne library.
pub struct DiagnosticReporter<'a> {
    sources: &'a crate::compile::SourceCache,
}

/* ------------------------ Impl: DiagnosticReporter ------------------------ */

impl<'a> DiagnosticReporter<'a> {
    /// `new` constructs a new [`DiagnosticReporter`] using the provided
    /// [`SourceCache`] for looking up source code for error spans.
    pub fn new(sources: &'a SourceCache) -> Self {
        Self { sources }
    }

    /// `report` prints information about the provided diagnostic to
    /// [`std::io::stderr`].
    pub fn report(&self, diagnostic: &Diagnostic) {
        self.write(diagnostic, std::io::stderr());
    }

    /// `write` reports information about the provided diagnostic to the
    /// provided [`std::io::Write`] implementer, assuming the output ultimately
    /// ends up routing to [`std::io::stderr`].
    pub fn write<W>(&self, diagnostic: &Diagnostic, writer: W)
    where
        W: std::io::Write,
    {
        let import = &diagnostic.span.context;
        let location = import.to_string();

        let source = self
            .sources
            .read(import)
            .expect("missing source file in cache");

        let range = diagnostic.span.start..diagnostic.span.end;

        ariadne::Report::build(
            diagnostic.severity.into(),
            (location.clone(), range.clone()),
        )
        .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
        .with_message(&diagnostic.message)
        .with_label(
            ariadne::Label::new((location.clone(), range))
                .with_message(&diagnostic.message)
                .with_color(diagnostic.severity.into()),
        )
        .finish()
        .write(
            (location.clone(), ariadne::Source::from(source.as_ref())),
            writer,
        )
        .unwrap();
    }
}
