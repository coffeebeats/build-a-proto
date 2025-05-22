use anyhow::anyhow;
use ariadne::Color;
use ariadne::Label;
use ariadne::Report;
use ariadne::ReportKind;
use ariadne::sources;
use std::path::Path;
use std::path::PathBuf;

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

    for path in args.files {
        println!(
            "Compiling {:?} into {:?} (binding={})",
            path,
            out_dir,
            if args.bindings.cpp { "cpp" } else { "gdscript" }
        );

        let contents = std::fs::read_to_string(&path).map_err(|e| anyhow!(e))?;

        let path = path
            .canonicalize()
            .map_err(|e| anyhow!(e))?
            .to_owned()
            .into_os_string()
            .into_string()
            .map_err(|s| anyhow!("couldn't convert path to string: {:?}", s))?;

        let mut parse_errs = vec![];

        let (tokens, lex_errs) = lex(&contents);

        if let Some(tokens) = tokens.as_ref() {
            let (exprs, errs) = parse(tokens, contents.len());
            parse_errs = errs;

            if let Some(exprs) = exprs.as_ref() {
                for expr in exprs {
                    println!("\n{:?}", expr);
                }
            }
        }

        println!("\n");

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
                            .with_message(format!("while parsing this {}", label))
                            .with_color(Color::Yellow)
                    }))
                    .finish()
                    .print(sources([(path.to_owned(), contents.clone())]))
                    .unwrap()
            });
    }

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
