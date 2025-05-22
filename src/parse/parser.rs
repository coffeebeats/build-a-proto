use chumsky::Parser;
use chumsky::extra::ParserExtra;
use chumsky::input::MapExtra;
use chumsky::input::ValueInput;
use chumsky::prelude::*;

use super::Keyword;
use super::Span;
use super::Spanned;
use super::Token;

/* -------------------------------------------------------------------------- */
/*                                  Fn: Parse                                 */
/* -------------------------------------------------------------------------- */

/// ParseError is a type alias for errors emitted during parsing.
type ParseError<'src> = Rich<'src, Token<'src>, Span>;

/// `parse` parses an input [`Token`] sequence into [`Expr`]s recognized by the
/// compiler.
pub fn parse<'src>(
    input: &'src Vec<Spanned<Token<'src>>>,
    size: usize,
) -> (Option<Vec<Spanned<Expr<'src>>>>, Vec<ParseError<'src>>) {
    parser()
        .parse(
            input
                .as_slice()
                .map(Span::from(size..size), |(t, s)| (t, s)),
        )
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
        .map(Expr::Package)
        .map_with(Expr::with_span)
        .labelled("package")
        .boxed();

    let include = just(Token::Keyword(Keyword::Include))
        .ignore_then(string)
        .then_ignore(just(Token::Semicolon))
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
        .labelled("comment");

    let line_comment = comment
        .then_ignore(just(Token::Newline))
        .map(Expr::Comment)
        .labelled("comment");
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

    let reference = ident.map(Type::Reference);

    let scalar = select! {
        Token::Ident("bit") => Type::Bit,
        Token::Ident("bool") => Type::Bool,
        Token::Ident("byte") => Type::Byte,
        Token::Ident("u8") => Type::UnsignedInt8,
        Token::Ident("u16") => Type::UnsignedInt16,
        Token::Ident("u32") => Type::UnsignedInt32,
        Token::Ident("u64") => Type::UnsignedInt64,
        Token::Ident("i8") => Type::SignedInt8,
        Token::Ident("i16") => Type::SignedInt16,
        Token::Ident("i32") => Type::SignedInt32,
        Token::Ident("i64") => Type::SignedInt64,
        Token::Ident("f32") => Type::Float32,
        Token::Ident("f64") => Type::Float64,
        Token::Ident("string") => Type::String,
    };

    let array = uint
        .or_not()
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar)
        .map(|(size, t)| Type::Array(Box::new(t), size));
    let map = scalar
        .delimited_by(just(Token::ListOpen), just(Token::ListClose))
        .then(scalar)
        .map(|(k, v)| Type::Map(Box::new(k), Box::new(v)));

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
        .then(uint.then_ignore(just(Token::Colon)).or_not())
        .then(ident)
        .then_ignore(just(Token::Semicolon))
        .map(|((comment, index), name)| {
            Expr::Variant(Variant {
                comment,
                index,
                name,
            })
        })
        .labelled("variant")
        .boxed();

    let field = (doc_comment.clone().or_not())
        .then(uint.then_ignore(just(Token::Colon)).or_not())
        .then(typ)
        .then(ident)
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
                .or_not(),
        )
        .then_ignore(just(Token::Semicolon))
        .map(|((((comment, index), typ), name), encoding)| {
            Expr::Field(Field {
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
        .then(just(Token::Keyword(Keyword::Enum)).ignore_then(ident))
        .then_ignore(
            choice((
                inline_comment.clone(),
                just(Token::Newline).map(|_| Expr::Invalid(&[])),
            ))
            .repeated(),
        )
        .then(
            just(Token::Newline)
                .repeated()
                .ignore_then(choice((
                    field.clone(),
                    variant.clone(),
                    line_comment.clone(),
                )))
                .then_ignore(just(Token::Newline).repeated())
                .repeated()
                .collect::<Vec<Expr<'src>>>()
                .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
        )
        .map(|((comment, name), variants)| Enum {
            comment,
            name,
            variants: variants
                .into_iter()
                .filter_map(|expr| match expr {
                    Expr::Field(f) => Some(VariantKind::Field(f)),
                    Expr::Variant(v) => Some(VariantKind::Variant(v)),
                    _ => None,
                })
                .collect(),
        })
        .map(Expr::Enum)
        .labelled("enum")
        .boxed();

    let message = recursive(|msg| {
        doc_comment
            .or_not()
            .then(just(Token::Keyword(Keyword::Message)).ignore_then(ident))
            .then_ignore(
                choice((
                    inline_comment,
                    just(Token::Newline).map(|_| Expr::Invalid(&[])),
                ))
                .repeated(),
            )
            .then(
                just(Token::Newline)
                    .repeated()
                    .ignore_then(choice((
                        msg,
                        enumeration.clone(),
                        field,
                        line_comment.clone(),
                    )))
                    .then_ignore(just(Token::Newline).repeated())
                    .repeated()
                    .collect::<Vec<Expr<'src>>>()
                    .delimited_by(just(Token::BlockOpen), just(Token::BlockClose)),
            )
            .map(|((comment, name), exprs)| {
                let mut enums = vec![];
                let mut fields = vec![];
                let mut messages = vec![];

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

    // FIXME: Exclude standalone line comments from parsed output.
    let ast = just(Token::Newline)
        .repeated()
        .ignore_then(choice((
            message.map_with(Expr::with_span),
            enumeration.map_with(Expr::with_span),
            package,
            include,
            line_comment.map_with(Expr::with_span),
        )))
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect::<Vec<_>>();

    missing.or(ast)
}

/* -------------------------------------------------------------------------- */
/*                                 Enum: Expr                                 */
/* -------------------------------------------------------------------------- */

/// `Expr` enumerates the set of potential expressions recognized by the
/// compiler.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr<'src> {
    Invalid(&'src [Token<'src>]),

    // Metadata
    Comment(&'src str),
    Include(&'src str),
    Package(&'src str),

    // Properties
    Field(Field<'src>),
    Variant(Variant<'src>),

    // Definitions
    Message(Message<'src>),
    Enum(Enum<'src>),
}

/* ----------------------------- Impl: with_span ---------------------------- */

impl<'src> Expr<'src> {
    /// `with_span`` is a convenience method for creating a [`Spanned`] item
    /// from the provided [`chumsky::MapExtra`] details.
    fn with_span<I, E>(self, info: &mut MapExtra<'src, '_, I, E>) -> Spanned<Expr<'src>>
    where
        I: ValueInput<'src, Token = Token<'src>, Span = Span>,
        E: ParserExtra<'src, I>,
    {
        (self, info.span())
    }
}

/* ----------------------------- Struct: Message ---------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub struct Message<'src> {
    comment: Option<Vec<&'src str>>,
    enums: Vec<Enum<'src>>,
    fields: Vec<Field<'src>>,
    messages: Vec<Message<'src>>,
    name: &'src str,
}

/* ------------------------------ Struct: Enum ------------------------------ */

#[derive(Clone, Debug, PartialEq)]
pub struct Enum<'src> {
    comment: Option<Vec<&'src str>>,
    name: &'src str,
    variants: Vec<VariantKind<'src>>,
}

/* ------------------------------ Struct: Field ----------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub struct Field<'src> {
    comment: Option<Vec<&'src str>>,
    encoding: Option<Vec<Encoding>>,
    index: Option<usize>,
    name: &'src str,
    typ: Type<'src>,
}

/* ----------------------------- Struct: Variant ---------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub struct Variant<'src> {
    comment: Option<Vec<&'src str>>,
    index: Option<usize>,
    name: &'src str,
}

/* ---------------------------- Enum: VariantKind --------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub enum VariantKind<'src> {
    Field(Field<'src>),
    Variant(Variant<'src>),
}

/* ----------------------------- Enum: Encoding ----------------------------- */

#[derive(Clone, Debug, PartialEq)]
pub enum Encoding {
    // Sizing
    Bits(usize),
    BitsVariable(usize),
    FixedPoint(usize, usize),

    // Encodings
    Delta,
    Pad(usize),
    ZigZag,
}

/* ------------------------------- Enum: Type ------------------------------- */

#[derive(Clone, Debug, PartialEq)]
enum Type<'src> {
    Reference(&'src str),

    // Scalars
    Bit,
    Bool,
    Byte,
    Float32,
    Float64,
    SignedInt16,
    SignedInt32,
    SignedInt64,
    SignedInt8,
    String,
    UnsignedInt16,
    UnsignedInt32,
    UnsignedInt64,
    UnsignedInt8,

    // Containers
    Array(Box<Type<'src>>, Option<usize>),
    Map(Box<Type<'src>>, Box<Type<'src>>),
}

/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
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
            Token::String("../a/b/c.ext"),
            Token::Semicolon,
            // No line break!
            Token::Keyword(Keyword::Include),
            Token::String("d.ext"),
            Token::Semicolon,
        ];

        // When: The input is parsed.
        let output = parser().parse(input.as_slice());

        // Then: The input has no errors.
        println!("{:?}", output.errors().collect::<Vec<_>>());
        assert!(!output.has_errors());

        // Then: The output expression list matches expectations.
        let exprs = vec![
            (Expr::Package("abc.def"), Span::from(1..4)),
            (Expr::Include("../a/b/c.ext"), Span::from(6..9)),
            (Expr::Include("d.ext"), Span::from(9..12)),
        ];
        assert_eq!(output.output(), Some(&exprs));
    }
}
