use anyhow::anyhow;
use std::path::Path;
use std::path::PathBuf;

use crate::analyze::DiagnosticReporter;
use crate::compile::Compiler;
use crate::core::{ImportRoot, SchemaImport};
use crate::generate::RustGenerator;
use crate::generate::{ExternalGenerator, Generator};
use crate::ir;

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
pub fn handle(args: Args) -> anyhow::Result<()> {
    let out_dir = parse_out_dir(args.out)?;
    let import_roots = parse_import_roots(args.import_roots)?;

    let inputs: Vec<SchemaImport> = args
        .files
        .into_iter()
        .map(|path| SchemaImport::try_from(path).map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut compiler = Compiler::new(import_roots.clone());

    for schema in inputs {
        println!("Compiling {:?}", schema.as_path());

        compiler.compile(schema);
    }

    for diagnostic in &compiler.diagnostics {
        if let Err(err) = compiler.sources.insert(&diagnostic.span.context) {
            return Err(anyhow!("Failed to read source file: {}", err));
        }
    }

    if !compiler.diagnostics.is_empty() {
        let reporter = DiagnosticReporter::new(&compiler.sources);

        for diagnostic in &compiler.diagnostics {
            reporter.report(diagnostic);
        }

        let error_count = compiler
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, crate::analyze::Severity::Error))
            .count();

        if error_count > 0 {
            return Err(anyhow!("Compilation failed with {} error(s).", error_count));
        }
    }

    let ir = ir::Schema::from(compiler);

    let generator: Box<dyn Generator> = if args.generator.rust {
        Box::new(RustGenerator)
    } else if let Some(plugin_path) = args.generator.plugin {
        Box::new(ExternalGenerator::new(plugin_path).map_err(|e| anyhow!(e))?)
    } else {
        // This shouldn't happen due to clap group constraints
        return Err(anyhow!("No generator specified"));
    };

    println!("Generating {} bindings...", generator.name());

    let output = generator.generate(&ir).map_err(|e| anyhow!(e))?;

    // Write generated files
    for (path, contents) in &output.files {
        let path = out_dir.join(path);
        write_generated_file(&path, contents)?;
        println!("  Wrote: {}", path.display());
    }

    println!(
        "\nGeneration successful! {} file(s) written.",
        output.files.len()
    );
    Ok(())
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

/* ------------------------ Fn: write_generated_file ------------------------ */

/// `write_generated_file` writes the specified `contents`` to the provided
/// `path`, creating any intermediate directories as needed.
fn write_generated_file<T: AsRef<Path>, U: AsRef<str>>(
    path: T,
    contents: U,
) -> std::io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path.as_ref(), contents.as_ref())
}
