use color_eyre::eyre::Report;

pub enum Event {
    Term(crossterm::event::Event),
    NewLine(String),
    EOF,
    Err(Report),
}
