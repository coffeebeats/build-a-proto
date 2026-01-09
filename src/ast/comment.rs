use derive_more::Display;

use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                               Struct: Comment                              */
/* -------------------------------------------------------------------------- */

/// `Comment` represents a single line of documentation.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", content)]
pub struct Comment {
    pub content: String,
    pub span: Span,
}

/* -------------------------------------------------------------------------- */
/*                            Struct: CommentBlock                            */
/* -------------------------------------------------------------------------- */

/// `CommentBlock` represents a contiguous group of comments (i.e. comments
/// separated only by [`crate::lex::Token::Newline`] tokens). These can either
/// be attached to declarations, where they'd be interpreted as a "doc comment",
/// or freestanding.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", itertools::join(self.comments.iter().map(Comment::to_string), "\n"))]
pub struct CommentBlock {
    pub comments: Vec<Comment>,
    pub span: Span,
}
