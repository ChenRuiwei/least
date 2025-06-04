use std::{alloc::Layout, fs::File, io::Read};

use color_eyre::{Result, eyre::Ok};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph, Widget},
};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    buf_string: String,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.read_file()?;
        self.running = true;
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn read_file(&mut self) -> Result<()> {
        let mut f = File::open("/home/crw/Code/rust/least/tmp/pacman.conf")?;
        let mut v = Vec::new();
        f.read_to_end(&mut v)?;
        let s = String::from_utf8(v)?;
        self.buf_string = s;
        Ok(())
    }

    /// Create some lines to display in the paragraph.
    fn create_lines(&self) -> Result<Vec<Line<'_>>> {
        let lines = self.buf_string.split("\n").collect::<Vec<_>>();
        let mut ret = Vec::new();
        for l in lines {
            ret.push(Line::from(l));
        }
        Ok(ret)
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.create_lines().unwrap())
            .gray()
            .render(area, buf);
    }
}
