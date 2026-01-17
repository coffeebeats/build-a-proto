mod analyze;
mod ast;
mod cmd;
mod compile;
mod core;
mod generate;
mod ir;
mod lex;
mod parse;
mod visit;

/* ------------------------------ Mod: Compile ------------------------------ */

pub use compile::compile;

/* ------------------------------ Mod: Generate ----------------------------- */

pub use generate::Language;
pub use generate::{CodeWriter, CodeWriterBuilder, CodeWriterBuilderError};
pub use generate::{Generator, GeneratorError, GeneratorOutput};

/* --------------------------------- Mod: IR -------------------------------- */

#[allow(unused)]
use ir::lower::*; // Hide lowering implementation.
pub use ir::*;
