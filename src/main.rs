mod app;
mod input;
mod keys;
mod tracing;
mod utils;

use app::App;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing::initialize_logging()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}
