use anyhow::anyhow;
use std::path::Path;
use std::path::PathBuf;

use crate::analyze::DiagnosticReporter;
use crate::core::ImportRoot;
use crate::core::SchemaImport;
use crate::generate::Generator;
use crate::ir;

/* ------------------------------ Mod: Collect ------------------------------ */

mod collect;
pub use collect::*;

/* ------------------------------ Mod: Compiler ----------------------------- */

mod compiler;
pub use compiler::*;

/* ------------------------------- Mod: Source ------------------------------ */

mod source;
pub use source::*;

/* ------------------------------- Mod: Symbol ------------------------------ */

mod symbol;
pub use symbol::*;

/* -------------------------------------------------------------------------- */
/*                                 Fn: compile                                */
/* -------------------------------------------------------------------------- */

/// `compile` compiles the provided input schema `files` into the specified
/// `out` directory. Schema imports will be searched for within `import_roots`;
/// target language bindings will be generated using `generator`.
pub fn compile<P: AsRef<Path>, G: Generator>(
    files: Vec<P>,
    import_roots: Vec<P>,
    out: Option<P>,
    generator: G,
) -> anyhow::Result<()> {
    let out_dir = parse_out_dir(out)?;
    let import_roots = parse_import_roots(import_roots)?;

    let inputs: Vec<SchemaImport> = files
        .into_iter()
        .map(|path| SchemaImport::try_from(path.as_ref()).map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut compiler = Compiler::new(import_roots.clone());

    for schema in inputs {
        compiler.compile(schema);
    }

    for diagnostic in &compiler.diagnostics {
        if let Err(err) = compiler.sources.insert(&diagnostic.span.context) {
            return Err(anyhow!("Failed to read source file: {}", err));
        }
    }

    if !compiler.diagnostics.is_empty() {
        // TODO: Allow customize diagnostic reporting.
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

    let output = generator.generate(&ir).map_err(|e| anyhow!(e))?;

    for (path, contents) in &output.files {
        let path = out_dir.join(path);
        write_generated_file(&path, contents)?;
    }

    Ok(())
}

/* ---------------------------- Fn: parse_out_dir --------------------------- */

/// `parse_out_dir` accepts an optional output directory and returns the
/// directory in which generated artifacts should be written.
fn parse_out_dir<P: AsRef<Path>>(out_dir: Option<P>) -> anyhow::Result<PathBuf> {
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
fn parse_import_roots<P: AsRef<Path>>(roots: Vec<P>) -> anyhow::Result<Vec<ImportRoot>> {
    if roots.is_empty() {
        let cwd = std::env::current_dir()?;
        return Ok(vec![ImportRoot::try_from(cwd).map_err(|e| anyhow!(e))?]);
    }

    roots
        .into_iter()
        .map(|root| ImportRoot::try_from(root.as_ref()).map_err(|e| anyhow!(e)))
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
