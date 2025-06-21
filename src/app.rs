use std::{
    cell::{Ref, RefCell, RefMut},
    cmp::min,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self},
};

use clap::Parser;
use color_eyre::eyre::eyre;
use crossterm::event::{KeyEvent, KeyEventKind};
use keys::{Action, KeyState};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Rect, Size},
    style::Stylize,
    widgets::{Paragraph, Widget},
};

use crate::{
    error::*,
    event::Event,
    input::{Input, OpenedInput},
    keys,
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
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub files: Vec<PathBuf>,
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    cli: Cli,
    mode: AppMode,
    opened_input: Option<RefCell<OpenedInput>>,
    current_line: usize,
    key_state: KeyState,
    term_size: Size,
    rx: Option<Receiver<Event>>,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        Self {
            cli,
            ..Default::default()
        }
    }

    fn inputs(&self) -> Result<Vec<Input>> {
        if self.cli.files.is_empty() {
            return Ok(vec![Input::stdin()]);
        }
        let mut file_input = Vec::new();
        for file in &self.cli.files {
            file_input.push(Input::ordinary_file(file));
        }
        Ok(file_input)
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut inputs = self.inputs()?;
        let input = inputs.pop().unwrap();

        let (tx, rx) = mpsc::channel::<Event>();
        self.rx = Some(rx);

        self.opened_input = Some(RefCell::new(input.open(tx.clone())?));

        Self::spawn_term_thread(tx.clone());
        self.term_size = terminal.size()?;

        while self.mode != AppMode::Terminated {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn spawn_term_thread(tx: Sender<Event>) {
        thread::spawn(move || {
            loop {
                match crossterm::event::read() {
                    Ok(event) => tx.send(Event::Term(event)).unwrap(),
                    Err(err) => tx.send(Event::Err(err.into())).unwrap(),
                };
            }
        });
    }

    fn handle_events(&mut self) -> Result<()> {
        match self.rx.as_ref().unwrap().recv().unwrap() {
            Event::Term(event) => {
                self.handle_crossterm_events(event)?;
            }
            e @ (Event::NewLines(_) | Event::EOF) => self.opened_input_mut().handle_event(e)?,
            Event::Err(error) => return Err(error),
            Event::NewLines(items) => todo!(),
            Event::EOF => todo!(),
            Event::ReaderThreadErrReturned => {
                let reader_thread = self.opened_input.take().unwrap().into_inner().reader;
                if reader_thread.is_finished() {
                    let res = reader_thread.join().unwrap();
                    match res {
                        Ok(_) => unreachable!(),
                        Err(err) => {
                            return Err(eyre!("reader thread failed {}", err));
                        }
                    }
                }
            }
        };
        Ok(())
    }

    fn handle_crossterm_events(&mut self, event: crossterm::event::Event) -> Result<()> {
        match event {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            crossterm::event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key_event(key)
            }
            crossterm::event::Event::Mouse(_) => {}
            crossterm::event::Event::Resize(colomns, rows) => {
                self.on_term_resize(Size::new(colomns, rows))
            }
            _ => {}
        }
        Ok(())
    }

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

    fn opened_input(&self) -> Ref<OpenedInput> {
        self.opened_input.as_ref().unwrap().borrow()
    }

    fn opened_input_mut(&self) -> RefMut<OpenedInput> {
        self.opened_input.as_ref().unwrap().borrow_mut()
    }

    fn current_max_line(&self) -> usize {
        let mut opened_input = self.opened_input_mut();
        opened_input
            .current_total_lines()
            .saturating_sub(self.term_height())
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
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let current_line = self.current_line;
        let term_hight = self.term_height();
        let mut opened_input = self.opened_input_mut();

        let lines = opened_input.lines(current_line, term_hight).unwrap();
        Paragraph::new(lines).white().render(area, buf);
        log::trace!("buffer {:?}", buf);
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
