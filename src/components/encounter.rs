use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use tokio::sync::mpsc::UnboundedSender;
use undo::Record;

use super::Component;
use crate::{
    action::Action,
    creature::{Creature, CreatureId, Creatures},
    edit::{CreatureEdit, HealthChange, InitiativeChange},
    input::{
        AppMode, HealthInput, HealthOperation, InitiativeInput, NewCreatureField, RenameInput,
        letter_suffix, parse_i32, parse_positive_i32, textarea_value,
    },
};

const ROW_HEIGHT: u16 = 3;

/// Encounter tracker component.
pub struct Encounter {
    pub creatures: Creatures,
    pub hovered: Option<usize>,
    pub scroll_offset: usize,
    pub selected: BTreeSet<CreatureId>,
    pub mode: AppMode,
    history: Record<CreatureEdit>,
    command_tx: Option<UnboundedSender<Action>>,
}

impl Encounter {
    pub fn new() -> Self {
        let mut encounter = Self {
            creatures: Creatures::default(),
            hovered: Some(0),
            scroll_offset: 0,
            selected: BTreeSet::new(),
            mode: AppMode::Normal,
            history: Record::new(),
            command_tx: None,
        };

        // Seed data until persistence/import is added.
        encounter
            .creatures
            .add(Creature::new("john", Some(2), Some(12), 35));
        encounter
            .creatures
            .add(Creature::new("jane", Some(1), Some(10), 25));
        encounter
            .creatures
            .add(Creature::new("horace", Some(3), Some(15), 40));
        encounter.reconcile_selection_and_hover();

        encounter
    }

    pub fn move_next(&mut self) {
        if self.creatures.is_empty() {
            self.hovered = None;
            return;
        }
        self.hovered = Some(self.hovered.map_or(0, |index| {
            if index + 1 >= self.creatures.len() {
                0
            } else {
                index + 1
            }
        }));
    }

    pub fn move_previous(&mut self) {
        if self.creatures.is_empty() {
            self.hovered = None;
            return;
        }
        self.hovered = Some(self.hovered.map_or(0, |index| {
            if index == 0 {
                self.creatures.len() - 1
            } else {
                index - 1
            }
        }));
    }

    pub fn move_first(&mut self) {
        self.hovered = (!self.creatures.is_empty()).then_some(0);
    }

    pub fn move_last(&mut self) {
        self.hovered = self.creatures.len().checked_sub(1);
    }

    pub fn toggle_hovered_selection(&mut self) {
        let Some(id) = self.hovered_id() else {
            return;
        };

        if !self.selected.insert(id) {
            self.selected.remove(&id);
        }
    }

    pub fn hovered_id(&self) -> Option<CreatureId> {
        self.hovered.and_then(|index| self.creatures.id_at(index))
    }

    pub fn target_ids(&self) -> Vec<CreatureId> {
        if self.selected.is_empty() {
            self.hovered_id().into_iter().collect()
        } else {
            self.selected.iter().copied().collect()
        }
    }

