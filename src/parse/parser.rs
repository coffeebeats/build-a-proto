use std::collections::HashSet;
use std::path::PathBuf;

use chumsky::Parser;
use chumsky::extra::ParserExtra;
use chumsky::input::Emitter;
use chumsky::input::MapExtra;
use chumsky::input::ValueInput;
use chumsky::prelude::*;
use chumsky::text::ascii::ident;
use thiserror::Error;

use crate::core::Encoding;
use crate::syntax::PackageNameError;
use crate::syntax::Reference;

use super::Enum;
use super::Expr;
use super::Field;
use super::Keyword;
use super::Message;
use super::Span;
use super::Spanned;
use super::Token;
use super::Type;
use super::TypeKind;
use super::Variant;
use super::VariantKind;

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
/*                                  Fn: Parse                                 */
/* -------------------------------------------------------------------------- */

/// ParseError is a type alias for errors emitted during parsing.
pub type ParseError<'src> = Rich<'src, Token<'src>, Span>;

/// `parse` parses an input [`Token`] sequence into [`Expr`]s recognized by the
/// compiler.
pub fn parse<'src>(
    input: &'src Vec<Spanned<Token<'src>>>,
    size: usize,
) -> (Option<Vec<Spanned<Expr<'src>>>>, Vec<ParseError<'src>>) {
    parser()
        .parse(input.as_slice().map(Span::from(size..size), |spanned| {
            (&spanned.node, &spanned.span)
        }))
        .into_output_errors()
}

/* ------------------------------- Fn: parser ------------------------------- */

