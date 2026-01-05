use crate::analyze::Diagnostic;

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
    /// Creates a new diagnostic reporter with the given source cache.
    pub fn new(sources: &'a crate::compile::SourceCache) -> Self {
        Self { sources }
    }

    /// `report` prints information about the provided diagnostic.
    pub fn report(&self, diagnostic: &Diagnostic) {
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
        .eprint((location.clone(), ariadne::Source::from(source.as_ref())))
        .unwrap();
    }
}
