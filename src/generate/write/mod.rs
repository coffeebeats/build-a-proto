use std::path::Path;

/* -------------------------------- Mod: File ------------------------------- */

mod file;
#[allow(unused)]
pub use file::*;

/* ------------------------------- Mod: String ------------------------------ */
mod string;
pub use string::*;

/* -------------------------------------------------------------------------- */
/*                                Trait: Writer                               */
/* -------------------------------------------------------------------------- */

pub trait Writer: Default {
    #[allow(unused)]
    /// `close` closes the [`Writer`], cleaning up any open resources.
    fn close(&mut self) -> anyhow::Result<()>;

    /// `open` opens the [`Writer`], creating any necessary resources.
    fn open<T: AsRef<Path>>(&mut self, path: T) -> anyhow::Result<()>;

    /// `write` writes the provided input to the opened target resource.
    fn write<T: AsRef<str>>(&mut self, input: T) -> anyhow::Result<()>;
}
