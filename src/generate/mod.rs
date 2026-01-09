pub mod api;
mod code;
mod external;
mod generator;
pub mod lang;
mod write;
mod writer;

/* ----------------------------- Mod: Generator ----------------------------- */

// Internal visitor-pattern generator - not publicly exported to avoid
// conflict with the simple Generator trait from api module
#[allow(dead_code, unused_imports)]
pub(crate) use generator::*;

/* ------------------------------- Mod: Write ------------------------------- */

pub use write::*;

/* ------------------------------- Mod: Code -------------------------------- */

pub use code::*;

/* -------------------------------- Mod: Lang ------------------------------- */

#[allow(unused_imports)]
pub use lang::{gdscript, rust};

/* ------------------------------- Mod: API --------------------------------- */

pub use api::{Generator, GeneratorError, GeneratorOutput};

/* ----------------------------- Mod: External ------------------------------ */

pub use external::ExternalGenerator;

/* ----------------------------- Mod: Writer -------------------------------- */

pub use writer::FileWriter;
