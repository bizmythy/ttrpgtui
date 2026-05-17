use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use ratatui_textarea::TextArea;
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::Component;
use crate::{
    action::Action,
    config::Config,
    storage::{self, EncounterInfo, SessionInfo},
};

type TreeIdentifier = String;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PickerInput {
    SessionName,
    EncounterName,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct SessionNode {
    session: SessionInfo,
    encounters: Vec<EncounterInfo>,
}

/// Unified session/encounter tree for opening file-backed encounters.
pub struct SessionPicker {
    data_dir: std::path::PathBuf,
    active: bool,
    can_close: bool,
    input: Option<PickerInput>,
    nodes: Vec<SessionNode>,
    tree_state: TreeState<TreeIdentifier>,
    textarea: TextArea<'static>,
    message: Option<String>,
    current_session_dir: Option<String>,
    current_encounter_file: Option<String>,
}

impl SessionPicker {
    pub fn new() -> Self {
        Self {
            data_dir: std::path::PathBuf::new(),
            active: true,
            can_close: false,
            input: None,
            nodes: Vec::new(),
            tree_state: TreeState::default(),
            textarea: TextArea::default(),
            message: None,
            current_session_dir: None,
            current_encounter_file: None,
        }
    }

    fn activate(&mut self) -> color_eyre::Result<()> {
        self.active = true;
        self.input = None;
        self.message = None;
        self.refresh_tree()?;
        self.select_current_or_first();
        Ok(())
    }

    fn refresh_tree(&mut self) -> color_eyre::Result<()> {
        let sessions = storage::list_sessions(&self.data_dir)?;
        self.nodes = sessions
            .into_iter()
            .map(|session| {
                let encounters = storage::list_encounters(&session)?;
                Ok(SessionNode {
                    session,
                    encounters,
                })
            })
            .collect::<color_eyre::Result<Vec<_>>>()?;

        for node in &self.nodes {
            self.tree_state.open(vec![session_id(&node.session)]);
        }
        Ok(())
    }

    fn select_current_or_first(&mut self) {
        if let (Some(session_dir), Some(encounter_file)) = (
            self.current_session_dir.as_deref(),
            self.current_encounter_file.as_deref(),
        ) {
            if self.find_encounter(session_dir, encounter_file).is_some() {
                self.tree_state.select(vec![
                    format_session_id(session_dir),
                    format_encounter_id(encounter_file),
                ]);
                self.tree_state.open(vec![format_session_id(session_dir)]);
                return;
            }
        }

        if let Some(first) = self.nodes.first() {
            self.tree_state.select(vec![session_id(&first.session)]);
        } else {
            self.tree_state.select(Vec::new());
        }
    }

    fn selected_session(&self) -> Option<&SessionInfo> {
        let selected = self.tree_state.selected();
        selected
            .first()
            .and_then(|id| parse_session_id(id))
            .and_then(|dir| self.find_session(dir))
    }

    fn selected_encounter(&self) -> Option<(&SessionInfo, &EncounterInfo)> {
        let selected = self.tree_state.selected();
        let session_dir = selected.first().and_then(|id| parse_session_id(id))?;
        let encounter_file = selected.get(1).and_then(|id| parse_encounter_id(id))?;
        self.find_encounter(session_dir, encounter_file)
    }

    fn find_session(&self, session_dir: &str) -> Option<&SessionInfo> {
        self.nodes
            .iter()
            .find(|node| node.session.dir_name == session_dir)
            .map(|node| &node.session)
    }

    fn find_encounter(
        &self,
        session_dir: &str,
        encounter_file: &str,
    ) -> Option<(&SessionInfo, &EncounterInfo)> {
        let node = self
            .nodes
            .iter()
            .find(|node| node.session.dir_name == session_dir)?;
        let encounter = node
            .encounters
            .iter()
            .find(|encounter| encounter.file_name == encounter_file)?;
        Some((&node.session, encounter))
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
                self.refresh_tree()?;
                self.tree_state.select(vec![session_id(&session)]);
                self.tree_state.open(vec![session_id(&session)]);
            }
            PickerInput::EncounterName => {
                let Some(session) = self.selected_session().cloned() else {
                    color_eyre::eyre::bail!("select a session before creating an encounter");
                };
                let encounter = storage::create_encounter(&session, name)?;
                self.refresh_tree()?;
                self.tree_state
                    .select(vec![session_id(&session), encounter_id(&encounter)]);
                self.tree_state.open(vec![session_id(&session)]);
                self.input = None;
                return self.load_encounter(session, encounter);
            }
        }

        self.input = None;
        Ok(Some(Action::Render))
    }

    fn open_selected(&mut self) -> color_eyre::Result<Option<Action>> {
        if let Some((session, encounter)) = self.selected_encounter() {
            return self.load_encounter(session.clone(), encounter.clone());
        }

        let selected = self.tree_state.selected().to_vec();
        if selected.is_empty() && !self.nodes.is_empty() {
            self.tree_state.key_down();
        } else {
            self.tree_state.toggle_selected();
        }
        Ok(Some(Action::Render))
    }

    fn load_encounter(
        &mut self,
        session: SessionInfo,
        encounter: EncounterInfo,
    ) -> color_eyre::Result<Option<Action>> {
        let persisted = storage::load_encounter(&encounter)?;
        self.active = false;
        self.can_close = true;
        self.current_session_dir = Some(session.dir_name.clone());
        self.current_encounter_file = Some(encounter.file_name.clone());
        Ok(Some(Action::LoadEncounter {
            session_dir: session.dir_name,
            encounter_file: encounter.file_name,
            encounter_name: persisted.name,
            creatures: persisted.creatures,
        }))
    }

    fn close_or_back_out(&mut self) {
        if self.input.take().is_some() {
            return;
        }
        if self.can_close {
            self.active = false;
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        let [title_area, body_area, footer_area] = ratatui::layout::Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .areas(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "Sessions & encounters",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    self.data_dir.display().to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(Block::default().borders(Borders::BOTTOM)),
            title_area,
        );

        self.render_tree(frame, body_area.inner(Margin::new(1, 0)));
        self.render_footer(frame, footer_area);

        if self.input.is_some() {
            let popup = area.centered(Constraint::Min(48), Constraint::Length(3));
            frame.render_widget(Clear, popup);
            frame.render_widget(&self.textarea, popup);
        }
    }

    fn render_tree(&mut self, frame: &mut Frame, area: Rect) {
        if self.nodes.is_empty() {
            frame.render_widget(
                Paragraph::new("No sessions yet. Press s to create one.")
                    .block(Block::bordered().title("Library"))
                    .style(Style::default().fg(Color::DarkGray)),
                area,
            );
            return;
        }

        let items = self.tree_items();
        let tree = Tree::new(&items)
            .expect("session tree identifiers should be unique")
            .block(Block::bordered().title("Library"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("› ")
            .node_closed_symbol("▸ ")
            .node_open_symbol("▾ ")
            .node_no_children_symbol("  ");
        frame.render_stateful_widget(tree, area, &mut self.tree_state);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help = if self.can_close {
            "↑/↓ or j/k move • ←/→ collapse/expand • click selects/opens • Enter open • n new encounter • s new session • Esc close • q quit"
        } else {
            "↑/↓ or j/k move • ←/→ collapse/expand • click selects/opens • Enter open • n new encounter • s new session • q quit"
        };
        let text = self.message.clone().unwrap_or_else(|| help.to_string());
        let style = if self.message.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(text).style(style).wrap(Wrap { trim: true }),
            area,
        );
    }

    fn tree_items(&self) -> Vec<TreeItem<'static, TreeIdentifier>> {
        self.nodes
            .iter()
            .map(|node| {
                let children = node
                    .encounters
                    .iter()
                    .map(|encounter| {
                        let style = if self.current_session_dir.as_deref()
                            == Some(node.session.dir_name.as_str())
                            && self.current_encounter_file.as_deref()
                                == Some(encounter.file_name.as_str())
                        {
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        TreeItem::new_leaf(
                            encounter_id(encounter),
                            Line::from(vec![Span::styled(encounter.name.clone(), style)]),
                        )
                    })
                    .collect();
                TreeItem::new(session_id(&node.session), session_line(node), children)
                    .expect("encounter identifiers should be unique per session")
            })
            .collect()
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
        self.activate()?;
        Ok(())
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::OpenSessionPicker => self.activate()?,
            Action::LoadEncounter {
                session_dir,
                encounter_file,
                ..
            } => {
                self.current_session_dir = Some(session_dir);
                self.current_encounter_file = Some(encounter_file);
                self.can_close = true;
                self.active = false;
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if !self.active {
            return Ok(None);
        }

        if self.input.is_some() {
            if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.input = None;
                return Ok(Some(Action::Render));
            }

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

        if key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.close_or_back_out();
            return Ok(Some(Action::Render));
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.tree_state.key_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tree_state.key_up();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.tree_state.key_right();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.tree_state.key_left();
            }
            KeyCode::Char(' ') => {
                self.tree_state.toggle_selected();
            }
            KeyCode::Enter => {
                return self.open_selected().or_else(|error| {
                    self.message = Some(error.to_string());
                    Ok(Some(Action::Error(error.to_string())))
                });
            }
            KeyCode::Char('n') => self.open_input(PickerInput::EncounterName),
            KeyCode::Char('s') => self.open_input(PickerInput::SessionName),
            KeyCode::Esc => self.close_or_back_out(),
            _ => return Ok(None),
        }

        Ok(Some(Action::Render))
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> color_eyre::Result<Option<Action>> {
        if !self.active || self.input.is_some() {
            return Ok(None);
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.tree_state
                    .click_at(Position::new(mouse.column, mouse.row));
                if let Some((session, encounter)) = self.selected_encounter() {
                    return self
                        .load_encounter(session.clone(), encounter.clone())
                        .or_else(|error| {
                            self.message = Some(error.to_string());
                            Ok(Some(Action::Error(error.to_string())))
                        });
                }
            }
            MouseEventKind::ScrollDown => {
                self.tree_state.scroll_down(3);
            }
            MouseEventKind::ScrollUp => {
                self.tree_state.scroll_up(3);
            }
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

fn session_line(node: &SessionNode) -> Line<'static> {
    let encounter_count = node.encounters.len();
    Line::from(vec![
        Span::styled(node.session.date.clone(), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(
            node.session.name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  ({encounter_count})")),
    ])
}

fn session_id(session: &SessionInfo) -> TreeIdentifier {
    format_session_id(&session.dir_name)
}

fn encounter_id(encounter: &EncounterInfo) -> TreeIdentifier {
    format_encounter_id(&encounter.file_name)
}

fn format_session_id(dir_name: &str) -> TreeIdentifier {
    format!("session:{dir_name}")
}

fn format_encounter_id(file_name: &str) -> TreeIdentifier {
    format!("encounter:{file_name}")
}

fn parse_session_id(id: &str) -> Option<&str> {
    id.strip_prefix("session:")
}

fn parse_encounter_id(id: &str) -> Option<&str> {
    id.strip_prefix("encounter:")
}

fn input_block(title: &'static str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightBlue))
        .title(title)
}
