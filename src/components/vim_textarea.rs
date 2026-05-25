use std::{fmt, io::stdout};

use crossterm::{cursor::SetCursorStyle, event::KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders},
};
use ratatui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};

use crate::models::creature::CreatureId;

/// Modal textarea component using the Vim emulation state machine from
/// ratatui-textarea's `examples/vim.rs`, adapted for embedding in the encounter UI.
pub struct VimTextArea {
    pub target_ids: Vec<CreatureId>,
    pub textarea: TextArea<'static>,
    vim: Vim,
}

impl VimTextArea {
    pub fn new(target_ids: Vec<CreatureId>, initial_text: &str) -> Self {
        let mode = Mode::Insert;
        let mut textarea = TextArea::default();
        textarea.insert_str(initial_text);
        textarea.set_placeholder_text("Description");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));
        textarea.set_cursor_style(mode.cursor_style());

        Self {
            target_ids,
            textarea,
            vim: Vim::new(mode),
        }
    }

    pub fn input(&mut self, key: KeyEvent) {
        let vim = std::mem::replace(&mut self.vim, Vim::new(Mode::Normal));
        self.vim = match vim.transition(key.into(), &mut self.textarea) {
            Transition::Mode(mode) if vim.mode != mode => {
                self.textarea.set_cursor_style(mode.cursor_style());
                Vim::new(mode)
            }
            Transition::Nop | Transition::Mode(_) => vim,
            Transition::Pending(input) => vim.with_pending(input),
        };
    }

    pub fn is_normal_mode(&self) -> bool {
        self.vim.mode == Mode::Normal
    }

    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_block(&mut self, title: String) {
        self.textarea.set_block(self.vim.mode.block(title));
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(&self.textarea, area);
        if self.vim.mode == Mode::Insert {
            let _ = crossterm::execute!(stdout(), SetCursorStyle::SteadyBar);
            let cursor = self.textarea.screen_cursor();
            let inner = area.inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
            if !inner.is_empty() {
                frame.set_cursor_position(Position {
                    x: inner
                        .x
                        .saturating_add(cursor.col as u16)
                        .min(inner.right().saturating_sub(1)),
                    y: inner
                        .y
                        .saturating_add(cursor.row as u16)
                        .min(inner.bottom().saturating_sub(1)),
                });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Replace(bool), // true = replace once (r), false = overtype (R)
    Visual,
    Operator(char),
}

impl Mode {
    fn block<'a>(&self, title: String) -> Block<'a> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue))
            .title(title)
            .title(Line::from(self.to_string()).alignment(Alignment::Right))
    }

    fn cursor_style(&self) -> Style {
        match self {
            Self::Insert => Style::default(),
            Self::Normal => Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::REVERSED),
            Self::Replace(_) => Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::REVERSED),
            Self::Visual => Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::REVERSED),
            Self::Operator(_) => Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::REVERSED),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Replace(_) => write!(f, "REPLACE"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
        }
    }
}

// How the Vim emulation state transitions
enum Transition {
    Nop,
    Mode(Mode),
    Pending(Input),
}

// State of Vim emulation
struct Vim {
    mode: Mode,
    pending: Input, // Pending input to handle a sequence with two keys like gg
}

