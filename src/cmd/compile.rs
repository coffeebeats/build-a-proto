use anyhow::anyhow;
use std::path::Path;
use std::path::PathBuf;

use crate::parse::lex;

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
pub fn handle(args: Args) -> anyhow::Result<()> {
    let out_dir = parse_out_dir(args.out)?;

    let mut failed: Vec<(PathBuf, Vec<_>)> = vec![];

    for path in args.files {
        println!(
            "Compiling {:?} into {:?} (binding={})",
            path,
            out_dir,
            if args.bindings.cpp { "cpp" } else { "gdscript" }
        );

        let contents = std::fs::read_to_string(&path).map_err(|e| anyhow!(e))?;

        match lex(&contents) {
            Err(errs) => {
                failed.push((path, errs.into_iter().map(|e| e.into_owned()).collect()));
            }
            Ok(_) => {}
        };
    }

    if failed.is_empty() {
        return Ok(());
    }

    for failure in failed {
        println!("Failed to parse file: {:?}", failure.0);

        for err in failure.1 {
            println!("  {:?}", err);
        }
    }

    Err(anyhow!("Compilation failed"))
}

/* -------------------------------------------------------------------------- */
/*                           Function: parse_out_dir                          */
/* -------------------------------------------------------------------------- */

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
