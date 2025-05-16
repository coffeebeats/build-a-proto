use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "baproto", author, version, about)]
#[command(arg_required_else_help = true)]
struct Cli {
    /// Silences all non-essential logging.
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// Enables additional detailed logging.
    #[arg(short, long, global = true)]
    verbose: bool,
}

fn main() -> Result<()> {
    let _ = Cli::parse();
    Ok(())
}