/// [parser] creates a parser which parses an input [`Token`] slice into a
/// sequence of [`Expr`]s.
fn parser<'src, I>() -> impl Parser<'src, I, Vec<Spanned<Expr<'src>>>, extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let ident = select! { Token::Ident(id) => id };
    let string = select! { Token::String(s) => s };
    let uint = select! { Token::Uint(n) => n };

    let package = just(Token::Keyword(Keyword::Package))
        .ignore_then(string)
        .then_ignore(just(Token::Semicolon))
        .map_with(|s: &str, e| (s, e.span()))
        .validate(
            |(s, span), _, emitter| match package_name().parse(s).into_result() {
                Ok(segments) => segments,
                Err(errs) => {
                    for err in errs {
                        let msg = format!("{}", err.reason());
                        emitter.emit(Rich::custom(span, msg));
                    }

                    vec![]
                }
            },
        )
        .map(Expr::Package)
        .map_with(Expr::with_span)
        .labelled("package")
        .boxed();

    let include = just(Token::Keyword(Keyword::Include))
        .ignore_then(string)
        .then_ignore(just(Token::Semicolon))
        .map_with(|s: &str, e| (s, e.span()))
        .validate(
            |(s, span), _, emitter| match import_path().parse(s).into_result() {
                Ok(path) => path,
                Err(errs) => {
                    for err in errs {
                        let msg = format!("{}", err.reason());
                        emitter.emit(Rich::custom(span, msg));
                    }

                    PathBuf::new()
                }
            },
        )
        .map(Expr::Include)
        .map_with(Expr::with_span)
        .labelled("include")
        .boxed();

    // Comments

    let comment = select! { Token::Comment(c) => c };
    let inline_comment = just(Token::Newline)
        .not()
        .ignore_then(comment)
        .then_ignore(just(Token::Newline))
        .map(Expr::Comment)
        .labelled("inline comment");

    let line_comment = comment
        .then_ignore(just(Token::Newline))
        .map(Expr::Comment)
        .labelled("line comment");
    let doc_comment = comment
        .map(|c| vec![c])
        .foldl(
            just(Token::Newline).ignore_then(comment).repeated(),
            |mut v, c| {
                v.push(c);
                v
            },
        )
        .then_ignore(just(Token::Newline))
        .labelled("doc comment");

    // Types

    let reference = just(Token::Dot)
        .or_not()
        .then(
            ident
                // TODO: Add support for segment-specific error-reporting/spans.
                // .map_with(|id, e| Spanned::new(id, e.span()))
                .separated_by(just(Token::Dot))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map_with(|(leading_dot, segments), e| (leading_dot, segments, e.span()))
        .validate(|(leading_dot, segments, span), _, emitter| {
            let absolute = leading_dot.is_some();
            let name = segments.last().unwrap();
            let path: Vec<_> = segments.iter().take(segments.len() - 1).copied().collect();

            let result = if absolute {
                Reference::try_new_absolute(path, name)
            } else {
                Reference::try_new_relative(path, name)
            };

            match result {
                Ok(r) => TypeKind::Reference(r),
                Err(err) => {
                    emitter.emit(Rich::custom(span, format!("invalid reference: {}", err)));
                    TypeKind::Invalid
                }
            }
        })
        .map_with(|kind, e| Type {
            kind,
            span: e.span(),
        });

    let scalar = select! {
        Token::Ident("bit") => TypeKind::Bit,
        Token::Ident("bool") => TypeKind::Bool,
        Token::Ident("byte") => TypeKind::Byte,
        Token::Ident("u8") => TypeKind::UnsignedInt8,
        Token::Ident("u16") => TypeKind::UnsignedInt16,
        Token::Ident("u32") => TypeKind::UnsignedInt32,
        Token::Ident("u64") => TypeKind::UnsignedInt64,
        Token::Ident("i8") => TypeKind::SignedInt8,
        Token::Ident("i16") => TypeKind::SignedInt16,
        Token::Ident("i32") => TypeKind::SignedInt32,
        Token::Ident("i64") => TypeKind::SignedInt64,
        Token::Ident("f32") => TypeKind::Float32,
        Token::Ident("f64") => TypeKind::Float64,
        Token::Ident("string") => TypeKind::String,
    }
    .map_with(|kind, e| Type {
        kind,
        span: e.span(),
    });

    let array = uint
        .or_not()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar)
        .map_with(|(size, t), e| Type {
            kind: TypeKind::Array(Box::new(t), size),
            span: e.span(),
        });
    let map = scalar
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar)
        .map_with(|(k, v), e| Type {
            kind: TypeKind::Map(Box::new(k), Box::new(v)),
            span: e.span(),
        });

    let typ = choice((scalar, array, map, reference))
        .labelled("type")
        .boxed();

    // Definitions

    let bits = just(Token::Ident("bits"))
        .ignore_then(uint.delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(Encoding::Bits);
    let bits_variable = just(Token::Ident("bits"))
        .ignore_then(
            just(Token::Ident("var"))
                .ignore_then(uint.delimited_by(just(Token::FnOpen), just(Token::FnClose)))
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(Encoding::BitsVariable);
    let fixed_point = just(Token::Ident("fixed_point"))
        .ignore_then(
            uint.separated_by(just(Token::Comma))
                .exactly(2)
                .collect()
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(|args: Vec<usize>| Encoding::FixedPoint(args[0], args[1]));

    let delta = just(Token::Ident("delta")).map(|_| Encoding::Delta);
    let zig_zag = just(Token::Ident("zig_zag")).map(|_| Encoding::ZigZag);
    let pad = just(Token::Ident("pad"))
        .ignore_then(uint.delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(Encoding::Pad);

    let encoding = choice((
        // Sizing
        bits,
        bits_variable,
        fixed_point,
        // Encodings
        delta,
        zig_zag,
        pad,
    ))
    .labelled("encoding")
    .boxed();

    let variant = (doc_comment.clone().or_not())
        .then(
            uint.then_ignore(just(Token::Colon))
                .map_with(|idx, e| Spanned::new(idx, e.span()))
                .or_not(),
        )
        .then(ident.map_with(|name, e| Spanned::new(name, e.span())))
        .then_ignore(just(Token::Semicolon))
        .map_with(|((comment, index), name), e| {
            Expr::Variant(Variant {
                span: e.span(),
                comment,
                index,
                name,
            })
        })
        .labelled("variant")
        .boxed();

    let field = (doc_comment.clone().or_not())
        .then(
            uint.then_ignore(just(Token::Colon))
                .map_with(|idx, e| Spanned::new(idx, e.span()))
                .or_not(),
        )
        .then(typ)
        .then(ident.map_with(|name, e| Spanned::new(name, e.span())))
        .then(
            just(Token::Equal)
                .ignore_then(choice((
                    // Single encoding
                    encoding.clone().map(|enc| vec![enc]),
                    // Multiple encodings
                    encoding
                        .separated_by(just(Token::Comma).then(just(Token::Newline).repeated()))
                        .allow_trailing()
                        .collect::<Vec<_>>()
                        .delimited_by(
                            just(Token::ListOpen).then(just(Token::Newline).repeated()),
                            just(Token::ListClose),
                        ),
                )))
                .map_with(|enc, e| Spanned::new(enc, e.span()))
                .or_not(),
        )
        .then_ignore(just(Token::Semicolon))
        .map_with(|((((comment, index), typ), name), encoding), e| {
            Expr::Field(Field {
                span: e.span(),
                comment,
                encoding,
                index,
                name,
                typ,
            })
        })
        .labelled("field")
        .boxed();

    let enumeration = doc_comment
        .clone()
        .or_not()
        .then(
            just(Token::Keyword(Keyword::Enum))
                .ignore_then(ident.map_with(|name, e| Spanned::new(name, e.span()))),
        )
        .then_ignore(
            choice((
                inline_comment.clone(),
                just(Token::Newline).map(|_| Expr::Invalid(&[])),
            ))
            .repeated(),
        )
        .then(
            choice((field.clone(), variant.clone(), line_comment.clone()))
                .delimited_by(
                    just(Token::Newline).repeated(),
                    just(Token::Newline).repeated(),
                )
                .boxed()
                .repeated()
                .collect::<Vec<Expr<'src>>>()
                .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
        )
        .then_ignore(just(Token::Newline).or_not())
        .validate(|((comment, name), mut exprs), info, emitter| {
            // TODO: Replace these with context-sensitive field parsing.
            check_field_names(&mut exprs, info, emitter);
            set_field_indices(&mut exprs, info, emitter);

            Enum {
                span: info.span(),
                comment,
                name,
                variants: exprs
                    .into_iter()
                    .filter_map(|expr| match expr {
                        Expr::Field(f) => Some(VariantKind::Field(f)),
                        Expr::Variant(v) => Some(VariantKind::Variant(v)),
                        _ => None,
                    })
                    .collect(),
            }
        })
        .map(Expr::Enum)
        .labelled("enum")
        .boxed();

    let message = recursive(|msg| {
        doc_comment
            .or_not()
            .then(
                just(Token::Keyword(Keyword::Message))
                    .ignore_then(ident.map_with(|name, e| Spanned::new(name, e.span()))),
            )
            .then_ignore(
                choice((
                    inline_comment,
                    just(Token::Newline).map(|_| Expr::Invalid(&[])),
                ))
                .repeated(),
            )
            .then(
                choice((msg, enumeration.clone(), field, line_comment.clone()))
                    .delimited_by(
                        just(Token::Newline).repeated(),
                        just(Token::Newline).repeated(),
                    )
                    .boxed()
                    .repeated()
                    .collect::<Vec<Expr<'src>>>()
                    .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
            )
            .then_ignore(just(Token::Newline).or_not())
            .validate(|((comment, name), mut exprs), info, emitter| {
                let mut enums = vec![];
                let mut fields = vec![];
                let mut messages = vec![];

                // TODO: Replace these with context-sensitive field parsing.
                check_field_names(&mut exprs, info, emitter);
                set_field_indices(&mut exprs, info, emitter);

                for expr in exprs {
                    match expr {
                        Expr::Enum(en) => enums.push(en),
                        Expr::Message(msg) => messages.push(msg),
                        Expr::Field(f) => fields.push(f),
                        Expr::Comment(_) => {} // Ignore comments!
                        _ => unreachable!(),
                    }
                }

                Message {
                    span: info.span(),
                    comment,
                    name,
                    fields,
                    enums,
                    messages,
                }
            })
            .map(Expr::Message)
            .labelled("message")
            .boxed()
    });

    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        vec![]
    });

    // Leading content: newlines and line comments before package declaration.
    let leading = choice((
        just(Token::Newline).to(None),
        line_comment.clone().map_with(Expr::with_span).map(Some),
    ))
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| v.into_iter().flatten().collect::<Vec<_>>());

    // Header: leading content, a required package declaration, and includes.
    let header = leading.then(package).then(
        choice((
            just(Token::Newline).to(None),
            include.map(Some),
            line_comment.clone().map_with(Expr::with_span).map(Some),
        ))
        .repeated()
        .collect::<Vec<_>>()
        .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
    );

    let declaration = choice((
        message.map_with(Expr::with_span),
        enumeration.map_with(Expr::with_span),
    ))
    .recover_with(skip_then_retry_until(any().ignored(), end()));

    let declarations = choice((
        just(Token::Newline).to(None),
        declaration.map(Some),
        line_comment.map_with(Expr::with_span).map(Some),
    ))
    .repeated()
    .collect::<Vec<_>>()
    .map(|v: Vec<Option<Spanned<Expr<'src>>>>| v.into_iter().flatten().collect::<Vec<_>>());

    let ast = header
        .then(declarations)
        .map(|(((leading_comments, pkg), includes), decls)| {
            let mut exprs: Vec<Spanned<Expr<'src>>> = leading_comments;
            exprs.push(pkg);
            exprs.extend(includes);
            exprs.extend(decls);
            exprs
        });

    missing.or(ast)
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

/* ----------------------------- Fn: package_name ---------------------------- */

/// Parses a package name string into a vector of segments.
fn package_name<'a>() -> impl Parser<'a, &'a str, Vec<&'a str>, extra::Err<Rich<'a, char>>> {
    let segment = ident()
        .map_with(|s: &'a str, e| Spanned::new(s, e.span()))
        .validate(|spanned, _info, emitter| {
            let s = spanned.node;

            if let Some(first) = s.chars().next() {
                if !first.is_ascii_lowercase() {
                    emitter.emit(Rich::custom(
                        spanned.span,
                        PackageNameError::InvalidStart(s.to_owned()),
                    ));
                }
            }

            if s.chars().any(|c| c.is_ascii_uppercase()) {
                emitter.emit(Rich::custom(
                    spanned.span,
                    PackageNameError::InvalidCharacters(s.to_owned()),
                ));
            }

            s
        });

    segment
        .separated_by(just('.'))
        .at_least(1)
        .collect::<Vec<_>>()
        .validate(|segments, info, emitter| {
            if segments.is_empty() {
                emitter.emit(Rich::custom(info.span(), PackageNameError::Empty));
            }

            segments
        })
        .then_ignore(end())
}

/* -------------------------- Fn: check_field_names ------------------------- */

fn check_field_names<'src, I, Ex>(
    fields: &mut Vec<Expr<'src>>,
    info: &mut MapExtra<'src, '_, I, Ex>,
    emitter: &mut Emitter<Rich<'src, Token<'src>, Span>>,
) where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
    Ex: ParserExtra<'src, I>,
{
    let mut seen = HashSet::<&'src str>::with_capacity(fields.len());

    for expr in fields.iter_mut() {
        let target: &'src str = match expr {
            Expr::Field(f) => f.name.node,
            Expr::Variant(v) => v.name.node,
            _ => continue,
        };

        if seen.contains(target) {
            emitter.emit(Rich::custom(
                info.span(),
                format!("Duplicate field name: {}", target),
            ));

            continue;
        }

        seen.insert(target);
    }
}

