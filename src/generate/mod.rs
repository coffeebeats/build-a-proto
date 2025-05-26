mod code;
mod generator;
mod lang;
mod write;

/* ----------------------------- Mod: Generator ----------------------------- */

#[allow(dead_code, unused_imports)]
pub use generator::*;

/* ------------------------------- Mod: Write ------------------------------- */

pub use write::*;

/* ------------------------------- Mod: Writer ------------------------------ */

pub use code::*;

/* -------------------------------- Mod: Lang ------------------------------- */

pub fn gdscript<W>() -> impl Generator<W>
where
    W: Writer,
{
    lang::GDScript::default()
}
