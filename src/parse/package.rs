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
                        emitter.emit(Rich::custom(span.clone(), msg));
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
    use crate::parse::tests::*;

    /* --------------------------- Tests: package --------------------------- */

    #[test]
    fn test_package_single_component_succeeds() {
        // Given: A simple package declaration.
        let input = "package foo;";

        // When: The input is parsed.
        let package = assert_parse_succeeds(parse_single(input, package()));

        // Then: The package has one component.
        assert_eq!(package.components.len(), 1);
        assert_eq!(package.components[0].name, "foo");
        assert!(package.comment.is_none());
    }

    #[test]
    fn test_package_multiple_components_succeeds() {
        // Given: A package with dot-separated components.
        let input = "package foo.bar.baz;";

        // When: The input is parsed.
        let package = assert_parse_succeeds(parse_single(input, package()));

        // Then: All components are present.
        assert_eq!(package.components.len(), 3);
        assert_eq!(package.components[0].name, "foo");
        assert_eq!(package.components[1].name, "bar");
        assert_eq!(package.components[2].name, "baz");
    }

    #[test]
    fn test_package_with_doc_comment_succeeds() {
        // Given: A package with a preceding doc comment.
        let input = "// Package comment\npackage foo.bar;";

        // When: The input is parsed.
        let package = assert_parse_succeeds(parse_single(input, package()));

        // Then: The comment is captured.
        assert!(package.comment.is_some());
        let comment = package.comment.as_ref().unwrap();
        assert_eq!(comment.comments.len(), 1);
        assert_eq!(comment.comments[0].content, "Package comment");
    }

    #[test]
    fn test_package_without_semicolon_fails() {
        // Given: A package declaration missing the semicolon.
        let input = "package foo";

        // When: The input is parsed.
        assert_parse_fails::<ast::Package>(parse_single(input, package()));
    }

    /* ---------------------------- Tests: import --------------------------- */

    #[test]
    fn test_import_valid_path_succeeds() {
        // Given: A valid import statement.
        let input = "include \"foo/bar.baproto\";";

        // When: The input is parsed.
        let import = assert_parse_succeeds(parse_single(input, import()));

        // Then: The path is correct.
        assert_eq!(import.path.to_str().unwrap(), "foo/bar.baproto");
    }

    #[test]
    fn test_import_rejects_dot_prefix() {
        // Given: An import with a ./ prefixed path.
        let input = "include \"./local/file.baproto\";";

        // When: The input is parsed.
        let (import, errors) = parse_single(input, import());

        // Then: Parsing reports an error.
        assert!(!errors.is_empty());

        // Then: The error mentions relative segments.
        let error_msg = format!("{:?}", errors[0]);
        assert!(error_msg.contains("relative segment"));

        // Then: A default import is still returned for error recovery.
        assert!(import.is_some());
    }

    #[test]
    fn test_import_rejects_dotdot_prefix() {
        // Given: An import with a ../ prefixed path.
        let input = "include \"../parent/file.baproto\";";

        // When: The input is parsed.
        let (import, errors) = parse_single(input, import());

        // Then: Parsing reports an error.
        assert!(!errors.is_empty());
        assert!(import.is_some()); // Error recovery
    }

    #[test]
    fn test_import_rejects_trailing_dot() {
        // Given: An import with a trailing dot in a segment.
        let input = "include \"foo./bar.baproto\";";

        // When: The input is parsed.
        let (import, errors) = parse_single(input, import());

        // Then: Parsing reports an error.
        assert!(!errors.is_empty());
        assert!(import.is_some());
    }

    #[test]
    fn test_import_rejects_missing_extension() {
        // Given: An import path missing the .baproto extension.
        let input = "include \"foo/bar\";";

        // When: The input is parsed.
        let (import, errors) = parse_single(input, import());

        // Then: Parsing reports an error.
        assert!(!errors.is_empty());
        assert!(import.is_some());
    }

    #[test]
    fn test_import_rejects_wrong_extension() {
        // Given: An import with a wrong extension.
        let input = "include \"foo/bar.txt\";";

        // When: The input is parsed.
        let (import, errors) = parse_single(input, import());

        // Then: Parsing reports an error.
        assert!(!errors.is_empty());
        assert!(import.is_some());
    }

    #[test]
    fn test_import_rejects_empty_filename() {
        // Given: An import with just '.baproto' as the filename.
        let input = "include \".baproto\";";

        // When: The input is parsed.
        let (import, errors) = parse_single(input, import());

        // Then: Parsing reports an error.
        assert!(!errors.is_empty());
        assert!(import.is_some());
    }
}