    pub fn open_health_input(&mut self, operation: HealthOperation) {
        let target_ids = self.target_ids();
        if target_ids.is_empty() {
            return;
        }

        self.mode = AppMode::HealthInput(Box::new(HealthInput::new(operation, target_ids)));
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn open_initiative_input(&mut self) {
        let target_ids = self.target_ids();
        if target_ids.is_empty() {
            return;
        }

        self.mode = AppMode::InitiativeInput(Box::new(InitiativeInput::new(target_ids)));
    }

    pub fn open_rename_input(&mut self) {
        if !self.selected.is_empty() {
            return;
        }
        let Some(target_id) = self.hovered_id() else {
            return;
        };
        let Some(creature) = self.creatures.get(target_id) else {
            return;
        };

        self.mode = AppMode::RenameInput(Box::new(RenameInput::new(target_id, &creature.name)));
    }

    pub fn cancel_input(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn submit_health_input(&mut self) {
        let AppMode::HealthInput(input) = &self.mode else {
            return;
        };

        let amount = match parse_positive_i32(textarea_value(&input.textarea), "amount") {
            Ok(amount) => amount,
            Err(error) => {
                if let AppMode::HealthInput(input) = &mut self.mode {
                    input.error = Some(error);
                }
                return;
            }
        };

        let delta = match input.operation {
            HealthOperation::Add => amount,
            HealthOperation::Subtract => -amount,
        };
        let target_ids = input.target_ids.clone();
        let mut changes = Vec::new();

        for id in target_ids {
            if let Some(creature) = self.creatures.get(id) {
                let mut clone = creature.clone();
                let before = clone.get_health();
                let after = clone.modify_health(delta);
                if before != after {
                    changes.push(HealthChange::new(id, before, after));
                }
            }
        }

        if !changes.is_empty() {
            self.history
                .edit(&mut self.creatures, CreatureEdit::AdjustHealth { changes });
            self.reconcile_selection_and_hover();
        }
        self.mode = AppMode::Normal;
    }

    pub fn submit_initiative_input(&mut self) {
        let AppMode::InitiativeInput(input) = &self.mode else {
            return;
        };

        let initiative = match parse_i32(textarea_value(&input.textarea), "initiative") {
            Ok(initiative) => initiative,
            Err(error) => {
                if let AppMode::InitiativeInput(input) = &mut self.mode {
                    input.error = Some(error);
                }
                return;
            }
        };
        let target_ids = input.target_ids.clone();
        let first_target = target_ids.first().copied();
        let mut changes = Vec::new();

        for id in target_ids {
            if let Some(creature) = self.creatures.get(id) {
                let before = creature.initiative;
                let after = Some(initiative);
                if before != after {
                    changes.push(InitiativeChange::new(id, before, after));
                }
            }
        }

        if !changes.is_empty() {
            self.history
                .edit(&mut self.creatures, CreatureEdit::SetInitiative { changes });
            if let Some(first_target) = first_target {
                self.hovered = self.creatures.index_of(first_target);
            }
            self.reconcile_selection_and_hover();
        }
        self.mode = AppMode::Normal;
    }

    pub fn route_textarea_key(&mut self, key_event: KeyEvent) {
        match &mut self.mode {
            AppMode::HealthInput(input) => {
                if numeric_textarea_key_allowed(key_event) {
                    input.textarea.input(key_event);
                }
            }
            AppMode::InitiativeInput(input) => {
                if numeric_textarea_key_allowed(key_event) {
                    input.textarea.input(key_event);
                }
            }
            AppMode::RenameInput(input) => {
                input.textarea.input(key_event);
            }
            AppMode::NewCreature(form) => {
                if !matches!(form.active_field, NewCreatureField::Name)
                    && !numeric_textarea_key_allowed(key_event)
                {
                    return;
                }
                form.active_textarea_mut().input(key_event);
            }
            AppMode::Normal => {}
        }
    }

    pub fn submit_rename_input(&mut self) {
        let AppMode::RenameInput(input) = &self.mode else {
            return;
        };

        let after = textarea_value(&input.textarea).trim().to_string();
        if after.is_empty() {
            if let AppMode::RenameInput(input) = &mut self.mode {
                input.error = Some("name is required".to_string());
            }
            return;
        }

        let Some(creature) = self.creatures.get(input.target_id) else {
            self.mode = AppMode::Normal;
            self.reconcile_selection_and_hover();
            return;
        };
        let before = creature.name.clone();
        let target_id = input.target_id;

        if before != after {
            self.history.edit(
                &mut self.creatures,
                CreatureEdit::RenameCreature {
                    id: target_id,
                    before,
                    after,
                },
            );
            self.hovered = self.creatures.index_of(target_id);
            self.reconcile_selection_and_hover();
        }
        self.mode = AppMode::Normal;
    }

    pub fn open_new_creature_form(&mut self) {
        self.mode = AppMode::NewCreature(Box::default());
    }

    pub fn focus_next_new_creature_field(&mut self) {
        if let AppMode::NewCreature(form) = &mut self.mode {
            form.active_field = form.active_field.next();
        }
    }

    pub fn focus_previous_new_creature_field(&mut self) {
        if let AppMode::NewCreature(form) = &mut self.mode {
            form.active_field = form.active_field.previous();
        }
    }

    pub fn submit_new_creature_form(&mut self) {
        let AppMode::NewCreature(form) = &self.mode else {
            return;
        };

        let parsed = match form.parse() {
            Ok(parsed) => parsed,
            Err(error) => {
                if let AppMode::NewCreature(form) = &mut self.mode {
                    form.error = Some(error);
                }
                return;
            }
        };

        let mut creatures = Vec::new();
        for index in 0..parsed.count {
            let name = if parsed.count == 1 {
                parsed.name.clone()
            } else {
                format!("{} {}", parsed.name, letter_suffix(index))
            };
            let id = self.creatures.add(Creature::new(
                name,
                parsed.initiative,
                parsed.ac,
                parsed.health,
            ));
            if let Some(creature) = self.creatures.get(id) {
                creatures.push(creature.clone());
            }
        }

        if !creatures.is_empty() {
            let ids: Vec<CreatureId> = creatures.iter().map(|creature| creature.id).collect();
            self.creatures.remove_by_ids(&ids);
            self.history.edit(
                &mut self.creatures,
                CreatureEdit::AddCreatures { creatures },
            );
            self.selected.clear();
            if let Some(first_id) = ids.first().copied() {
                self.hovered = self.creatures.index_of(first_id);
            }
            self.reconcile_selection_and_hover();
        }
        self.mode = AppMode::Normal;
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.creatures);
        self.reconcile_selection_and_hover();
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.creatures);
        self.reconcile_selection_and_hover();
    }

