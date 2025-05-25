pub mod enumeration;
pub mod message;
pub mod registry;
pub mod types;

/* ---------------------------- Mod: Enumeration ---------------------------- */

pub use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

pub use message::*;

/* ------------------------------ Mod: Registry ----------------------------- */

pub use registry::*;

/* ------------------------------- Mod: Types ------------------------------- */

pub use types::*;

/* -------------------------------------------------------------------------- */
/*                               Struct: Module                               */
/* -------------------------------------------------------------------------- */

use derive_builder::Builder;
use derive_more::Display;
use std::path::PathBuf;

#[derive(Builder, Clone, Debug, Display)]
#[display("Module({:?}): {:?}", self.path.as_path(), self.messages.iter().chain(self.enums.iter()).map(ToString::to_string).collect::<Vec<_>>())]
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
