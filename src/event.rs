use color_eyre::eyre::Report;

pub enum Event {
    Term(crossterm::event::Event),
    NewLines(Vec<String>),
    EOF,
    Err(Report),
    ReaderThreadErrReturned,
}