    pub fn reconcile_selection_and_hover(&mut self) {
        self.selected.retain(|id| self.creatures.contains(*id));
        if self.creatures.is_empty() {
            self.hovered = None;
            self.scroll_offset = 0;
            return;
        }

        self.hovered = Some(
            self.hovered
                .unwrap_or(0)
                .min(self.creatures.len().saturating_sub(1)),
        );
        self.scroll_offset = self
            .scroll_offset
            .min(self.creatures.len().saturating_sub(1));
    }

    pub fn ensure_hover_visible(&mut self, visible_rows: usize) {
        let Some(hovered) = self.hovered else {
            self.scroll_offset = 0;
            return;
        };
        let visible_rows = visible_rows.max(1);
        if hovered < self.scroll_offset {
            self.scroll_offset = hovered;
        } else if hovered >= self.scroll_offset + visible_rows {
            self.scroll_offset = hovered + 1 - visible_rows;
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let [header_area, list_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

        render_header(frame, header_area);
        render_creatures(self, frame, list_area);
        render_footer(frame, footer_area);
        render_popup(self, frame, area, list_area);
    }
}

impl Default for Encounter {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Encounter {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        let action = match self.mode {
            AppMode::Normal => None,
            AppMode::HealthInput(_) => match key.code {
                KeyCode::Esc => Some(Action::CancelInput),
                KeyCode::Enter => Some(Action::SubmitHealthInput),
                _ => Some(Action::TextInput(key)),
            },
            AppMode::InitiativeInput(_) => match key.code {
                KeyCode::Esc => Some(Action::CancelInput),
                KeyCode::Enter => Some(Action::SubmitInitiativeInput),
                _ => Some(Action::TextInput(key)),
            },
            AppMode::RenameInput(_) => match key.code {
                KeyCode::Esc => Some(Action::CancelInput),
                KeyCode::Enter => Some(Action::SubmitRenameInput),
                _ => Some(Action::TextInput(key)),
            },
            AppMode::NewCreature(_) => match key.code {
                KeyCode::Esc => Some(Action::CancelInput),
                KeyCode::Enter => Some(Action::SubmitNewCreatureForm),
                KeyCode::BackTab => Some(Action::FocusPreviousNewCreatureField),
                KeyCode::Tab => Some(Action::FocusNextNewCreatureField),
                _ => Some(Action::TextInput(key)),
            },
        };
        Ok(action)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::ClearSelection => self.clear_selection(),
            Action::MoveNext => self.move_next(),
            Action::MovePrevious => self.move_previous(),
            Action::MoveFirst => self.move_first(),
            Action::MoveLast => self.move_last(),
            Action::ToggleSelection => self.toggle_hovered_selection(),
            Action::OpenAddHealth => self.open_health_input(HealthOperation::Add),
            Action::OpenSubtractHealth => self.open_health_input(HealthOperation::Subtract),
            Action::OpenInitiativeInput => self.open_initiative_input(),
            Action::OpenNewCreatureForm => self.open_new_creature_form(),
            Action::OpenRenameInput => self.open_rename_input(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::CancelInput => self.cancel_input(),
            Action::SubmitHealthInput => self.submit_health_input(),
            Action::SubmitInitiativeInput => self.submit_initiative_input(),
            Action::SubmitRenameInput => self.submit_rename_input(),
            Action::SubmitNewCreatureForm => self.submit_new_creature_form(),
            Action::FocusNextNewCreatureField => self.focus_next_new_creature_field(),
            Action::FocusPreviousNewCreatureField => self.focus_previous_new_creature_field(),
            Action::TextInput(key) => self.route_textarea_key(key),
            _ => {}
        };
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        self.render(frame, area);
        Ok(())
    }
}

fn numeric_textarea_key_allowed(key_event: KeyEvent) -> bool {
    match key_event.code {
        KeyCode::Char(character) => {
            character.is_ascii_digit()
                && !key_event
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        }
        _ => true,
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let columns = row_columns(area.inner(Margin {
        horizontal: 1,
        vertical: 0,
    }));
    let style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(Paragraph::new("Name").style(style), columns[0]);
    frame.render_widget(Paragraph::new("Init").style(style), columns[1]);
    frame.render_widget(Paragraph::new("AC").style(style), columns[2]);
    frame.render_widget(Paragraph::new("Health").style(style), columns[3]);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let help = "j/k move • Space select • +/- health • i initiative • n new • r rename • u undo • Ctrl+R redo • q quit";
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn render_creatures(encounter: &mut Encounter, frame: &mut Frame, area: Rect) {
    let visible_rows = visible_rows(area);
    encounter.ensure_hover_visible(visible_rows);

    for (visible_index, creature) in encounter
        .creatures
        .iter()
        .skip(encounter.scroll_offset)
        .take(visible_rows)
        .enumerate()
    {
        let y = area.y + visible_index as u16 * ROW_HEIGHT;
        if y >= area.bottom() {
            break;
        }
        let row_area = Rect::new(area.x, y, area.width, ROW_HEIGHT.min(area.bottom() - y));
        render_creature_row(
            encounter,
            frame,
            row_area,
            creature,
            encounter.scroll_offset + visible_index,
        );
    }
}

fn visible_rows(area: Rect) -> usize {
    (area.height / ROW_HEIGHT).max(1) as usize
}

fn render_creature_row(
    encounter: &Encounter,
    frame: &mut Frame,
    area: Rect,
    creature: &Creature,
    display_index: usize,
) {
    let is_hovered = encounter.hovered == Some(display_index);
    let is_selected = encounter.selected.contains(&creature.id);
    let border_style = row_border_style(is_hovered, is_selected);

    if is_hovered || is_selected {
        frame.render_widget(Block::bordered().border_style(border_style), area);
    }

    let content_area = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if content_area.is_empty() {
        return;
    }

    let columns = row_columns(content_area);
    let text_style = if creature.is_down() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::White)
    };
    let selected_marker = if is_selected { "● " } else { "  " };

    render_cell(
        frame,
        columns[0],
        Line::from(vec![
            Span::styled(selected_marker, border_style),
            Span::styled(
                creature.name.clone(),
                text_style.add_modifier(Modifier::BOLD),
            ),
        ]),
    );
    render_cell(
        frame,
        columns[1],
        Line::from(Span::styled(
            creature
                .initiative
                .map_or_else(|| "—".to_string(), |initiative| initiative.to_string()),
            text_style,
        )),
    );
    render_cell(
        frame,
        columns[2],
        Line::from(Span::styled(
            creature
                .ac
                .map_or_else(|| "—".to_string(), |ac| ac.to_string()),
            text_style,
        )),
    );
    render_cell(
        frame,
        columns[3],
        Line::from(Span::styled(
            format!("{}/{}", creature.get_health(), creature.get_max_health()),
            text_style,
        )),
    );
}

fn row_columns(area: Rect) -> [Rect; 4] {
    Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(18),
        Constraint::Percentage(12),
        Constraint::Fill(1),
    ])
    .areas(area)
}

