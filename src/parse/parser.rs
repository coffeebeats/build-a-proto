use std::collections::HashSet;
use std::path::PathBuf;

use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use thiserror::Error;

use crate::ast;
use crate::core::PackageName;
use crate::core::Reference;

use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Spanned;
use crate::lex::Token;
use crate::lex::spanned;

/* -------------------------------------------------------------------------- */
/*                            Enum: ImportPathError                           */
/* -------------------------------------------------------------------------- */

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ImportPathError {
    #[error("relative segment '{0}' not allowed in import path")]
    RelativeSegment(String),

    #[error("import path must end with '.baproto'")]
    InvalidExtension,

    #[error("path segment cannot end with '.': {0}")]
    TrailingDot(String),

    #[error("filename cannot be just '.baproto'")]
    EmptyFilename,
}

/* -------------------------------------------------------------------------- */
/*                             Struct: ParseResult                            */
/* -------------------------------------------------------------------------- */

/// `ParseResult` contains the result of parsing a `.baproto` file.
#[derive(Debug)]
pub struct ParseResult<'src> {
    /// The parsed AST, if parsing succeeded (possibly with recovered errors).
    pub ast: Option<ast::Schema>,
    /// Errors encountered during parsing.
    pub errors: Vec<ParseError<'src>>,
}

/* -------------------------------------------------------------------------- */
/*                                  Fn: Parse                                 */
/* -------------------------------------------------------------------------- */

/// ParseError is a type alias for errors emitted during parsing.
pub type ParseError<'src> = Rich<'src, Token<'src>, Span>;

/// `parse` parses an input [`Token`] sequence into an [`ast::SourceFile`].
pub fn parse<'src>(input: &'src Vec<Spanned<Token<'src>>>, size: usize) -> ParseResult<'src> {
    let (ast, errors) = parser()
        .parse(input.as_slice().map(Span::from(size..size), |spanned| {
            (&spanned.inner, &spanned.span)
        }))
        .into_output_errors();

    ParseResult {
        ast: ast.flatten(),
        errors,
    }
}

/* ------------------------------- Fn: parser ------------------------------- */

/// [parser] creates a parser which parses an input [`Token`] slice into an
/// optional [`ast::SourceFile`]. Returns `None` for empty/missing input.
fn parser<'src, I>() -> impl Parser<'src, I, Option<ast::Schema>, extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    empty().map(|_| None) // TODO: Replace this with new parsing implementation.
}

/* ------------------------- Struct: MessageBodyItem ------------------------ */

#[derive(Clone, Debug, PartialEq)]
enum MessageBodyItem {
    Field(ast::Field),
    Message(ast::Message),
    Enum(ast::Enum),
}

/* ----------------------------- Fn: import_path ---------------------------- */

/// Parses an import path string into a [`PathBuf`].
fn import_path<'a>() -> impl Parser<'a, &'a str, PathBuf, extra::Err<Rich<'a, char>>> {
    let character =
        any().filter(|c: &char| c.is_ascii_alphanumeric() || ['_', '-', '.'].contains(c));

    let segment = character
        .repeated()
        .at_least(1)
        .collect::<String>()
        .try_map(|s, span| {
            if s == "." || s == ".." {
                Err(Rich::custom(span, ImportPathError::RelativeSegment(s)))
            } else if s.ends_with(".") {
                Err(Rich::custom(span, ImportPathError::TrailingDot(s)))
            } else {
                Ok(s)
            }
        });

    segment
        .separated_by(just('/'))
        .at_least(1)
        .collect::<Vec<String>>()
        .try_map(|segments, span| {
            let last = segments.last().unwrap();

            if !last.ends_with(".baproto") {
                return Err(Rich::custom(span, ImportPathError::InvalidExtension));
            }

            if last == ".baproto" {
                return Err(Rich::custom(span, ImportPathError::EmptyFilename));
            }

            Ok(segments.into_iter().collect::<PathBuf>())
        })
        .then_ignore(end())
}
