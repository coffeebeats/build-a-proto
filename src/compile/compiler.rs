use derive_more::Display;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use crate::core::Descriptor;
use crate::core::Kind;
use crate::core::Registry;

/* -------------------------------------------------------------------------- */
/*                             Enum: CompileError                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Display, Error)]
pub enum CompileError {
    #[display("Unknown include path: {_0:?}; did you forget to import?")]
    MissingInclude(PathBuf),
}

/* ------------------------------- Fn: compile ------------------------------ */

pub fn compile(registry: &mut Registry) -> Result<(), CompileError> {
    let mut modules = HashMap::<PathBuf, Descriptor>::default();

    for (d, module) in registry.iter_modules() {
        modules.insert(module.path.clone(), d.clone());
    }

    for d in modules.values() {
        // Extract the module so it can be modified.
        let kind = registry.remove(d).unwrap();
        debug_assert!(matches!(kind, Kind::Module(_)));

        if let Kind::Module(mut module) = kind {
            for descriptor in module.deps.iter_mut() {
                let dep_path = PathBuf::from(descriptor.name.take().unwrap());
                let dep = modules
                    .get(&dep_path)
                    .ok_or(CompileError::MissingInclude(dep_path))?;

                *descriptor = dep.clone();
            }

            registry.insert(d.clone(), Kind::Module(module));
        }
    }

    Ok(())
}