fn render_cell(frame: &mut Frame, area: Rect, line: Line<'static>) {
    frame.render_widget(Paragraph::new(line), area);
}

fn row_border_style(is_hovered: bool, is_selected: bool) -> Style {
    match (is_hovered, is_selected) {
        (true, true) => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        (false, true) => Style::default().fg(Color::Green),
        (true, false) => Style::default().fg(Color::Cyan),
        (false, false) => Style::default().fg(Color::DarkGray),
    }
}

fn target_label(creatures: &Creatures, target_ids: &[CreatureId]) -> String {
    if target_ids.len() == 1 {
        target_ids
            .first()
            .and_then(|id| creatures.get(*id))
            .map_or_else(|| "creature".to_string(), |creature| creature.name.clone())
    } else {
        format!("{} creatures", target_ids.len())
    }
}

fn render_popup(encounter: &mut Encounter, frame: &mut Frame, full_area: Rect, list_area: Rect) {
    match &mut encounter.mode {
        AppMode::Normal => {}
        AppMode::HealthInput(input) => {
            let target_label = target_label(&encounter.creatures, &input.target_ids);
            let title = match input.operation {
                HealthOperation::Add => format!("Add HP to {target_label}"),
                HealthOperation::Subtract => format!("Subtract HP from {target_label}"),
            };
            input.textarea.set_block(input_block(title));

            let area = row_popup_area(
                full_area,
                list_area,
                encounter.hovered,
                encounter.scroll_offset,
            );
            render_textarea_popup(
                frame,
                full_area,
                area,
                &input.textarea,
                input.error.as_deref(),
            );
        }
        AppMode::InitiativeInput(input) => {
            let target_label = target_label(&encounter.creatures, &input.target_ids);
            input
                .textarea
                .set_block(input_block(format!("Set initiative for {target_label}")));

            let area = row_popup_area(
                full_area,
                list_area,
                encounter.hovered,
                encounter.scroll_offset,
            );
            render_textarea_popup(
                frame,
                full_area,
                area,
                &input.textarea,
                input.error.as_deref(),
            );
        }
        AppMode::RenameInput(input) => {
            let title = encounter.creatures.get(input.target_id).map_or_else(
                || "Rename creature".to_string(),
                |creature| format!("Rename {}", creature.name),
            );
            input.textarea.set_block(input_block(title));

            let area = row_popup_area(
                full_area,
                list_area,
                encounter.hovered,
                encounter.scroll_offset,
            );
            render_textarea_popup(
                frame,
                full_area,
                area,
                &input.textarea,
                input.error.as_deref(),
            );
        }
        AppMode::NewCreature(form) => {
            style_new_creature_form(form);
            let area = full_area.centered(Constraint::Min(50), Constraint::Length(19));
            frame.render_widget(Clear, area);
            frame.render_widget(
                Block::bordered()
                    .title("New creature")
                    .border_style(Style::default().fg(Color::LightBlue)),
                area,
            );
            let inner = area.inner(Margin {
                horizontal: 1,
                vertical: 1,
            });
            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);
            frame.render_widget(&form.fields.name, chunks[0]);
            frame.render_widget(&form.fields.initiative, chunks[1]);
            frame.render_widget(&form.fields.ac, chunks[2]);
            frame.render_widget(&form.fields.health, chunks[3]);
            frame.render_widget(&form.fields.count, chunks[4]);
            let message = form
                .error
                .clone()
                .unwrap_or_else(|| "Tab/Shift+Tab fields • Enter create • Esc cancel".to_string());
            let style = if form.error.is_some() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            frame.render_widget(
                Paragraph::new(message)
                    .style(style)
                    .wrap(Wrap { trim: true }),
                chunks[5],
            );
        }
    }
}

