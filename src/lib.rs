mod analyze;
mod ast;
mod cmd;
mod lex;
mod parse;
mod visit;

/* ------------------------------ Mod: Compile ------------------------------ */

mod compile;
pub use compile::compile;

/* -------------------------------- Mod: Core ------------------------------- */

mod core;
pub use core::*;

/* ------------------------------ Mod: Generate ----------------------------- */

mod generate;
pub use generate::Language;
pub use generate::{CodeWriter, CodeWriterBuilder, CodeWriterBuilderError};
pub use generate::{FileWriter, StringWriter, Writer};
pub use generate::{Generator, GeneratorError, GeneratorOutput};

/* --------------------------------- Mod: IR -------------------------------- */

mod ir;

#[allow(unused)]
use ir::lower::*; // Hide lowering implementation.
pub use ir::*;
