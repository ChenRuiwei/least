use std::{
    cmp,
    fmt::{self},
    io,
    io::BufRead,
    str,
};

use color_eyre::Result;

use ratatui::text::Line;

pub struct InputReader {
    inner: Box<dyn BufRead>,
}

impl InputReader {
    pub fn new<R: BufRead + 'static>(reader: R) -> InputReader {
        InputReader {
            inner: Box::new(reader),
        }
    }

    pub fn read_line(&mut self, buf: &mut Vec<u8>) -> io::Result<bool> {
        let res = self.inner.read_until(b'\n', buf).map(|size| size > 0)?;
        Ok(res)
    }
}

#[derive(Default)]
pub struct InputBuffer {
    lines: Vec<Vec<u8>>,
    pub reader: Option<InputReader>,
    reached_eof: bool,
}

impl fmt::Debug for InputBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("lines", &self.lines)
            .field("reached_eof", &self.reached_eof)
            .finish()
    }
}

impl InputBuffer {
    fn reader_mut(&mut self) -> &mut InputReader {
        self.reader.as_mut().unwrap()
    }

    pub fn lines(&mut self, line_number_start: usize, line_size: usize) -> Result<Vec<Line<'_>>> {
        let line_number_end = if self.reached_eof {
            cmp::min(line_number_start + line_size, self.lines.len())
        } else {
            line_number_start + line_size
        };

        while !self.reached_eof && self.lines.len() < line_number_end {
            let mut current_line_buffer: Vec<u8> = Vec::new();
            if self.reader_mut().read_line(&mut current_line_buffer)? {
                self.lines.push(current_line_buffer);
            } else {
                self.reached_eof = true;
            }
        }

        Ok(self.lines[line_number_start..line_number_end]
            .iter()
            .map(|line| Line::from(str::from_utf8(line).expect("checked to be utf8")))
            .collect())
    }
}
