use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use ratatui_textarea::TextArea;

use super::Component;
use crate::{
    action::Action,
    config::Config,
    storage::{self, EncounterInfo, SessionInfo},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PickerStep {
    Session,
    Encounter,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PickerInput {
    SessionName,
    EncounterName,
}

/// Startup picker for selecting or creating session/encounter files.
pub struct SessionPicker {
    data_dir: std::path::PathBuf,
    active: bool,
    step: PickerStep,
    input: Option<PickerInput>,
    sessions: Vec<SessionInfo>,
    encounters: Vec<EncounterInfo>,
    selected_session: Option<SessionInfo>,
    session_state: ListState,
    encounter_state: ListState,
    textarea: TextArea<'static>,
    message: Option<String>,
}

impl SessionPicker {
    pub fn new() -> Self {
        Self {
            data_dir: std::path::PathBuf::new(),
            active: true,
            step: PickerStep::Session,
            input: None,
            sessions: Vec::new(),
            encounters: Vec::new(),
            selected_session: None,
            session_state: ListState::default(),
            encounter_state: ListState::default(),
            textarea: TextArea::default(),
            message: None,
        }
    }

    fn refresh_sessions(&mut self) -> color_eyre::Result<()> {
        self.sessions = storage::list_sessions(&self.data_dir)?;
        clamp_selection(&mut self.session_state, self.sessions.len());
        Ok(())
    }

    fn refresh_encounters(&mut self) -> color_eyre::Result<()> {
        self.encounters = self
            .selected_session
            .as_ref()
            .map(storage::list_encounters)
            .transpose()?
            .unwrap_or_default();
        clamp_selection(&mut self.encounter_state, self.encounters.len());
        Ok(())
    }

    fn selected_session(&self) -> Option<SessionInfo> {
        self.session_state
            .selected()
            .and_then(|index| self.sessions.get(index))
            .cloned()
    }

    fn selected_encounter(&self) -> Option<EncounterInfo> {
        self.encounter_state
            .selected()
            .and_then(|index| self.encounters.get(index))
            .cloned()
    }

    fn open_input(&mut self, input: PickerInput) {
        self.input = Some(input);
        self.textarea = TextArea::default();
        self.textarea.set_block(input_block(match input {
            PickerInput::SessionName => "New session name",
            PickerInput::EncounterName => "New encounter name",
        }));
        self.message = None;
    }

    fn submit_input(&mut self) -> color_eyre::Result<Option<Action>> {
        let Some(input) = self.input else {
            return Ok(Some(Action::Render));
        };
        let name = self.textarea.lines().join(" ");
        let name = name.trim();

        match input {
            PickerInput::SessionName => {
                let session = storage::create_session(&self.data_dir, name)?;
                self.refresh_sessions()?;
                if let Some(index) = self
                    .sessions
                    .iter()
                    .position(|candidate| candidate.dir_name == session.dir_name)
                {
                    self.session_state.select(Some(index));
                }
                self.selected_session = Some(session);
                self.refresh_encounters()?;
                self.step = PickerStep::Encounter;
            }
            PickerInput::EncounterName => {
                let Some(session) = self.selected_session.clone() else {
                    color_eyre::eyre::bail!("select a session before creating an encounter");
                };
                let encounter = storage::create_encounter(&session, name)?;
                self.refresh_encounters()?;
                if let Some(index) = self
                    .encounters
                    .iter()
                    .position(|candidate| candidate.file_name == encounter.file_name)
                {
                    self.encounter_state.select(Some(index));
                }
                self.input = None;
                return self.load_encounter(encounter);
            }
        }

        self.input = None;
        Ok(Some(Action::Render))
    }

    fn select_session(&mut self) -> color_eyre::Result<Option<Action>> {
        let Some(session) = self.selected_session() else {
            self.open_input(PickerInput::SessionName);
            return Ok(Some(Action::Render));
        };
        self.selected_session = Some(session);
        self.refresh_encounters()?;
        self.step = PickerStep::Encounter;
        Ok(Some(Action::Render))
    }

    fn select_encounter(&mut self) -> color_eyre::Result<Option<Action>> {
        let Some(encounter) = self.selected_encounter() else {
            self.open_input(PickerInput::EncounterName);
            return Ok(Some(Action::Render));
        };
        self.load_encounter(encounter)
    }

    fn load_encounter(&mut self, encounter: EncounterInfo) -> color_eyre::Result<Option<Action>> {
        let Some(session) = self.selected_session.clone() else {
            color_eyre::eyre::bail!("select a session before opening an encounter");
        };
        let persisted = storage::load_encounter(&encounter)?;
        self.active = false;
        Ok(Some(Action::LoadEncounter {
            session_dir: session.dir_name,
            encounter_file: encounter.file_name,
            encounter_name: persisted.name,
            creatures: persisted.creatures,
        }))
    }

    fn move_next(&mut self) {
        match self.step {
            PickerStep::Session => {
                select_next_wrapping(&mut self.session_state, self.sessions.len())
            }
            PickerStep::Encounter => {
                select_next_wrapping(&mut self.encounter_state, self.encounters.len())
            }
        }
    }

    fn move_previous(&mut self) {
        match self.step {
            PickerStep::Session => {
                select_previous_wrapping(&mut self.session_state, self.sessions.len())
            }
            PickerStep::Encounter => {
                select_previous_wrapping(&mut self.encounter_state, self.encounters.len())
            }
        }
    }

    fn go_back(&mut self) {
        match self.input.take() {
            Some(_) => {}
            None if self.step == PickerStep::Encounter => {
                self.step = PickerStep::Session;
                self.selected_session = None;
                self.encounters.clear();
            }
            None => {}
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        let [title_area, body_area, footer_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .areas(area);

        let title = match self.step {
            PickerStep::Session => "Choose a session",
            PickerStep::Encounter => "Choose an encounter",
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(
                    self.data_dir.display().to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(Block::default().borders(Borders::BOTTOM)),
            title_area,
        );

        match self.step {
            PickerStep::Session => self.render_sessions(frame, body_area),
            PickerStep::Encounter => self.render_encounters(frame, body_area),
        }

        let help = match self.step {
            PickerStep::Session => "j/k or ↑/↓ move • Enter open • n new session • q quit",
            PickerStep::Encounter => {
                "j/k or ↑/↓ move • Enter open • n new encounter • Esc back • q quit"
            }
        };
        let footer = self.message.clone().unwrap_or_else(|| help.to_string());
        let style = if self.message.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(Paragraph::new(footer).style(style), footer_area);

        if self.input.is_some() {
            let popup = area.centered(Constraint::Min(48), Constraint::Length(3));
            frame.render_widget(Clear, popup);
            frame.render_widget(&self.textarea, popup);
        }
    }

    fn render_sessions(&mut self, frame: &mut Frame, area: Rect) {
        let items = if self.sessions.is_empty() {
            vec![ListItem::new("No sessions yet — press n to create one")]
        } else {
            self.sessions
                .iter()
                .map(|session| {
                    ListItem::new(Line::from(vec![
                        Span::styled(&session.date, Style::default().fg(Color::Cyan)),
                        Span::raw("  "),
                        Span::raw(&session.name),
                    ]))
                })
                .collect()
        };
        let list = List::new(items)
            .block(Block::bordered().title("Sessions"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("› ");
        frame.render_stateful_widget(list, area.inner(Margin::new(1, 0)), &mut self.session_state);
    }

    fn render_encounters(&mut self, frame: &mut Frame, area: Rect) {
        let [session_area, list_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        let session_name = self
            .selected_session
            .as_ref()
            .map(|session| format!("{} — {}", session.date, session.name))
            .unwrap_or_else(|| "No session selected".to_string());
        frame.render_widget(
            Paragraph::new(session_name)
                .style(Style::default().fg(Color::Cyan))
                .wrap(Wrap { trim: true }),
            session_area.inner(Margin::new(1, 0)),
        );

        let items = if self.encounters.is_empty() {
            vec![ListItem::new("No encounters yet — press n to create one")]
        } else {
            self.encounters
                .iter()
                .map(|encounter| ListItem::new(encounter.name.clone()))
                .collect()
        };
        let list = List::new(items)
            .block(Block::bordered().title("Encounters"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("› ");
        frame.render_stateful_widget(
            list,
            list_area.inner(Margin::new(1, 0)),
            &mut self.encounter_state,
        );
    }
}

impl Default for SessionPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for SessionPicker {
    fn register_config_handler(&mut self, config: Config) -> color_eyre::Result<()> {
        self.data_dir = config.config.data_dir;
        self.refresh_sessions()?;
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if !self.active {
            return Ok(None);
        }

        if self.input.is_some() {
            match key.code {
                KeyCode::Esc => self.input = None,
                KeyCode::Enter => {
                    return self.submit_input().or_else(|error| {
                        self.message = Some(error.to_string());
                        Ok(Some(Action::Error(error.to_string())))
                    });
                }
                _ => {
                    self.textarea.input(key);
                }
            }
            return Ok(Some(Action::Render));
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_next(),
            KeyCode::Char('k') | KeyCode::Up => self.move_previous(),
            KeyCode::Char('n') => self.open_input(match self.step {
                PickerStep::Session => PickerInput::SessionName,
                PickerStep::Encounter => PickerInput::EncounterName,
            }),
            KeyCode::Enter => {
                return match self.step {
                    PickerStep::Session => self.select_session(),
                    PickerStep::Encounter => self.select_encounter(),
                }
                .or_else(|error| {
                    self.message = Some(error.to_string());
                    Ok(Some(Action::Error(error.to_string())))
                });
            }
            KeyCode::Esc => self.go_back(),
            _ => return Ok(None),
        }

        Ok(Some(Action::Render))
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.active {
            self.render(frame, area);
        }
        Ok(())
    }
}

fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let selected = state.selected().unwrap_or(0).min(len - 1);
    state.select(Some(selected));
}

fn select_next_wrapping(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let next = state.selected().map_or(0, |index| (index + 1) % len);
    state.select(Some(next));
}

fn select_previous_wrapping(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let previous = state
        .selected()
        .map_or(0, |index| if index == 0 { len - 1 } else { index - 1 });
    state.select(Some(previous));
}

fn input_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightBlue))
        .title(title)
}
