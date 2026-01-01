use derive_more::Display;

/* -------------------------------------------------------------------------- */
/*                                 Enum: Token                                */
/* -------------------------------------------------------------------------- */

/// `Token` enumerates the set of potential tokens recognized by the parser.
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Token<'src> {
    Invalid(&'src str),

    // Syntax
    BlockClose,
    BlockOpen,
    Colon,
    Comma,
    Dot,
    Equal,
    FnClose,
    FnOpen,
    Keyword(Keyword),
    ListClose,
    ListOpen,
    Semicolon,

    // Whitespace
    Newline,

    // Input
    Comment(&'src str),
    Ident(&'src str),
    String(&'src str),
    /// `Uint` is any unsigned integer token; stored as a `u64` to ensure cross-
    /// platform compatibility and maximum flexibility. Downstream phases should
    /// enforce stricter size limits as needed.
    Uint(u64),
}

/* ------------------------------ Enum: Keyword ----------------------------- */

/// Keyword enumerates the language's reserved keywords.
#[derive(Clone, Debug, Display, PartialEq)]
pub enum Keyword {
    Encoding,
    Enum,
    Include,
    Message,
    Package,
}
