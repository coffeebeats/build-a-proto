mod file;
mod string;

/* -------------------------------- Mod: File ------------------------------- */

#[allow(unused_imports)]
pub use file::*;

/* ------------------------------- Mod: String ------------------------------ */

#[allow(unused_imports)]
pub use string::*;

use crate::core::Module;

/* -------------------------------------------------------------------------- */
/*                                Trait: Writer                               */
/* -------------------------------------------------------------------------- */

pub trait Writer: Default {
    fn configured(self, module: &Module) -> anyhow::Result<Self>;
    fn close(&mut self) -> anyhow::Result<()>;
    fn open(&mut self, path: &std::path::Path) -> anyhow::Result<()>;
    fn write(&mut self, input: &str) -> anyhow::Result<()>;
}
