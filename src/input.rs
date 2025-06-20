use std::{
    cmp,
    fmt::{self},
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str,
};

use ratatui::text::Line;

use crate::error::*;

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

    pub fn open<R: BufRead + 'static>(self, stdin: R) -> Result<OpenedInput> {
        match self.kind {
            InputKind::StdIn => Ok(OpenedInput {
                kind: OpenedInputKind::StdIn,
                reader: InputReader::new(stdin),
                lines: Vec::new(),
                reached_eof: false,
                current_total_lines: 0,
            }),
            InputKind::OrdinaryFile(path) => Ok(OpenedInput {
                kind: OpenedInputKind::OrdinaryFile(path.clone()),
                reader: {
                    let file = File::open(&path)
                        .map_err(|e| format!("'{}': {}", path.to_string_lossy(), e))?;
                    if file.metadata()?.is_dir() {
                        return Err(format!("'{}' is a directory.", path.to_string_lossy()).into());
                    }
                    InputReader::new(BufReader::new(file))
                },
                lines: Vec::new(),
                reached_eof: false,
                current_total_lines: 0,
            }),
        }
    }
}

#[derive(Debug)]
pub enum OpenedInputKind {
    OrdinaryFile(PathBuf),
    StdIn,
}

pub struct OpenedInput {
    pub kind: OpenedInputKind,
    pub reader: InputReader,
    lines: Vec<Vec<u8>>,
    reached_eof: bool,
    current_total_lines: usize,
}

impl fmt::Debug for OpenedInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenedInput")
            .field("kind", &self.kind)
            .field("lines", &self.lines)
            .field("total_lines", &self.current_total_lines)
            .finish()
    }
}

impl OpenedInput {
    fn reached_eof(&self) -> bool {
        self.reached_eof
    }

    fn advance(&mut self) -> Result<()> {
        if self.reached_eof() {
            return Err(Error::from("reached end"));
        }
        while !self.reached_eof() {
            let mut current_line_buffer: Vec<u8> = Vec::new();
            if self.reader.try_read_line(&mut current_line_buffer)? {
                self.lines.push(current_line_buffer);
            } else {
                self.reached_eof = true
            }
            self.current_total_lines = self.lines.len()
        }
        Ok(())
    }

    pub fn current_total_lines(&mut self) -> usize {
        let _ = self.advance();
        self.current_total_lines
    }

    pub fn lines(&mut self, line_number_start: usize, line_size: usize) -> Result<Vec<Line<'_>>> {
        log::trace!("create lines {line_number_start} {line_size}");
        while !self.reached_eof() && self.lines.len() < line_number_start + line_size {
            let mut current_line_buffer: Vec<u8> = Vec::new();
            if self.reader.read_line(&mut current_line_buffer)? {
                self.lines.push(current_line_buffer);
            } else {
                self.reached_eof = true
            }
            self.current_total_lines = self.lines.len()
        }

        let line_number_end = cmp::min(line_number_start + line_size, self.lines.len());
        log::trace!("line number end {line_number_end}");

        Ok(self.lines[line_number_start..line_number_end]
            .iter()
            .map(|line| Line::from(str::from_utf8(line).expect("checked to be utf8")))
            .collect())
    }
}

pub struct InputReader {
    inner: Box<dyn BufRead>,
}

impl InputReader {
    pub fn new<R: BufRead + 'static>(reader: R) -> InputReader {
        Self {
            inner: Box::new(reader),
        }
    }

    pub fn read_line(&mut self, buf: &mut Vec<u8>) -> Result<bool> {
        let res = self.inner.read_until(b'\n', buf).map(|size| size > 0)?;
        let line = String::from_utf8_lossy(buf);
        let replaced = line.replace('\t', "  ");
        buf.clear();
        buf.extend_from_slice(replaced.as_bytes());
        Ok(res)
    }

    pub fn try_read_line(&mut self, buf: &mut Vec<u8>) -> Result<bool> {
        if self.inner.has_data_left()? {
            self.read_line(buf)
        } else {
            Err(Error::from("no data left now"))
        }
    }
}
