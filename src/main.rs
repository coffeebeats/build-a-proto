mod analyze;
mod ast;
mod cmd;
mod compile;
mod core;
mod generate2;
mod ir;
mod lex;
mod parse;
mod visit;

use anyhow::Result;
use clap::Parser;

use cmd::Commands;

#[derive(Parser)]
#[command(name = "baproto", author, version, about)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Silences all non-essential logging.
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Enables additional detailed logging.
    #[arg(short, long, global = true)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        /* ------------------------ Category: Compile ----------------------- */
        Commands::Compile(args) => cmd::compile::handle(args),
    }
}
