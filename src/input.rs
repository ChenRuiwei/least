use std::{
    cmp,
    fmt::{self},
    fs::File,
    io::{BufRead, BufReader, stdin},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
};

use color_eyre::eyre::eyre;
use ratatui::text::Line;

use crate::{error::*, event::Event, utils::parse_styled_spans};

#[derive(Debug)]
pub enum InputKind {
    OrdinaryFile(PathBuf),
    StdIn,
}

#[derive(Debug)]
pub struct Input {
    pub kind: InputKind,
}

impl Input {
    pub fn ordinary_file(path: impl AsRef<Path>) -> Self {
        let kind = InputKind::OrdinaryFile(path.as_ref().to_path_buf());
        Input { kind }
    }

    pub fn stdin() -> Self {
        let kind = InputKind::StdIn;
        Input { kind }
    }

    pub fn is_stdin(&self) -> bool {
        matches!(self.kind, InputKind::StdIn)
    }

    pub fn open(self, tx: Sender<Event>) -> Result<OpenedInput> {
        let reader = thread::spawn(move || {
            let mut reader = match self.kind {
                InputKind::StdIn => InputReader::new(stdin().lock(), tx),
                InputKind::OrdinaryFile(path) => {
                    let file = File::open(&path)
                        .map_err(|e| eyre!("'{}': {}", path.to_string_lossy(), e))?;
                    if file.metadata()?.is_dir() {
                        return Err(eyre!("'{}' is a directory.", path.to_string_lossy()));
                    }
                    InputReader::new(BufReader::new(file), tx)
                }
            };

            loop {
                let mut buf = String::new();
                match reader.read_line(&mut buf) {
                    Ok(ret) => {
                        if ret {
                            reader.tx.send(Event::NewLine(buf)).unwrap();
                        } else {
                            reader.tx.send(Event::EOF).unwrap();
                            break;
                        }
                    }
                    Err(err) => reader.tx.send(Event::Err(err)).unwrap(),
                }
            }

            Ok(())
        });

        Ok(OpenedInput {
            reader,
            lines: Vec::new(),
            reached_eof: false,
            current_total_lines: 0,
        })
    }
}

pub struct OpenedInput {
    reader: JoinHandle<Result<()>>,
    lines: Vec<String>,
    reached_eof: bool,
    current_total_lines: usize,
}

impl fmt::Debug for OpenedInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenedInput")
            .field("lines", &self.lines)
            .field("total_lines", &self.current_total_lines)
            .finish()
    }
}

impl OpenedInput {
    fn reached_eof(&self) -> bool {
        self.reached_eof
    }

    pub fn current_total_lines(&mut self) -> usize {
        self.current_total_lines
    }

    pub fn recv_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::NewLine(line) => {
                self.lines.push(line);
                self.current_total_lines = self.lines.len();
            }
            Event::EOF => self.reached_eof = true,
            Event::Err(err) => return Err(err),
            _ => unreachable!(),
        }
        Ok(())
    }

    pub fn lines(&mut self, line_number_start: usize, line_size: usize) -> Result<Vec<Line<'_>>> {
        log::trace!("create lines {line_number_start} {line_size}");

        if line_size == 0 || self.lines.len() < line_number_start {
            return Ok(Vec::new());
        }
        let line_size = cmp::min(line_size, self.lines.len() - line_number_start);
        let mut lines = Vec::with_capacity(line_size);
        for line in self.lines[line_number_start..line_number_start + line_size].iter() {
            let spans = parse_styled_spans(line.clone().into_bytes());
            lines.push(spans);
        }

        Ok(lines.iter().map(|line| Line::from(line.clone())).collect())
    }
}

pub struct InputReader {
    inner: Box<dyn BufRead>,
    tx: Sender<Event>,
}

impl InputReader {
    pub fn new<R: BufRead + 'static>(reader: R, tx: Sender<Event>) -> InputReader {
        Self {
            inner: Box::new(reader),
            tx,
        }
    }

    pub fn read_line(&mut self, buf: &mut String) -> Result<bool> {
        let res = self.inner.read_line(buf).map(|size| size > 0)?;
        log::info!("read line {:?}", buf);
        *buf = buf.replace('\t', "  ");
        Ok(res)
    }
}