/* -------------------------- Fn: set_field_indices ------------------------- */

fn set_field_indices<'src, I, Ex>(
    fields: &mut Vec<Expr<'src>>,
    info: &mut MapExtra<'src, '_, I, Ex>,
    emitter: &mut Emitter<Rich<'src, Token<'src>, Span>>,
) where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
    Ex: ParserExtra<'src, I>,
{
    let mut indices: Vec<Option<()>> = fields
        .iter()
        .filter_map(|expr| match expr {
            Expr::Field(_) => Some(None),
            Expr::Variant(_) => Some(None),
            _ => None,
        })
        .collect();

    for expr in fields.iter_mut() {
        let (target, field_span): (&mut Option<Spanned<usize>>, Span) = match expr {
            Expr::Field(f) => (&mut f.index, f.span),
            Expr::Variant(v) => (&mut v.index, v.span),
            _ => continue,
        };

        match target {
            Some(spanned_index) => {
                let value = spanned_index.node;

                if value >= indices.len() {
                    emitter.emit(Rich::custom(
                        info.span(),
                        format!("Field index out of range: {}", value),
                    ));

                    return;
                }

                if indices.get(value).unwrap().is_some() {
                    emitter.emit(Rich::custom(
                        info.span(),
                        format!("Found duplicate field index: {}", value),
                    ));

                    return;
                }

                indices[value] = Some(());
            }
            None => {
                let next_index = indices.iter().position(Option::is_none);
                debug_assert!(next_index.is_some());

                if let Some(value) = next_index {
                    let _ = target.insert(Spanned::new(value, field_span));
                    indices[value] = Some(());
                }
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use crate::parse::FieldBuilder;
    use crate::parse::MessageBuilder;

    use super::*;

    #[test]
    fn test_empty_input_returns_empty_expr_list() {
        // Given: An input list of tokens.
        let input = vec![];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error.
        assert!(output.has_errors());
        assert_eq!(
            output.errors().collect::<Vec<_>>(),
            vec![&Rich::custom(Span::from(0..0), "missing input")]
        );

        // Then: The output expression list matches expectations.
        let exprs = vec![];
        assert_eq!(output.output(), Some(&exprs));
    }

    #[test]
    fn test_example_header_returns_correct_expr_list() {
        // Given: An input list of tokens.
        let input = vec![
            Token::Newline,
            Token::Keyword(Keyword::Package),
            Token::String("abc.def"),
            Token::Semicolon,
            Token::Newline,
            Token::Newline, // Two line breaks!
            Token::Keyword(Keyword::Include),
            Token::String("a/b/c.baproto"),
            Token::Semicolon,
            // No line break!
            Token::Keyword(Keyword::Include),
            Token::String("d.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output expression list matches expectations.
        let exprs = vec![
            Spanned::new(Expr::Package(vec!["abc", "def"]), Span::from(1..4)),
            Spanned::new(
                Expr::Include(PathBuf::from("a/b/c.baproto")),
                Span::from(6..9),
            ),
            Spanned::new(Expr::Include(PathBuf::from("d.baproto")), Span::from(9..12)),
        ];
        assert_eq!(output.output(), Some(&exprs));
    }

    #[test]
    fn test_include_rejects_dot_prefix() {
        // Given: An input with a ./ prefixed include path.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Include),
            Token::String("./local/file.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error (relative paths with . are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::RelativeSegment(".".to_owned()).to_string()
        );
    }

    #[test]
    fn test_include_rejects_dotdot_prefix() {
        // Given: An input with a ../ prefixed include path.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Include),
            Token::String("../parent/file.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error (relative paths with .. are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::RelativeSegment("..".to_owned()).to_string()
        );
    }

    #[test]
    fn test_include_rejects_trailing_dot() {
        // Given: An input with a '.' suffix in include path.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Include),
            Token::String("foo./bar.baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error (relative paths with . are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::TrailingDot("foo.".to_owned()).to_string()
        );
    }

    #[test]
    fn test_include_rejects_missing_extension() {
        // Given: An input with a path missing the .baproto extension.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Include),
            Token::String("foo/bar"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error (paths without .baproto are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::InvalidExtension.to_string()
        );
    }

    #[test]
    fn test_include_rejects_wrong_extension() {
        // Given: An input with a wrong extension.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Include),
            Token::String("foo/bar.txt"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error (paths without .baproto are rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::InvalidExtension.to_string()
        );
    }

    #[test]
    fn test_include_rejects_empty_filename() {
        // Given: An input with just '.baproto' as the filename.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Include),
            Token::String(".baproto"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has an error (empty filename is rejected).
        assert_eq!(output.errors().count(), 1);
        assert_eq!(
            output.into_errors().first().unwrap().reason().to_string(),
            ImportPathError::EmptyFilename.to_string()
        );
    }

    #[test]
    fn test_line_comment_in_message_returns_correct_expr_list() {
        // Given: An input list of tokens.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Comment("comment"),
            Token::Newline,
            Token::Newline,
            Token::Ident("u8"),
            Token::Ident("sequence_id"),
            Token::Semicolon,
            Token::Newline,
            Token::BlockClose,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(!output.has_errors());

        // Then: The output expression list matches expectations.
        use super::TypeKind;
        let exprs = vec![
            Spanned::new(Expr::Package(vec!["test"]), Span::from(0..3)),
            Spanned::new(
                MessageBuilder::default()
                    .span(Span::from(4..16))
                    .name(Spanned::new("Message", Span::from(5..6)))
                    .fields(vec![
                        FieldBuilder::default()
                            .span(Span::from(11..14))
                            .name(Spanned::new("sequence_id", Span::from(12..13)))
                            .typ(Type {
                                kind: TypeKind::UnsignedInt8,
                                span: Span::from(11..12),
                            })
                            .index(Spanned::new(0, Span::from(11..14)))
                            .build()
                            .unwrap(),
                    ])
                    .build()
                    .unwrap()
                    .into(),
                Span::from(4..16),
            ),
        ];
        assert_eq!(output.output(), Some(&exprs));
    }

    #[test]
    fn test_relative_type_reference_parses_correctly() {
        // Given: An input with a dot-separated relative type reference.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Ident("other"),
            Token::Dot,
            Token::Ident("package"),
            Token::Dot,
            Token::Ident("MyType"),
            Token::Ident("field_name"),
            Token::Semicolon,
            Token::Newline,
            Token::BlockClose,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(
            !output.has_errors(),
            "Errors: {:?}",
            output.errors().collect::<Vec<_>>()
        );

        let exprs = output.output().unwrap();
        let Expr::Message(msg) = &exprs[1].node else {
            panic!("Expected Message");
        };

        // Then: The field has the correct type reference.
        let TypeKind::Reference(r) = &msg.fields[0].typ.kind else {
            panic!("Expected Reference type");
        };
        assert!(!r.is_absolute(), "Expected relative reference");
        assert_eq!(r.name(), "MyType");
        assert_eq!(r.path(), vec!["other", "package"]);
    }

    #[test]
    fn test_absolute_type_reference_parses_correctly() {
        // Given: An input with a dot-prefixed absolute type reference.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::String("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Dot, // Leading dot for absolute reference
            Token::Ident("other"),
            Token::Dot,
            Token::Ident("package"),
            Token::Dot,
            Token::Ident("MyType"),
            Token::Ident("field_name"),
            Token::Semicolon,
            Token::Newline,
            Token::BlockClose,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        assert!(
            !output.has_errors(),
            "Errors: {:?}",
            output.errors().collect::<Vec<_>>()
        );

        let exprs = output.output().unwrap();
        let Expr::Message(msg) = &exprs[1].node else {
            panic!("Expected Message");
        };

        // Then: The field has the correct absolute type reference.
        let TypeKind::Reference(r) = &msg.fields[0].typ.kind else {
            panic!("Expected Reference type");
        };
        assert!(r.is_absolute(), "Expected absolute reference");
        assert_eq!(r.path(), vec!["other", "package"]);
        assert_eq!(r.name(), "MyType");
    }
}