impl Vim {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            pending: Input::default(),
        }
    }

    fn with_pending(self, pending: Input) -> Self {
        Self {
            mode: self.mode,
            pending,
        }
    }

    fn is_before_line_end(textarea: &TextArea<'_>) -> bool {
        let cursor = textarea.cursor();
        cursor.1 < textarea.lines()[cursor.0].len().saturating_sub(1)
    }

    fn transition(&self, input: Input, textarea: &mut TextArea<'_>) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        match self.mode {
            Mode::Normal | Mode::Visual | Mode::Operator(_) => {
                match input {
                    Input {
                        key: Key::Char('h') | Key::Left,
                        ..
                    } => textarea.move_cursor(CursorMove::Back),
                    Input {
                        key: Key::Char('j') | Key::Down,
                        ..
                    } => textarea.move_cursor(CursorMove::Down),
                    Input {
                        key: Key::Char('k') | Key::Up,
                        ..
                    } => textarea.move_cursor(CursorMove::Up),
                    Input {
                        key: Key::Char('l') | Key::Right,
                        ..
                    } => textarea.move_cursor(CursorMove::Forward),
                    Input {
                        key: Key::Char('w'),
                        ..
                    } => textarea.move_cursor(CursorMove::WordForward),
                    Input {
                        key: Key::Char('e'),
                        ctrl: false,
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::WordEnd);
                        if matches!(self.mode, Mode::Operator(_)) {
                            textarea.move_cursor(CursorMove::Forward); // Include the text under the cursor
                        }
                    }
                    Input {
                        key: Key::Char('b'),
                        ctrl: false,
                        ..
                    } => textarea.move_cursor(CursorMove::WordBack),
                    Input {
                        key: Key::Char('^'),
                        ..
                    } => textarea.move_cursor(CursorMove::Head),
                    Input {
                        key: Key::Char('$'),
                        ..
                    } => textarea.move_cursor(CursorMove::End),
                    Input {
                        key: Key::Char('D'),
                        ..
                    } => {
                        textarea.delete_line_by_end();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('C'),
                        ..
                    } => {
                        textarea.delete_line_by_end();
                        textarea.cancel_selection();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('p'),
                        ..
                    } => {
                        textarea.paste();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('u'),
                        ctrl: false,
                        ..
                    } => {
                        textarea.undo();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('r'),
                        ctrl: true,
                        ..
                    } => {
                        textarea.redo();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('x'),
                        ..
                    } if Self::is_before_line_end(textarea)
                        || textarea.lines()[textarea.cursor().0].is_empty() =>
                    {
                        textarea.delete_next_char();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('i'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('a'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        if Self::is_before_line_end(textarea) {
                            textarea.move_cursor(CursorMove::Forward);
                        }
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('A'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('o'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::End);
                        textarea.insert_newline();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('O'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.insert_newline();
                        textarea.move_cursor(CursorMove::Up);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('I'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::Head);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('J'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        let row = textarea.cursor().0;
                        if row + 1 < textarea.lines().len() {
                            textarea.move_cursor(CursorMove::End);
                            textarea.delete_next_char(); // delete newline
                            textarea.insert_char(' ');
                        }
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('J'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        // Join all lines in selection
                        let (start, end) = {
                            let sel = textarea.selection_range();
                            match sel {
                                Some((s, e)) => (s.0, e.0),
                                None => return Transition::Mode(Mode::Normal),
                            }
                        };
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::Jump(start as u16, 0));
                        for _ in start..end {
                            textarea.move_cursor(CursorMove::End);
                            textarea.delete_next_char();
                            textarea.insert_char(' ');
                        }
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('S'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.delete_line_by_end();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('S'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.move_cursor(CursorMove::Forward);
                        textarea.cut();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('r'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        return Transition::Mode(Mode::Replace(true));
                    }
                    Input {
                        key: Key::Char('R'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        return Transition::Mode(Mode::Replace(false));
                    }
                    Input {
                        key: Key::Char('e'),
                        ctrl: true,
                        ..
                    } => textarea.scroll((1, 0)),
                    Input {
                        key: Key::Char('y'),
                        ctrl: true,
                        ..
                    } => textarea.scroll((-1, 0)),
                    Input {
                        key: Key::Char('d'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::HalfPageDown),
                    Input {
                        key: Key::Char('u'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::HalfPageUp),
                    Input {
                        key: Key::Char('f'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::PageDown),
                    Input {
                        key: Key::Char('b'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::PageUp),
                    Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.start_selection();
                        return Transition::Mode(Mode::Visual);
                    }
                    Input {
                        key: Key::Char('V'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(Mode::Visual);
                    }
                    Input { key: Key::Esc, .. }
                    | Input {
                        key: Key::Char('['),
                        ctrl: true,
                        ..
                    }
                    | Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.cancel_selection();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('g'),
                        ctrl: false,
                        ..
                    } if matches!(
                        self.pending,
                        Input {
                            key: Key::Char('g'),
                            ctrl: false,
                            ..
                        }
                    ) =>
                    {
                        textarea.move_cursor(CursorMove::Top)
                    }
                    Input {
                        key: Key::Char('G'),
                        ctrl: false,
                        ..
                    } => textarea.move_cursor(CursorMove::Bottom),
                    Input {
                        key: Key::Char(c),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Operator(c) => {
                        // Handle yy, dd, cc. (This is not strictly the same behavior as Vim)
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        let cursor = textarea.cursor();
                        textarea.move_cursor(CursorMove::Down);
                        if cursor == textarea.cursor() {
                            textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
                        }
                    }
                    Input {
                        key: Key::Char(op @ ('y' | 'd' | 'c')),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.start_selection();
                        return Transition::Mode(Mode::Operator(op));
                    }
                    Input {
                        key: Key::Char('y'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        textarea.copy();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('d'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        textarea.cut();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('c'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        textarea.cut();
                        return Transition::Mode(Mode::Insert);
                    }
                    input => return Transition::Pending(input),
                }

                // Handle the pending operator
                match self.mode {
                    Mode::Operator('y') => {
                        textarea.copy();
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('d') => {
                        textarea.cut();
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('c') => {
                        textarea.cut();
                        Transition::Mode(Mode::Insert)
                    }
                    _ => Transition::Nop,
                }
            }
            Mode::Insert => match input {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                }
                | Input {
                    key: Key::Char('['),
                    ctrl: true,
                    ..
                } => Transition::Mode(Mode::Normal),
                input => {
                    textarea.input(input); // Use default key mappings in insert mode
                    Transition::Mode(Mode::Insert)
                }
            },
            Mode::Replace(once) => match input {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('['),
                    ctrl: true,
                    ..
                } => Transition::Mode(Mode::Normal),
                Input {
                    key: Key::Char(c),
                    ctrl: false,
                    alt: false,
                    ..
                } => {
                    // Replace the character under the cursor
                    if Self::is_before_line_end(textarea)
                        || textarea.lines()[textarea.cursor().0].len() == textarea.cursor().1
                    {
                        textarea.delete_next_char();
                        textarea.insert_char(c);
                    }
                    if once {
                        Transition::Mode(Mode::Normal)
                    } else {
                        Transition::Mode(Mode::Replace(false))
                    }
                }
                _ => Transition::Mode(if once {
                    Mode::Normal
                } else {
                    Mode::Replace(false)
                }),
            },
        }
    }
}
