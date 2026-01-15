use std::collections::HashMap;
use std::rc::Rc;

use crate::core::SchemaImport;

/* -------------------------------------------------------------------------- */
/*                            Struct: SourceCache                             */
/* -------------------------------------------------------------------------- */

/// `SourceCache` maintains a cache of source file contents. Files are loaded
/// on-demand when first accessed and cached for subsequent requests.
#[derive(Clone, Default)]
pub struct SourceCache {
    cache: HashMap<SchemaImport, Rc<String>>,
}

/* --------------------------- Impl: SourceCache ---------------------------- */

impl SourceCache {
    /// `read` retrieves the source contents for a file, if it exists.
    pub fn read(&self, file: &SchemaImport) -> Option<Rc<String>> {
        self.cache.get(file).cloned()
    }

    /// `insert` adds the source contents of a file into the cache.
    pub fn insert(&mut self, import: &SchemaImport) -> Result<Rc<String>, std::io::Error> {
        match self.cache.get(import) {
            Some(c) => Ok(c.clone()),
            None => {
                let contents = Rc::new(std::fs::read_to_string(import.as_path())?);
                self.cache.insert(import.clone(), contents.clone());
                Ok(contents)
            }
        }
    }
}
