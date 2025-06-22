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

pub fn parse_styled_spans(input: String) -> Vec<Span<'static>> {
    enum State {
        Idle,
        SawChar(char),
        SawCharBack(char),
    }

    let mut result = Vec::new();
    let mut current_style = Style::default();
    let mut current_text = String::new();
    let mut state = State::Idle;

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

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match state {
            State::Idle => {
                state = State::SawChar(c);
            }
            State::SawChar(prev) => {
                if c == '\x08' {
                    state = State::SawCharBack(prev);
                } else {
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default(),
                    );
                    current_text.push(prev);
                    state = State::SawChar(c);
                }
            }
            State::SawCharBack(prev) => {
                if prev == c {
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::new().bold(),
                    );
                    current_text.push(c);
                } else if prev == '_' {
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::new().underlined(),
                    );
                    current_text.push(c);
                } else {
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default(),
                    );
                    current_text.push(prev);
                    push_span(
                        &mut result,
                        &mut current_text,
                        &mut current_style,
                        Style::default(),
                    );
                    current_text.push(c);
                }
                state = State::Idle;
            }
        }
    }

    if let State::SawChar(c) = state {
        push_span(
            &mut result,
            &mut current_text,
            &mut current_style,
            Style::default(),
        );
        current_text.push(c);
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
        let data = "\nN\x08NA\x08AM\x08ME\x08E _\x08X plain".to_string();
        let spans = parse_styled_spans(data);
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[1], Span::styled("NAME", Style::new().bold()));
        assert_eq!(spans[3], Span::styled("X", Style::new().underlined()));
        assert_eq!(spans[4], Span::styled(" plain", Style::new()));
    }
}
