/* ----------------------------- Mod: Descriptor ---------------------------- */

mod descriptor;
pub use descriptor::*;

/* ---------------------------- Mod: Enumeration ---------------------------- */

mod enumeration;
pub use enumeration::*;

/* ------------------------------ Mod: Message ------------------------------ */

mod message;
pub use message::*;

/* ------------------------------ Mod: Package ------------------------------ */

mod package;
pub use package::*;

/* ------------------------------- Mod: Import ------------------------------ */

mod import;
pub use import::*;

/* ----------------------------- Mod: Reference ----------------------------- */

mod reference;
pub use reference::*;

/* ------------------------------ Mod: Registry ----------------------------- */

mod registry;
pub use registry::*;

/* ------------------------------- Mod: Types ------------------------------- */

mod types;
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
    #[builder(default)]
    pub deps: Vec<SchemaImport>,
    #[builder(default)]
    pub enums: Vec<Descriptor>,
    #[builder(default)]
    pub messages: Vec<Descriptor>,
    pub path: PathBuf,
    pub package: PackageName,
}

/* ------------------------------ Impl: Module ------------------------------ */

impl Module {
    pub fn new(path: PathBuf) -> Module {
        ModuleBuilder::default().path(path).build().unwrap()
    }
}