fn input_block(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightBlue))
        .title(title)
}

fn render_textarea_popup(
    frame: &mut Frame,
    full_area: Rect,
    area: Rect,
    textarea: &ratatui_textarea::TextArea<'_>,
    error: Option<&str>,
) {
    frame.render_widget(Clear, area);
    frame.render_widget(textarea, area);
    if let Some(error) = error {
        let error_area = Rect::new(area.x, area.bottom(), area.width, 1).clamp(full_area);
        frame.render_widget(
            Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red)),
            error_area,
        );
    }
}

fn row_popup_area(
    full_area: Rect,
    list_area: Rect,
    hovered: Option<usize>,
    scroll_offset: usize,
) -> Rect {
    let width = full_area.width.min(36);
    let height = 3;
    let x = full_area.x + full_area.width.saturating_sub(width) / 2;
    let y = hovered
        .and_then(|hovered| hovered.checked_sub(scroll_offset))
        .map(|visible| list_area.y + visible as u16 * ROW_HEIGHT)
        .unwrap_or(full_area.y + full_area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width, height).clamp(full_area)
}

fn style_new_creature_form(form: &mut crate::input::NewCreatureForm) {
    set_field_block(
        &mut form.fields.name,
        "name",
        form.active_field == NewCreatureField::Name,
    );
    set_field_block(
        &mut form.fields.initiative,
        "initiative",
        form.active_field == NewCreatureField::Initiative,
    );
    set_field_block(
        &mut form.fields.ac,
        "AC",
        form.active_field == NewCreatureField::Ac,
    );
    set_field_block(
        &mut form.fields.health,
        "health",
        form.active_field == NewCreatureField::Health,
    );
    set_field_block(
        &mut form.fields.count,
        "count",
        form.active_field == NewCreatureField::Count,
    );
}

