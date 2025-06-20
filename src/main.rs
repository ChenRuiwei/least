#![feature(slice_internals)]

mod app;
mod error;
mod event;
mod input;
mod keys;
mod tracing;
mod utils;

use app::{App, Cli};
use clap::Parser;

use crate::error::*;

fn main() -> Result<()> {
    let cli = Cli::parse();
    color_eyre::install()?;
    tracing::initialize_logging()?;
    let terminal = ratatui::init();
    let result = App::new(cli).run(terminal);
    ratatui::restore();
    result
}
