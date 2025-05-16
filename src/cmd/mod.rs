pub mod compile;

/* -------------------------------------------------------------------------- */
/*                               Enum: Commands                               */
/* -------------------------------------------------------------------------- */

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /* -------------------------- Category: Compile ------------------------- */
    /// Compile the specified message definitions into bindings for the
    /// specified language.
    Compile(compile::Args),
}
