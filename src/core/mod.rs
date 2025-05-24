pub mod enumeration;
pub mod message;
pub mod registry;

/* ---------------------------- Mod: Enumeration ---------------------------- */

pub use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

pub use message::*;

/* ------------------------------ Mod: Registry ----------------------------- */

pub use registry::*;

/* -------------------------------------------------------------------------- */
/*                               Struct: Module                               */
/* -------------------------------------------------------------------------- */

use derive_builder::Builder;
use derive_more::Display;
use std::path::PathBuf;

#[derive(Builder, Clone, Debug, Display)]
#[display("Module({:?})", self.path.as_path())]
pub struct Module {
    pub path: PathBuf,
    #[builder(default)]
    pub package: Vec<String>,
    #[builder(default)]
    pub deps: Vec<Descriptor>,
    #[builder(default)]
    pub enums: Vec<Descriptor>,
    #[builder(default)]
    pub messages: Vec<Descriptor>,
}

/* -------------------------------- Impl: New ------------------------------- */

impl Module {
    pub fn new(path: PathBuf) -> Module {
        ModuleBuilder::default().path(path).build().unwrap()
    }
}
