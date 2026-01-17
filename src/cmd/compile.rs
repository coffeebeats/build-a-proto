use anyhow::anyhow;
use std::path::PathBuf;

use crate::compile::compile;
use crate::generate::ExternalGenerator;
use crate::generate::RustGenerator;

/* -------------------------------------------------------------------------- */
/*                                Struct: Args                                */
/* -------------------------------------------------------------------------- */

#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(flatten)]
    pub generator: GeneratorSelection,

    /// A path to a directory in which to generate language bindings in.
    #[arg(short, long, value_name = "OUT_DIR")]
    pub out: Option<PathBuf>,

    /// A root directory to search for imported '.baproto' files. Can be
    /// specified multiple times. Imports are resolved by searching each root in
    /// order. If not specified, defaults to the current working directory.
    #[arg(short = 'I', long = "import_root", value_name = "DIR")]
    pub import_roots: Vec<PathBuf>,

    /// A path to a message definition file to compile.
    #[arg(value_name = "FILES", required = true, num_args = 1..)]
    pub files: Vec<PathBuf>,
}

/* ------------------------- Struct: GeneratorSelection ------------------------- */

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
pub struct GeneratorSelection {
    /// Generate Rust language bindings.
    #[arg(long)]
    pub rust: bool,

    /// Use an external generator binary.
    #[arg(long = "plugin", value_name = "BINARY")]
    pub plugin: Option<PathBuf>,
}

/* -------------------------------------------------------------------------- */
/*                              Function: handle                              */
/* -------------------------------------------------------------------------- */

/// [`handle`] implements the `compile` command.
#[allow(unused)]
pub fn handle(args: Args) -> anyhow::Result<()> {
    if args.generator.rust {
        compile(args.files, args.import_roots, args.out, RustGenerator)
    } else if let Some(plugin_path) = args.generator.plugin {
        let generator = ExternalGenerator::new(plugin_path).map_err(|e| anyhow!(e))?;
        compile(args.files, args.import_roots, args.out, generator)
    } else {
        unreachable!()
    }
}
