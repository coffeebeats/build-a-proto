use derive_builder::Builder;
use derive_more::Display;
use std::collections::HashMap;

use super::Enum;
use super::Message;
use super::Module;

/* -------------------------------------------------------------------------- */
/*                              Struct: Registry                              */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Default)]
pub struct Registry(HashMap<Descriptor, Kind>);

/* ------------------------------- Enum: Kind ------------------------------- */

#[allow(dead_code)]
#[derive(Debug, Display)]
pub enum Kind {
    Enum(Enum),
    Message(Message),
    Module(Module),
}

/* -------------------------------- Impl: get ------------------------------- */

impl Registry {
    #[allow(dead_code)]
    pub fn get(&self, descriptor: &Descriptor) -> Option<&Kind> {
        self.0.get(descriptor)
    }

    #[allow(dead_code)]
    pub fn get_enum(&self, descriptor: &Descriptor) -> Option<&Enum> {
        self.0.get(descriptor).and_then(|k| match k {
            Kind::Enum(e) => Some(e),
            _ => None,
        })
    }

    #[allow(dead_code)]
    pub fn get_message(&self, descriptor: &Descriptor) -> Option<&Message> {
        self.0.get(descriptor).and_then(|k| match k {
            Kind::Message(e) => Some(e),
            _ => None,
        })
    }

    #[allow(dead_code)]
    pub fn get_module(&self, descriptor: &Descriptor) -> Option<&Module> {
        self.0.get(descriptor).and_then(|k| match k {
            Kind::Module(e) => Some(e),
            _ => None,
        })
    }
}

/* --------------------------- Impl: insert/remove -------------------------- */

impl Registry {
    pub fn insert(&mut self, descriptor: Descriptor, kind: Kind) -> Option<Kind> {
        self.0.insert(descriptor, kind)
    }

    pub fn remove(&mut self, descriptor: &Descriptor) -> Option<Kind> {
        self.0.remove(descriptor)
    }
}

/* ----------------------------- Impl: Iterator ----------------------------- */

impl Registry {
    #[allow(dead_code)]
    pub fn iter_enums(&self) -> impl Iterator<Item = (&Descriptor, &Enum)> + '_ {
        self.0.iter().filter_map(|(key, kind)| match kind {
            Kind::Enum(e) => Some((key, e)),
            _ => None,
        })
    }

    #[allow(dead_code)]
    pub fn iter_messages(&self) -> impl Iterator<Item = (&Descriptor, &Message)> + '_ {
        self.0.iter().filter_map(|(key, kind)| match kind {
            Kind::Message(m) => Some((key, m)),
            _ => None,
        })
    }

    pub fn iter_modules(&self) -> impl Iterator<Item = (&Descriptor, &Module)> + '_ {
        self.0.iter().filter_map(|(key, kind)| match kind {
            Kind::Module(m) => Some((key, m)),
            _ => None,
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                             Struct: Descriptor                             */
/* -------------------------------------------------------------------------- */

#[derive(Builder, Clone, Debug, Display, PartialEq, Eq, Hash)]
#[display("{}", String::from(self))]
pub struct Descriptor {
    #[builder(default)]
    pub package: Vec<String>,
    #[builder(default)]
    pub path: Vec<String>,
    #[builder(default, setter(into, strip_option))]
    pub name: Option<String>,
}

/* --------------------------- Impl: Into<String> --------------------------- */

impl From<&Descriptor> for String {
    fn from(value: &Descriptor) -> Self {
        let name = value.name.as_deref().unwrap_or("");
        let pkg = value.package.join(".");
        let path = if !value.path.is_empty() {
            value.path.join(".")
        } else {
            "".to_owned()
        };

        vec![pkg.as_str(), path.as_str(), name]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join(".")
    }
}
