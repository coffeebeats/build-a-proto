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
    pub ast: Option<ast::SourceFile>,
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
fn parser<'src, I>() -> impl Parser<'src, I, Option<ast::SourceFile>, extra::Err<ParseError<'src>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    let ident_token = select! { Token::Ident(id) => id };
    let string = select! { Token::String(s) => s };
    let uint = select! { Token::Uint(n) => n };

    let package = just(Token::Keyword(Keyword::Package))
        .ignore_then(
            ident_token
                .separated_by(just(Token::Dot))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(Token::Semicolon))
        .map_with(|segments: Vec<&str>, e| (segments, e.span()))
        .validate(
            |(segments, span), _, emitter| match PackageName::try_from(segments) {
                Ok(pkg_name) => Some(ast::Package {
                    name: pkg_name,
                    span,
                }),
                Err(e) => {
                    emitter.emit(Rich::custom(span, e.to_string()));
                    None
                }
            },
        )
        .labelled("package")
        .boxed();

    let include = just(Token::Keyword(Keyword::Include))
        .ignore_then(string)
        .then_ignore(just(Token::Semicolon))
        .map_with(|s: &str, e| (s, e.span()))
        .validate(
            |(s, span), _, emitter| match import_path().parse(s).into_result() {
                Ok(path) => Some(ast::Include { path, span }),
                Err(errs) => {
                    for err in errs {
                        let msg = format!("{}", err.reason());
                        emitter.emit(Rich::custom(span, msg));
                    }
                    None
                }
            },
        )
        .labelled("include")
        .boxed();

    // Comments

    let comment = select! { Token::Comment(c) => c };
    let inline_comment = just(Token::Newline)
        .not()
        .ignore_then(comment)
        .then_ignore(just(Token::Newline))
        .ignored()
        .labelled("inline comment");

    let line_comment = comment
        .then_ignore(just(Token::Newline))
        .ignored()
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
            ident_token
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
                Ok(r) => ast::TypeKind::Reference(r),
                Err(err) => {
                    emitter.emit(Rich::custom(span, format!("invalid reference: {}", err)));
                    ast::TypeKind::Invalid
                }
            }
        })
        .map_with(|kind, e| ast::Type {
            kind,
            span: e.span(),
        });

    let scalar = select! {
        Token::Ident("bit") => ast::TypeKind::Scalar(ast::ScalarType::Bit),
        Token::Ident("bool") => ast::TypeKind::Scalar(ast::ScalarType::Bool),
        Token::Ident("byte") => ast::TypeKind::Scalar(ast::ScalarType::Byte),
        Token::Ident("u8") => ast::TypeKind::Scalar(ast::ScalarType::Uint8),
        Token::Ident("u16") => ast::TypeKind::Scalar(ast::ScalarType::Uint16),
        Token::Ident("u32") => ast::TypeKind::Scalar(ast::ScalarType::Uint32),
        Token::Ident("u64") => ast::TypeKind::Scalar(ast::ScalarType::Uint64),
        Token::Ident("i8") => ast::TypeKind::Scalar(ast::ScalarType::Int8),
        Token::Ident("i16") => ast::TypeKind::Scalar(ast::ScalarType::Int16),
        Token::Ident("i32") => ast::TypeKind::Scalar(ast::ScalarType::Int32),
        Token::Ident("i64") => ast::TypeKind::Scalar(ast::ScalarType::Int64),
        Token::Ident("f32") => ast::TypeKind::Scalar(ast::ScalarType::Float32),
        Token::Ident("f64") => ast::TypeKind::Scalar(ast::ScalarType::Float64),
        Token::Ident("string") => ast::TypeKind::Scalar(ast::ScalarType::String),
    }
    .map_with(|kind, e| ast::Type {
        kind,
        span: e.span(),
    });

    let array = uint
        .or_not()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar)
        .map_with(|(size, t), e| ast::Type {
            kind: ast::TypeKind::Array {
                element: Box::new(t),
                size,
            },
            span: e.span(),
        });

    let map = scalar
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar)
        .map_with(|(k, v), e| ast::Type {
            kind: ast::TypeKind::Map {
                key: Box::new(k),
                value: Box::new(v),
            },
            span: e.span(),
        });

    let typ = choice((scalar, array, map, reference))
        .labelled("type")
        .boxed();

    // Encodings

    let bits = just(Token::Ident("bits"))
        .ignore_then(uint.delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(ast::Encoding::Bits);
    let bits_variable = just(Token::Ident("bits"))
        .ignore_then(
            just(Token::Ident("var"))
                .ignore_then(uint.delimited_by(just(Token::FnOpen), just(Token::FnClose)))
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(ast::Encoding::BitsVariable);
    let fixed_point = just(Token::Ident("fixed_point"))
        .ignore_then(
            uint.separated_by(just(Token::Comma))
                .exactly(2)
                .collect()
                .delimited_by(just(Token::FnOpen), just(Token::FnClose)),
        )
        .map(|args: Vec<u64>| ast::Encoding::FixedPoint(args[0], args[1]));

    let delta = just(Token::Ident("delta")).map(|_| ast::Encoding::Delta);
    let zig_zag = just(Token::Ident("zig_zag")).map(|_| ast::Encoding::ZigZag);
    let pad = just(Token::Ident("pad"))
        .ignore_then(uint.delimited_by(just(Token::FnOpen), just(Token::FnClose)))
        .map(ast::Encoding::Pad);

    let encoding = choice((bits, bits_variable, fixed_point, delta, zig_zag, pad))
        .labelled("encoding")
        .boxed();

    let variant = doc_comment
        .clone()
        .or_not()
        .then(uint.then_ignore(just(Token::Colon)).map_with(spanned))
        .then(ident_token.map_with(spanned))
        .then_ignore(just(Token::Semicolon))
        .map_with(|((comment, index), name), e| ast::Variant {
            doc: comment.map(|lines| ast::DocComment {
                lines: lines.into_iter().map(|s| s.to_string()).collect(),
                span: e.span(),
            }),
            index: ast::FieldIndexBuilder::default()
                .value(index.inner)
                .span(index.span)
                .build()
                .unwrap(),
            kind: ast::VariantKind::Unit(ast::Ident {
                name: name.inner.to_string(),
                span: name.span,
            }),
            span: e.span(),
        })
        .labelled("variant")
        .boxed();

    let field = doc_comment
        .clone()
        .or_not()
        .then(uint.then_ignore(just(Token::Colon)).map_with(spanned))
        .then(typ)
        .then(ident_token.map_with(spanned))
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
                .map_with(spanned)
                .or_not(),
        )
        .then_ignore(just(Token::Semicolon))
        .map_with(
            |((((comment, index), typ), name), encoding), e| ast::Field {
                doc: comment.map(|lines| ast::DocComment {
                    lines: lines.into_iter().map(|s| s.to_string()).collect(),
                    span: e.span(),
                }),
                encoding: encoding.map(|enc| ast::EncodingSpec {
                    encodings: enc.inner,
                    span: enc.span,
                }),
                index: ast::FieldIndexBuilder::default()
                    .value(index.inner)
                    .span(index.span)
                    .build()
                    .unwrap(),
                name: ast::Ident {
                    name: name.inner.to_string(),
                    span: name.span,
                },
                span: e.span(),
                typ,
            },
        )
        .labelled("field")
        .boxed();

    let enumeration = doc_comment
        .clone()
        .or_not()
        .then(just(Token::Keyword(Keyword::Enum)).ignore_then(ident_token.map_with(spanned)))
        .then_ignore(choice((inline_comment.clone(), just(Token::Newline).ignored())).repeated())
        .then(
            choice((
                field.clone().map(|f| {
                    Some(ast::Variant {
                        doc: f.doc.clone(),
                        index: f.index.clone(),
                        span: f.span,
                        kind: ast::VariantKind::Field(f),
                    })
                }),
                variant.clone().map(Some),
                line_comment.clone().to(None),
            ))
            .or(just(Token::Newline).to(None))
            .repeated()
            .collect::<Vec<Option<ast::Variant>>>()
            .map(|v: Vec<Option<ast::Variant>>| v.into_iter().flatten().collect())
            .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
        )
        .then_ignore(just(Token::Newline).or_not())
        .validate(
            |((comment, name), variants): ((Option<Vec<&str>>, _), Vec<ast::Variant>),
             info,
             emitter| {
                // Check for duplicate variant names
                let mut seen = HashSet::<String>::new();
                for variant in &variants {
                    let variant_name = match &variant.kind {
                        ast::VariantKind::Field(f) => &f.name.name,
                        ast::VariantKind::Unit(id) => &id.name,
                    };

                    if seen.contains(variant_name) {
                        emitter.emit(Rich::custom(
                            info.span(),
                            format!("Duplicate variant name: {}", variant_name),
                        ));
                    }
                    seen.insert(variant_name.clone());
                }

                ast::Enum {
                    doc: comment.map(|lines| ast::DocComment {
                        lines: lines.into_iter().map(|s| s.to_string()).collect(),
                        span: info.span(),
                    }),
                    name: ast::Ident {
                        name: name.inner.to_string(),
                        span: name.span,
                    },
                    span: info.span(),
                    variants,
                }
            },
        )
        .labelled("enum")
        .boxed();

    // Message (recursive)
    let message = recursive(|msg| {
        doc_comment
            .or_not()
            .then(just(Token::Keyword(Keyword::Message)).ignore_then(ident_token.map_with(spanned)))
            .then_ignore(choice((inline_comment, just(Token::Newline).ignored())).repeated())
            .then(
                choice((
                    msg.map(|m| Some(MessageBodyItem::Message(m))),
                    enumeration.clone().map(|e| Some(MessageBodyItem::Enum(e))),
                    field.map(|f| Some(MessageBodyItem::Field(f))),
                    line_comment.clone().to(None),
                ))
                .or(just(Token::Newline).to(None))
                .repeated()
                .collect::<Vec<Option<MessageBodyItem>>>()
                .map(|v: Vec<Option<MessageBodyItem>>| v.into_iter().flatten().collect())
                .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
            )
            .then_ignore(just(Token::Newline).or_not())
            .validate(
                |((comment, name), items): ((Option<Vec<&str>>, _), Vec<MessageBodyItem>),
                 info,
                 emitter| {
                    // Check for duplicate field names.
                    let mut seen = HashSet::<String>::new();
                    for item in &items {
                        if let MessageBodyItem::Field(f) = item {
                            if seen.contains(&f.name.name) {
                                emitter.emit(Rich::custom(
                                    info.span(),
                                    format!("Duplicate field name: {}", f.name.name),
                                ));
                            }

                            seen.insert(f.name.name.clone());
                        }
                    }

                    let mut fields = vec![];
                    let mut nested_enums = vec![];
                    let mut nested_messages = vec![];

                    for item in items {
                        match item {
                            MessageBodyItem::Field(f) => fields.push(f),
                            MessageBodyItem::Enum(e) => nested_enums.push(e),
                            MessageBodyItem::Message(m) => nested_messages.push(m),
                        }
                    }

                    ast::Message {
                        doc: comment.map(|lines| ast::DocComment {
                            lines: lines.into_iter().map(|s| s.to_string()).collect(),
                            span: info.span(),
                        }),
                        fields,
                        name: ast::Ident {
                            name: name.inner.to_string(),
                            span: name.span,
                        },
                        nested_enums,
                        nested_messages,
                        span: info.span(),
                    }
                },
            )
            .labelled("message")
            .boxed()
    });

    let missing = empty().then(end()).validate(|_, info, emitter| {
        emitter.emit(Rich::custom(info.span(), "missing input"));
        None
    });

    let leading = choice((
        just(Token::Newline).to(None),
        line_comment.clone().map_with(spanned).map(Some),
    ))
    .repeated()
    .collect::<Vec<_>>()
    .map(|v| v.into_iter().flatten().collect::<Vec<_>>());

    let header = leading.ignore_then(package).then(
        choice((just(Token::Newline).to(None), include.map(Some)))
            .repeated()
            .collect::<Vec<_>>()
            .map(|v| v.into_iter().flatten().flatten().collect::<Vec<_>>()),
    );

    let declaration = choice((
        message.map(ast::Item::Message),
        enumeration.map(ast::Item::Enum),
    ))
    .recover_with(skip_then_retry_until(any().ignored(), end()));

    let declarations = choice((just(Token::Newline).to(None), declaration.map(Some)))
        .repeated()
        .collect::<Vec<_>>()
        .map(|v: Vec<Option<ast::Item>>| v.into_iter().flatten().collect::<Vec<_>>());

    let ast = header
        .then(declarations)
        .map_with(|((package, includes), items), e| {
            Some(ast::SourceFile {
                includes,
                items,
                package: package?,
                span: e.span(),
            })
        });

    missing.or(ast)
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

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input_returns_empty_source_file() {
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

        // Then: The output is None (no valid AST).
        let result = output.output().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_example_header_returns_correct_source_file() {
        // Given: An input list of tokens.
        let input = vec![
            Token::Newline,
            Token::Keyword(Keyword::Package),
            Token::Ident("abc"),
            Token::Dot,
            Token::Ident("def"),
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

        // Then: The output SourceFile matches expectations.
        let source_file = output.output().unwrap().as_ref().unwrap();

        // Check package
        assert_eq!(source_file.package.name.to_string(), "abc.def");

        // Check includes
        assert_eq!(source_file.includes.len(), 2);
        assert_eq!(source_file.includes[0].path, PathBuf::from("a/b/c.baproto"));
        assert_eq!(source_file.includes[1].path, PathBuf::from("d.baproto"));

        // No items (no messages or enums)
        assert!(source_file.items.is_empty());
    }

    #[test]
    fn test_include_rejects_dot_prefix() {
        // Given: An input with a ./ prefixed include path.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::Ident("test"),
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
            Token::Ident("test"),
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
            Token::Ident("test"),
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
            Token::Ident("test"),
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
            Token::Ident("test"),
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
            Token::Ident("test"),
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
    fn test_line_comment_in_message_returns_correct_source_file() {
        // Given: An input list of tokens.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::Ident("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Comment("comment"),
            Token::Newline,
            Token::Newline,
            Token::Uint(0),
            Token::Colon,
            Token::Ident("u8"),
            Token::Ident("sequence_id"),
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

        // Then: The output SourceFile matches expectations.
        let source_file = output.output().unwrap().as_ref().unwrap();

        // Check package
        assert_eq!(source_file.package.name.to_string(), "test");

        // Check items - should have one message
        assert_eq!(source_file.items.len(), 1);
        let ast::Item::Message(msg) = &source_file.items[0] else {
            panic!("Expected Message item");
        };

        // Check message
        assert_eq!(msg.name.name, "Message");
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].name.name, "sequence_id");
        assert_eq!(msg.fields[0].index.value, 0);
        assert!(matches!(
            msg.fields[0].typ.kind,
            ast::TypeKind::Scalar(ast::ScalarType::Uint8)
        ));
    }

    #[test]
    fn test_relative_type_reference_parses_correctly() {
        // Given: An input with a dot-separated relative type reference.
        let input = vec![
            Token::Keyword(Keyword::Package),
            Token::Ident("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Uint(0),
            Token::Colon,
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

        let source_file = output.output().unwrap().as_ref().unwrap();
        let ast::Item::Message(msg) = &source_file.items[0] else {
            panic!("Expected Message");
        };

        // Then: The field has the correct type reference.
        let ast::TypeKind::Reference(r) = &msg.fields[0].typ.kind else {
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
            Token::Ident("test"),
            Token::Semicolon,
            Token::Newline,
            Token::Keyword(Keyword::Message),
            Token::Ident("Message"),
            Token::BlockOpen,
            Token::Newline,
            Token::Uint(0),
            Token::Colon,
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

        let source_file = output.output().unwrap().as_ref().unwrap();
        let ast::Item::Message(msg) = &source_file.items[0] else {
            panic!("Expected Message");
        };

        // Then: The field has the correct absolute type reference.
        let ast::TypeKind::Reference(r) = &msg.fields[0].typ.kind else {
            panic!("Expected Reference type");
        };
        assert!(r.is_absolute(), "Expected absolute reference");
        assert_eq!(r.path(), vec!["other", "package"]);
        assert_eq!(r.name(), "MyType");
    }
}
