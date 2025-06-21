use core::slice::memchr;
use std::{
    cmp,
    fmt::{self},
    fs::File,
    io::{BufRead, BufReader, stdin},
    os::fd::{AsRawFd, RawFd},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use color_eyre::eyre::eyre;
use mio::{Events, Interest, Poll, Token, unix::SourceFd};
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
            const INPUT: Token = Token(0);
            let mut reader = match self.kind {
                InputKind::StdIn => InputReader::new(stdin().lock(), stdin().as_raw_fd(), tx),
                InputKind::OrdinaryFile(path) => {
                    let file = File::open(&path)
                        .map_err(|e| eyre!("'{}': {}", path.to_string_lossy(), e))?;
                    if file.metadata()?.is_dir() {
                        return Err(eyre!("'{}' is a directory.", path.to_string_lossy()));
                    }
                    let raw_fd = file.as_raw_fd();
                    InputReader::new(BufReader::new(file), raw_fd, tx)
                }
            };

            let mut poll = Poll::new()?;
            let mut events = Events::with_capacity(128);
            poll.registry()
                .register(&mut SourceFd(&reader.raw_fd), INPUT, Interest::READABLE)?;

            let mut lines_batch = Vec::new();
            let mut line_buf = Vec::new();
            let flush_interval = Duration::from_millis(16);
            let mut last_flush = Instant::now();
            loop {
                let timeout = flush_interval
                    .checked_sub(last_flush.elapsed())
                    .unwrap_or_default();
                poll.poll(&mut events, Some(timeout))?;

                for event in &events {
                    if event.token() == INPUT {
                        let buf = reader.inner.fill_buf()?;
                        log::debug!("fill buf {:?}", buf);
                        if buf.is_empty() {
                            // EOF
                            if !line_buf.is_empty() {
                                lines_batch.push(String::from_utf8_lossy(&line_buf).into_owned());
                                line_buf.clear();
                            }
                            if !lines_batch.is_empty() {
                                let _ = reader.tx.send(Event::NewLines(lines_batch));
                            }
                            let _ = reader.tx.send(Event::EOF);
                            return Ok(());
                        }

                        let mut consumed = 0;
                        while let Some(i) = memchr::memchr(b'\n', &buf[consumed..]) {
                            let end = consumed + i + 1;
                            line_buf.extend_from_slice(&buf[consumed..end]);
                            lines_batch.push(String::from_utf8_lossy(&line_buf).into_owned());
                            line_buf.clear();
                            consumed = end;
                        }
                        line_buf.extend_from_slice(&buf[consumed..]);
                        consumed = buf.len();
                        reader.inner.consume(consumed);
                    }
                }

                // timeout: only flush completed lines
                if last_flush.elapsed() >= flush_interval && !lines_batch.is_empty() {
                    let _ = reader
                        .tx
                        .send(Event::NewLines(std::mem::take(&mut lines_batch)));
                    last_flush = Instant::now();
                }
            }
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

    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::NewLines(lines) => {
                log::debug!("received new lines {}", lines.len());
                self.lines.extend(lines);
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
    raw_fd: RawFd,
    tx: Sender<Event>,
}

impl InputReader {
    pub fn new<R: BufRead + 'static>(reader: R, raw_fd: RawFd, tx: Sender<Event>) -> InputReader {
        Self {
            inner: Box::new(reader),
            raw_fd,
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
