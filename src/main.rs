mod app;
mod input;
mod keys;
mod tracing;
mod utils;

use app::{App, Cli};
use clap::Parser;

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();
    color_eyre::install()?;
    tracing::initialize_logging()?;
    let terminal = ratatui::init();
    let result = App::new(cli).run(terminal);
    ratatui::restore();
    result
}
