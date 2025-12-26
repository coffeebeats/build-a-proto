use anyhow::anyhow;
use ariadne::Color;
use ariadne::Label;
use ariadne::Report;
use ariadne::ReportKind;
use ariadne::sources;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;

use crate::compile::compile;
use crate::compile::prepare;
use crate::core::ImportRoot;
use crate::core::Registry;
use crate::core::SchemaImport;
use crate::generate;
use crate::generate::FileWriter;
use crate::generate::generate;
use crate::parse::LexError;
use crate::parse::ParseError;
use crate::parse::lex;
use crate::parse::parse;

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
pub fn handle(args: Args) -> anyhow::Result<()> {
    let out_dir = parse_out_dir(args.out)?;

    let mut reg = Registry::default();

    let inputs: Vec<SchemaImport> = args
        .files
        .into_iter()
        .map(|path| SchemaImport::try_from(path).map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen = HashSet::<SchemaImport>::with_capacity(inputs.len());
    let mut files = VecDeque::<SchemaImport>::from(inputs);

    let import_roots = parse_import_roots(args.import_roots)?;

    while !files.is_empty() {
        let schema_import = files.pop_front().unwrap();

        if seen.contains(&schema_import) {
            continue;
        }

        let path = schema_import.as_path();

        println!(
            "Compiling {:?} into {:?} (binding={})",
            path,
            out_dir,
            if args.bindings.cpp { "cpp" } else { "gdscript" }
        );

        let contents = std::fs::read_to_string(path).map_err(|e| anyhow!(e))?;

        let (tokens, lex_errs) = lex(&contents);
        let mut parse_errs = vec![];

        if let Some(tokens) = tokens.as_ref() {
            let (exprs, errs) = parse(tokens, contents.len());
            parse_errs = errs;

            if let Some(exprs) = exprs {
                if let Err(err) = prepare(&path, &import_roots, &mut reg, exprs) {
                    parse_errs.push(err);
                }
            }
        }

        if !lex_errs.is_empty() || !parse_errs.is_empty() {
            report_errors(path.to_str().unwrap(), &contents, lex_errs, parse_errs);
            return Err(anyhow!("Failed to parse file: {:?}", path));
        }

        // Queue imported modules for processing.
        for (_, m) in reg.iter_modules() {
            for dep in &m.deps {
                if !seen.contains(dep) {
                    files.push_back(dep.clone());
                }
            }
        }

        seen.insert(schema_import);
    }

    // TODO: Implement full cyclical dependency detection here. At this point,
    // all modules have been registered in the registry with their dependencies
    // recorded in `Module.deps`. Build a dependency graph and detect cycles
    // (e.g., A → B → C → A) before proceeding with compilation.

    compile(&mut reg).map_err(|e| anyhow!(e))?;

    if args.bindings.gdscript {
        let mut gdscript = generate::gdscript::<FileWriter>();
        generate(out_dir, &mut reg, &mut gdscript)?;
    }

    Ok(())
}

fn report_errors<'a>(
    path: &str,
    contents: &str,
    lex_errs: Vec<LexError<'a>>,
    parse_errs: Vec<ParseError<'a>>,
) {
    lex_errs
        .into_iter()
        .map(|e| e.map_token(|c| c.to_string()))
        .chain(
            parse_errs
                .into_iter()
                .map(|e| e.map_token(|t| t.to_string())),
        )
        .for_each(|e| {
            Report::build(ReportKind::Error, (path.to_owned(), e.span().into_range()))
                .with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
                .with_message(e.to_string())
                .with_label(
                    Label::new((path.to_owned(), e.span().into_range()))
                        .with_message(e.reason().to_string())
                        .with_color(Color::Red),
                )
                .with_labels(e.contexts().map(|(label, span)| {
                    Label::new((path.to_owned(), span.into_range()))
                        .with_message(format!("while parsing this {:?}", label))
                        .with_color(Color::Yellow)
                }))
                .finish()
                .print(sources([(path.to_owned(), contents.to_owned())]))
                .unwrap()
        });
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
