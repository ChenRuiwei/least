use crossterm::event::{KeyCode, KeyEvent};

#[derive(Default, Debug, Clone, Copy)]
pub enum KeyState {
    #[default]
    Normal,
    WaitingG,
    WaitingGNumber(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    GoToMain,
    GoToTop,
    GoToBottom,
    GoToLine(usize),
    ScrollUpOneLine,
    ScrollDownOneLine,
    ScrollUpHalfScreen,
    ScrollDownHalfScreen,
    ScrollUpScreen,
    ScrollDownScreen,
    None,
    Quit,
}

impl KeyState {
    pub fn next(self, key: KeyEvent) -> (KeyState, Action) {
        match self {
            KeyState::Normal => match (key.modifiers, key.code) {
                (_, KeyCode::Esc | KeyCode::Char('q')) => (KeyState::Normal, Action::Quit),
                (_, KeyCode::Char('d')) => (KeyState::Normal, Action::ScrollDownHalfScreen),
                (_, KeyCode::Char('u')) => (KeyState::Normal, Action::ScrollUpHalfScreen),
                (_, KeyCode::Char('f')) => (KeyState::Normal, Action::ScrollDownScreen),
                (_, KeyCode::Char('b')) => (KeyState::Normal, Action::ScrollUpScreen),
                (_, KeyCode::Char('j')) => (KeyState::Normal, Action::ScrollDownOneLine),
                (_, KeyCode::Char('k')) => (KeyState::Normal, Action::ScrollUpOneLine),
                (_, KeyCode::Char('g')) => (KeyState::WaitingG, Action::None),
                (_, KeyCode::Char('G')) => (KeyState::Normal, Action::GoToBottom),
                _ => (KeyState::Normal, Action::None),
            },
            KeyState::WaitingG => match (key.modifiers, key.code) {
                (_, KeyCode::Char('g')) => (KeyState::Normal, Action::GoToTop),
                (_, KeyCode::Char(c)) if c.is_ascii_digit() => {
                    let c = c.to_digit(10).unwrap() as usize;
                    let n = c;
                    (KeyState::WaitingGNumber(n), Action::None)
                }
                _ => (KeyState::Normal, Action::None),
            },
            KeyState::WaitingGNumber(n) => match (key.modifiers, key.code) {
                (_, KeyCode::Char(c)) if c.is_ascii_digit() => {
                    let c = c.to_digit(10).unwrap() as usize;
                    let n = n * 10 + c;
                    (KeyState::WaitingGNumber(n), Action::None)
                }
                (_, KeyCode::Enter) => (KeyState::Normal, Action::GoToLine(n)),
                _ => (KeyState::Normal, Action::None),
            },
        }
    }
}
