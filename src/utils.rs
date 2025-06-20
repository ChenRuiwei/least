use std::io::Read;

use ratatui::{
    style::{Style, Stylize},
    text::Span,
};

use crate::error::*;

pub fn count_lines<R: Read>(reader: &mut R) -> Result<usize> {
    let mut buf = [0u8; 32 * 1024];
    let mut count = 0;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        count += buf[..n].iter().filter(|&&b| b == b'\n').count();
    }

    Ok(count)
}

pub fn parse_styled_spans(input: Vec<u8>) -> Vec<Span<'static>> {
    enum State {
        Idle,
        SawChar(u8),
        SawCharBack(u8),
    }

    let mut result = Vec::new();
    let mut current_style = Style::default();
    let mut current_text = String::new();

    let mut state = State::Idle;
    let mut i = 0;

    let push_span =
        |result: &mut Vec<Span>, current_text: &mut String, style: &mut Style, new_style: Style| {
            if *style != new_style {
                if !current_text.is_empty() {
                    result.push(Span::styled(current_text.clone(), *style));
                    current_text.clear();
                }
                *style = new_style;
            }
        };

    while i < input.len() {
        let byte = input[i];
        match state {
            State::Idle => {
                state = State::SawChar(byte);
                i += 1;
            }
            State::SawChar(prev) => {
                if byte == 0x08 {
                    state = State::SawCharBack(prev);
                    i += 1;
                } else {
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default(),
                    );
                    current_text.push(prev as char);
                    state = State::SawChar(byte);
                    i += 1;
                }
            }
            State::SawCharBack(prev) => {
                if prev == byte {
                    // X\bX → Bold
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::new().bold(),
                    );
                    current_text.push(byte as char);
                } else if prev == b'_' {
                    // _\bX → Underline
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default().underlined(),
                    );
                    current_text.push(byte as char);
                } else {
                    // Not a recognized pattern, emit prev and handle current as new
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default(),
                    );
                    current_text.push(prev as char);
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default(),
                    );
                    current_text.push(byte as char);
                }
                state = State::Idle;
                i += 1;
            }
        }
    }

    // Flush remaining state
    if let State::SawChar(c) = state {
        push_span(
            &mut result,
            &mut current_text,
            &mut current_style,
            Style::default(),
        );
        current_text.push(c as char);
    }

    if !current_text.is_empty() {
        result.push(Span::styled(current_text, current_style));
    }
    result
}

#[cfg(test)]
mod test {
    use ratatui::{
        style::{Style, Stylize},
        text::Span,
    };

    use crate::utils::parse_styled_spans;

    #[test]
    fn test_backspace_chars() {
        let data = b"\nN\x08NA\x08AM\x08ME\x08E _\x08X plain".to_vec();
        let spans = parse_styled_spans(data);
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[1], Span::styled("NAME", Style::new().bold()));
        assert_eq!(spans[3], Span::styled("X", Style::new().underlined()));
        assert_eq!(spans[4], Span::styled(" plain", Style::new()));
    }
}
