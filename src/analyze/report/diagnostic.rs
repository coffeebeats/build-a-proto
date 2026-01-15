use derive_more::Display;

use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                             Struct: Diagnostic                             */
/* -------------------------------------------------------------------------- */

/// `Diagnostic` represents a compilation error, warning, or informational
/// message with source location information.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

/* ---------------------------- Impl: Diagnostic ---------------------------- */

impl Diagnostic {
    /// `error` creates a new error [`Diagnostic`].
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: Severity::Error,
            span,
        }
    }

    #[allow(unused)]
    /// `warning` creates a new warning [`Diagnostic`].
    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: Severity::Warning,
            span,
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                               Enum: Severity                               */
/* -------------------------------------------------------------------------- */

/// `Severity` indicates the importance level of a diagnostic message.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
pub enum Severity {
    #[display("error")]
    Error,

    #[display("warning")]
    Warning,
}

/* ----------------------- Impl: Into<ariadne::Color> ----------------------- */

impl From<Severity> for ariadne::Color {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::Red,
            Severity::Warning => Self::Yellow,
        }
    }
}

/* --------------------- Impl: Into<ariadne::ReportKind> -------------------- */

impl<'a> From<Severity> for ariadne::ReportKind<'a> {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::Error,
            Severity::Warning => Self::Warning,
        }
    }
}
