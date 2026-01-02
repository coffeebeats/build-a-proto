use std::path::PathBuf;

use chumsky::Parser;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use thiserror::Error;

use crate::ast;
use crate::lex::Keyword;
use crate::lex::Span;
use crate::lex::Token;
use crate::parse;

/* -------------------------------------------------------------------------- */
/*                                 Fn: package                                */
/* -------------------------------------------------------------------------- */

/// `package` creates a new [`Parser`] that parses a package declaration into an
/// [`ast::Package`], if the declaration is valid.
pub(super) fn package<'src, I>()
-> impl Parser<'src, I, ast::Package, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    parse::comment_block()
        .or_not()
        .then(
            just(Token::Keyword(Keyword::Package))
                .ignore_then(
                    parse::ident()
                        .separated_by(just(Token::Dot))
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just(Token::Semicolon)),
        )
        .map_with(|(comment, components), e| ast::Package {
            comment,
            components,
            span: e.span(),
        })
        .labelled("package")
        .boxed()
}

/* -------------------------------------------------------------------------- */
/*                                 Fn: import                                 */
/* -------------------------------------------------------------------------- */

/// `import` creates a new [`Parser`] that parses an import into an
/// [`ast::Include`], if the parsed path is valid.
pub(super) fn import<'src, I>()
-> impl Parser<'src, I, ast::Include, chumsky::extra::Err<parse::ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    just(Token::Keyword(Keyword::Include))
        .ignore_then(parse::text())
        .then_ignore(just(Token::Semicolon))
        .map_with(|t, e| (t, e.span()))
        .validate(
            |(t, span), _, emitter| match import_path().parse(&t.content).into_result() {
                Ok(path) => ast::Include { path, span },
                Err(errs) => {
                    for err in errs {
                        let msg = format!("{}", err.reason());
                        emitter.emit(Rich::custom(span, msg));
                    }

                    ast::Include {
                        path: PathBuf::default(),
                        span,
                    }
                }
            },
        )
        .labelled("include")
        .boxed()
}

/* -------------------------- Enum: ImportPathError ------------------------- */

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

/* ----------------------------- Fn: import_path ---------------------------- */

/// `import_path` creates a [`Parser`] that parses an import path string into a
/// [`PathBuf`]. Note that the parsed path is not guaranteed to exist.
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

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_rejects_dot_prefix() {
        // Given: An input with a ./ prefixed include path.
        let input = vec![
            Token::Keyword(Keyword::Include),
            Token::String("./local/file.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = import().parse(input.as_slice());

        // Then: The input has an error (relative paths with . are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::RelativeSegment(".".to_owned()).to_string()
        );
    }

    #[test]
    fn test_import_rejects_dotdot_prefix() {
        // Given: An input with a ../ prefixed include path.
        let input = vec![
            Token::Keyword(Keyword::Include),
            Token::String("../parent/file.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = import().parse(input.as_slice());

        // Then: The input has an error (relative paths with .. are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::RelativeSegment("..".to_owned()).to_string()
        );
    }

    #[test]
    fn test_import_rejects_trailing_dot() {
        // Given: An input with a '.' suffix in include path.
        let input = vec![
            Token::Keyword(Keyword::Include),
            Token::String("foo./bar.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = import().parse(input.as_slice());

        // Then: The input has an error (relative paths with . are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::TrailingDot("foo.".to_owned()).to_string()
        );
    }

    #[test]
    fn test_import_rejects_missing_extension() {
        // Given: An input with a path missing the .baproto extension.
        let input = vec![
            Token::Keyword(Keyword::Include),
            Token::String("foo/bar"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = import().parse(input.as_slice());

        // Then: The input has an error (paths without .baproto are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::InvalidExtension.to_string()
        );
    }

    #[test]
    fn test_import_rejects_wrong_extension() {
        // Given: An input with a wrong extension.
        let input = vec![
            Token::Keyword(Keyword::Include),
            Token::String("foo/bar.txt"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = import().parse(input.as_slice());

        // Then: The input has an error (paths without .baproto are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::InvalidExtension.to_string()
        );
    }

    #[test]
    fn test_import_rejects_empty_filename() {
        // Given: An input with just '.baproto' as the filename.
        let input = vec![
            Token::Keyword(Keyword::Include),
            Token::String(".baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = import().parse(input.as_slice());

        // Then: The input has an error (empty filename is rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::EmptyFilename.to_string()
        );
    }
}
