use std::{cmp::min, fs::File, io::BufReader};

use clap::Parser;
use color_eyre::{Result, eyre::Ok};
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use keys::{Action, KeyState};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Rect, Size},
    style::Stylize,
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::{
    input::{InputBuffer, InputReader},
    keys,
    utils::count_lines,
};

/// least: a minimal pager to replace `less`
#[derive(Default, Parser, Debug)]
#[clap(
    name = "least",
    version = "0.1.0",
    author = "ChenRuiwei",
    about = "A lightweight pager as a simpler alternative to `less`"
)]
pub struct Cli {
    #[clap(value_name = "FILE")]
    pub file: String,
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    cli: Cli,
    mode: AppMode,
    buffer: InputBuffer,
    current_line: usize,
    total_lines: usize,
    key_state: KeyState,
    term_size: Size,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new(cli: Cli) -> Self {
        Self {
            cli,
            ..Default::default()
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.read_file()?;
        self.term_size = terminal.size()?;

        while self.mode != AppMode::Terminated {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
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
            Event::Resize(colomns, rows) => self.on_term_resize(Size::new(colomns, rows)),
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        let (key_state, action) = self.key_state.next(key);
        self.key_state = key_state;
        self.on_action(action);
    }

    fn on_action(&mut self, action: Action) {
        match action {
            Action::GoToMain => {}
            Action::GoToTop => self.go_to_top(),
            Action::GoToBottom => self.go_to_bottom(),
            Action::GoToLine(line) => self.go_to_line(line),
            Action::ScrollUpOneLine => self.scroll_up_one_line(),
            Action::ScrollDownOneLine => self.scroll_down_one_line(),
            Action::ScrollUpHalfScreen => self.scroll_up_half_screen(),
            Action::ScrollDownHalfScreen => self.scroll_down_half_screen(),
            Action::ScrollUpScreen => self.scroll_up_screen(),
            Action::ScrollDownScreen => self.scroll_down_screen(),
            Action::None => {}
            Action::Quit => self.quit(),
        }
    }

    fn on_term_resize(&mut self, new_size: Size) {
        self.term_size = new_size;
        self.current_line = min(self.current_line, self.current_max_line());
    }

    fn term_half_height(&self) -> usize {
        (self.term_size.height / 2) as _
    }

    fn term_height(&self) -> usize {
        self.term_size.height as _
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.mode = AppMode::Terminated
    }

    fn scroll_up_one_line(&mut self) {
        self.current_line = self.current_line.saturating_sub(1)
    }

    fn scroll_down_one_line(&mut self) {
        self.current_line = min(self.current_line.saturating_add(1), self.current_max_line())
    }

    fn scroll_up_half_screen(&mut self) {
        self.current_line = self.current_line.saturating_sub(self.term_half_height())
    }

    fn scroll_down_half_screen(&mut self) {
        self.current_line = min(
            self.current_line.saturating_add(self.term_half_height()),
            self.current_max_line(),
        )
    }

    fn scroll_up_screen(&mut self) {
        self.current_line = self.current_line.saturating_sub(self.term_height())
    }

    fn scroll_down_screen(&mut self) {
        self.current_line = min(
            self.current_line.saturating_add(self.term_height()),
            self.current_max_line(),
        )
    }

    fn current_max_line(&self) -> usize {
        self.total_lines - self.term_height()
    }

    fn go_to_top(&mut self) {
        self.current_line = 0
    }

    fn go_to_bottom(&mut self) {
        self.current_line = self.current_max_line()
    }

    fn go_to_line(&mut self, line: usize) {
        self.current_line = min(line, self.current_max_line())
    }

    fn read_file(&mut self) -> Result<()> {
        log::info!("read file {}", self.cli.file);
        let mut f = File::open(&self.cli.file)?;
        self.total_lines = count_lines(&mut f)?;
        let f = File::open(&self.cli.file)?;
        self.buffer.reader = Some(InputReader::new(BufReader::new(f)));
        Ok(())
    }

    /// Create some lines to display in the paragraph.
    fn create_lines(&mut self) -> Result<Vec<Line<'_>>> {
        let lines = self
            .buffer
            .lines(self.current_line, self.term_height())
            .unwrap();
        Ok(lines)
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.create_lines().unwrap())
            .gray()
            .render(area, buf);
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub enum AppMode {
    #[default]
    Main,
    Search,
    Help,
    Terminated,
}