fn set_field_block(
    textarea: &mut ratatui_textarea::TextArea<'static>,
    title: &'static str,
    active: bool,
) {
    let style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::LightBlue)
    };
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title),
    );
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{
        Terminal,
        backend::TestBackend,
        style::{Color, Modifier},
    };

    use super::{Component, Encounter};
    use crate::{
        action::Action,
        input::{AppMode, HealthOperation, NewCreatureField, textarea_value},
    };

    fn apply(encounter: &mut Encounter, action: Action) {
        encounter.update(action).unwrap();
    }

    #[test]
    fn target_ids_use_selection_when_present() {
        let mut encounter = Encounter::new();
        let first = encounter.creatures.id_at(0).unwrap();
        let second = encounter.creatures.id_at(1).unwrap();

        assert_eq!(encounter.target_ids(), vec![first]);
        encounter.hovered = Some(1);
        encounter.toggle_hovered_selection();
        assert_eq!(encounter.target_ids(), vec![second]);
    }

    #[test]
    fn rename_is_undoable_and_keeps_target_hovered_after_resort() {
        let mut encounter = Encounter::new();
        let id = encounter.hovered_id().unwrap();

        encounter.open_rename_input();
        if let AppMode::RenameInput(input) = &mut encounter.mode {
            input.textarea.select_all();
            input.textarea.cut();
            input.textarea.insert_str("zzzz");
        }
        encounter.submit_rename_input();

        assert_eq!(encounter.creatures.get(id).unwrap().name, "zzzz");
        assert_eq!(encounter.hovered_id(), Some(id));

        encounter.undo();
        assert_ne!(encounter.creatures.get(id).unwrap().name, "zzzz");
        encounter.redo();
        assert_eq!(encounter.creatures.get(id).unwrap().name, "zzzz");
    }

    #[test]
    fn undo_redo_restores_clamped_health_exactly() {
        let mut encounter = Encounter::new();
        let id = encounter.creatures.id_at(0).unwrap();
        encounter.selected.clear();
        encounter.hovered = encounter.creatures.index_of(id);

        encounter.open_health_input(HealthOperation::Subtract);
        if let AppMode::HealthInput(input) = &mut encounter.mode {
            input.textarea.insert_str("100");
        }
        encounter.submit_health_input();
        assert!(encounter.creatures.get(id).unwrap().get_health() < 0);

        encounter.open_health_input(HealthOperation::Add);
        if let AppMode::HealthInput(input) = &mut encounter.mode {
            input.textarea.insert_str("999");
        }
        encounter.submit_health_input();
        let max = encounter.creatures.get(id).unwrap().get_max_health();
        assert_eq!(encounter.creatures.get(id).unwrap().get_health(), max);

        encounter.undo();
        assert!(encounter.creatures.get(id).unwrap().get_health() < 0);
        encounter.redo();
        assert_eq!(encounter.creatures.get(id).unwrap().get_health(), max);
    }

    #[test]
    fn numeric_input_ignores_letters_and_punctuation() {
        let mut encounter = Encounter::new();

        encounter.open_health_input(HealthOperation::Add);
        for character in ['1', 'a', '-', '.', '2'] {
            encounter
                .route_textarea_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }

        let AppMode::HealthInput(input) = &encounter.mode else {
            panic!("expected health input mode");
        };
        assert_eq!(textarea_value(&input.textarea), "12");
    }

    #[test]
    fn new_creature_numeric_fields_filter_input_but_name_does_not() {
        let mut encounter = Encounter::new();

        encounter.open_new_creature_form();
        if let AppMode::NewCreature(form) = &mut encounter.mode {
            form.active_field = NewCreatureField::Name;
        }
        encounter.route_textarea_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        encounter.route_textarea_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE));

        if let AppMode::NewCreature(form) = &mut encounter.mode {
            form.active_field = NewCreatureField::Health;
        }
        encounter.route_textarea_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        encounter.route_textarea_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
        encounter.route_textarea_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));

        let AppMode::NewCreature(form) = &encounter.mode else {
            panic!("expected new creature mode");
        };
        assert_eq!(textarea_value(&form.fields.name), "a-");
        assert_eq!(textarea_value(&form.fields.health), "4");
    }

    #[test]
    fn space_toggles_hovered_selection() {
        let mut encounter = Encounter::new();
        let id = encounter.hovered_id().unwrap();

        apply(&mut encounter, Action::ToggleSelection);
        assert!(encounter.selected.contains(&id));

        apply(&mut encounter, Action::ToggleSelection);
        assert!(!encounter.selected.contains(&id));
    }

    #[test]
    fn plus_opens_add_health_input_for_current_target() {
        let mut encounter = Encounter::new();
        let id = encounter.hovered_id().unwrap();

        apply(&mut encounter, Action::OpenAddHealth);

        let AppMode::HealthInput(input) = encounter.mode else {
            panic!("expected health input mode");
        };
        assert_eq!(input.operation, HealthOperation::Add);
        assert_eq!(input.target_ids, vec![id]);
    }

    #[test]
    fn r_opens_rename_only_without_multiselect() {
        let mut encounter = Encounter::new();

        apply(&mut encounter, Action::OpenRenameInput);
        assert!(matches!(encounter.mode, AppMode::RenameInput(_)));

        encounter.cancel_input();
        encounter.toggle_hovered_selection();
        apply(&mut encounter, Action::OpenRenameInput);
        assert!(matches!(encounter.mode, AppMode::Normal));
    }

    #[test]
    fn escape_clears_selection_without_quitting() {
        let mut encounter = Encounter::new();
        encounter.toggle_hovered_selection();

        apply(&mut encounter, Action::ClearSelection);

        assert!(encounter.selected.is_empty());
    }

    #[test]
    fn movement_wraps_between_first_and_last_rows() {
        let mut encounter = Encounter::new();
        encounter.move_last();

        apply(&mut encounter, Action::MoveNext);
        assert_eq!(encounter.hovered, Some(0));

        apply(&mut encounter, Action::MovePrevious);
        assert_eq!(encounter.hovered, encounter.creatures.len().checked_sub(1));
    }

    #[test]
    fn i_updates_initiative_and_keeps_target_hovered_after_resort() {
        let mut encounter = Encounter::new();
        encounter.move_last();
        let id = encounter.hovered_id().unwrap();

        apply(&mut encounter, Action::OpenInitiativeInput);
        if let AppMode::InitiativeInput(input) = &mut encounter.mode {
            input.textarea.insert_str("99");
        } else {
            panic!("expected initiative input mode");
        }
        apply(&mut encounter, Action::SubmitInitiativeInput);

        assert_eq!(encounter.creatures.get(id).unwrap().initiative, Some(99));
        assert_eq!(encounter.hovered_id(), Some(id));
    }

    #[test]
    fn hovered_row_draws_border() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut encounter = Encounter::new();

        terminal
            .draw(|frame| encounter.draw(frame, frame.area()).unwrap())
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 1)].symbol(), "┌");
        assert_eq!(buffer[(79, 1)].symbol(), "┐");
    }

    #[test]
    fn down_creature_renders_red() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut encounter = Encounter::new();
        let id = encounter.hovered_id().unwrap();
        encounter.creatures.get_mut(id).unwrap().modify_health(-100);

        terminal
            .draw(|frame| encounter.draw(frame, frame.area()).unwrap())
            .unwrap();
        let buffer = terminal.backend().buffer();

        let john_x = 3;
        let content_y = 2;
        assert_eq!(buffer[(john_x, content_y)].fg, Color::Red);
    }

    #[test]
    fn creature_names_render_bold() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut encounter = Encounter::new();

        terminal
            .draw(|frame| encounter.draw(frame, frame.area()).unwrap())
            .unwrap();
        let buffer = terminal.backend().buffer();

        let first_name_x = 3;
        let first_name_y = 2;
        assert!(
            buffer[(first_name_x, first_name_y)]
                .modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn hovered_selected_row_uses_cyan_border() {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut encounter = Encounter::new();
        encounter.toggle_hovered_selection();

        terminal
            .draw(|frame| encounter.draw(frame, frame.area()).unwrap())
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 1)].fg, Color::Cyan);
    }
}
