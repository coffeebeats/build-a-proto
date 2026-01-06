/* -------------------------------- Mod: File ------------------------------- */

mod file;

#[allow(unused_imports)]
pub use file::*;

/* ------------------------------- Mod: String ------------------------------ */

mod string;

#[allow(unused_imports)]
pub use string::*;

/* -------------------------------------------------------------------------- */
/*                                Trait: Writer                               */
/* -------------------------------------------------------------------------- */

use std::path::Path;

pub trait Writer: Default {
    fn close(&mut self) -> anyhow::Result<()>;
    fn open<T>(&mut self, path: T) -> anyhow::Result<()>
    where
        T: AsRef<Path>;
    fn write(&mut self, input: &str) -> anyhow::Result<()>;
}
