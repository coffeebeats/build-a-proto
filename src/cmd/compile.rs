use anyhow::anyhow;
use std::path::Path;
use std::path::PathBuf;

use crate::core::ImportRoot;

/* -------------------------------------------------------------------------- */
/*                                Struct: Args                                */
/* -------------------------------------------------------------------------- */

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(flatten)]
    bindings: Bindings,

    /// A path to a directory in which to generate language bindings in.
    #[arg(short, long, value_name = "OUT_DIR")]
    out: Option<PathBuf>,

    /// A root directory to search for imported '.baproto' files. Can be
    /// specified multiple times. Imports are resolved by searching each root in
    /// order. If not specified, defaults to the current working directory.
    #[arg(short = 'I', long = "import_root", value_name = "DIR")]
    import_roots: Vec<PathBuf>,

    /// A path to a message definition file to compile.
    #[arg(value_name = "FILES", required = true, num_args = 1..)]
    files: Vec<PathBuf>,
}

/* ---------------------------- Struct: Bindings ---------------------------- */

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
pub struct Bindings {
    /// Whether to compile C++ language bindings.
    #[arg(long)]
    cpp: bool,

    /// Whether to compile GDScript language bindings.
    #[arg(long)]
    gdscript: bool,
}

/* -------------------------------------------------------------------------- */
/*                              Function: handle                              */
/* -------------------------------------------------------------------------- */

/// [`handle`] implements the `compile` command.
pub fn handle(_args: Args) -> anyhow::Result<()> {
    todo!()
}

/* ---------------------------- Fn: parse_out_dir --------------------------- */

/// `parse_out_dir` accepts an optional output directory and returns the
/// directory in which generated artifacts should be written.
fn parse_out_dir(out_dir: Option<impl AsRef<Path>>) -> anyhow::Result<PathBuf> {
    let path: PathBuf;

    if let Some(directory) = out_dir {
        if !directory.as_ref().is_dir() {
            return Err(anyhow!("invalid argument: expected a directory for 'out'"));
        }

        path = directory.as_ref().to_owned()
    } else {
        path = std::env::current_dir()?;
    }

    Ok(path.canonicalize()?)
}

/* ------------------------- Fn: parse_import_roots ------------------------- */

/// `parse_import_roots` validates and canonicalizes the import root
/// directories. If no roots are provided, defaults to the current working
/// directory.
fn parse_import_roots(roots: Vec<PathBuf>) -> anyhow::Result<Vec<ImportRoot>> {
    if roots.is_empty() {
        let cwd = std::env::current_dir()?;
        return Ok(vec![ImportRoot::try_from(cwd).map_err(|e| anyhow!(e))?]);
    }

    roots
        .into_iter()
        .map(|root| ImportRoot::try_from(root).map_err(|e| anyhow!(e)))
        .collect()
}
