use std::collections::BTreeSet;

use ratatui::crossterm::event::KeyEvent;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;
use undo::{Edit, Record};

use crate::creature::{Creature, CreatureId, Creatures};

/// Application.
pub struct App {
    /// should the application exit?
    pub should_quit: bool,
    /// Creature states
    pub creatures: Creatures,
    /// Hovered display row.
    pub hovered: Option<usize>,
    /// First visible row in the encounter list.
    pub scroll_offset: usize,
    /// Multi-selected creatures.
    pub selected: BTreeSet<CreatureId>,
    /// Current interaction mode.
    pub mode: AppMode,
    history: Record<CreatureEdit>,
}

pub enum AppMode {
    Normal,
    HealthInput(Box<HealthInput>),
    RenameInput(Box<RenameInput>),
    NewCreature(Box<NewCreatureForm>),
}

pub struct HealthInput {
    pub operation: HealthOperation,
    pub target_ids: Vec<CreatureId>,
    pub textarea: TextArea<'static>,
    pub error: Option<String>,
}

pub struct RenameInput {
    pub target_id: CreatureId,
    pub textarea: TextArea<'static>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthOperation {
    Add,
    Subtract,
}

pub struct NewCreatureForm {
    pub fields: NewCreatureFields,
    pub active_field: NewCreatureField,
    pub error: Option<String>,
}

pub struct NewCreatureFields {
    pub name: TextArea<'static>,
    pub initiative: TextArea<'static>,
    pub ac: TextArea<'static>,
    pub health: TextArea<'static>,
    pub count: TextArea<'static>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewCreatureField {
    Name,
    Initiative,
    Ac,
    Health,
    Count,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthChange {
    id: CreatureId,
    before: i32,
    after: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreatureEdit {
    AdjustHealth {
        changes: Vec<HealthChange>,
    },
    RenameCreature {
        id: CreatureId,
        before: String,
        after: String,
    },
    AddCreatures {
        creatures: Vec<Creature>,
    },
}

impl Edit for CreatureEdit {
    type Target = Creatures;
    type Output = ();

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            Self::AdjustHealth { changes } => {
                for change in changes {
                    if let Some(creature) = target.get_mut(change.id) {
                        creature.set_health(change.after);
                    }
                }
                target.sort();
            }
            Self::RenameCreature { id, after, .. } => {
                if let Some(creature) = target.get_mut(*id) {
                    creature.name.clone_from(after);
                }
                target.sort();
            }
            Self::AddCreatures { creatures } => {
                for creature in creatures.clone() {
                    target.add_existing(creature);
                }
            }
        }
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            Self::AdjustHealth { changes } => {
                for change in changes {
                    if let Some(creature) = target.get_mut(change.id) {
                        creature.set_health(change.before);
                    }
                }
                target.sort();
            }
            Self::RenameCreature { id, before, .. } => {
                if let Some(creature) = target.get_mut(*id) {
                    creature.name.clone_from(before);
                }
                target.sort();
            }
            Self::AddCreatures { creatures } => {
                let ids: Vec<CreatureId> = creatures.iter().map(|creature| creature.id).collect();
                target.remove_by_ids(&ids);
            }
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        let mut app = Self {
            should_quit: false,
            creatures: Creatures::default(),
            hovered: Some(0),
            scroll_offset: 0,
            selected: BTreeSet::new(),
            mode: AppMode::Normal,
            history: Record::new(),
        };

        // TEMP: test data
        app.creatures
            .add(Creature::new("john", Some(2), Some(12), 35));
        app.creatures
            .add(Creature::new("jane", Some(1), Some(10), 25));
        app.creatures
            .add(Creature::new("horace", Some(3), Some(15), 40));
        app.reconcile_selection_and_hover();

        app
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set should_quit to true to quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn move_next(&mut self) {
        if self.creatures.is_empty() {
            self.hovered = None;
            return;
        }
        self.hovered = Some(
            self.hovered
                .map_or(0, |index| (index + 1).min(self.creatures.len() - 1)),
        );
    }

    pub fn move_previous(&mut self) {
        if self.creatures.is_empty() {
            self.hovered = None;
            return;
        }
        self.hovered = Some(self.hovered.map_or(0, |index| index.saturating_sub(1)));
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

        self.mode = AppMode::HealthInput(Box::new(HealthInput {
            operation,
            target_ids,
            textarea: single_line_textarea("amount", ""),
            error: None,
        }));
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

        let mut textarea = single_line_textarea("name", "Name");
        textarea.insert_str(&creature.name);
        self.mode = AppMode::RenameInput(Box::new(RenameInput {
            target_id,
            textarea,
            error: None,
        }));
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
                    changes.push(HealthChange { id, before, after });
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

    pub fn route_textarea_key(&mut self, key_event: KeyEvent) {
        match &mut self.mode {
            AppMode::HealthInput(input) => {
                input.textarea.input(key_event);
            }
            AppMode::RenameInput(input) => {
                input.textarea.input(key_event);
            }
            AppMode::NewCreature(form) => {
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
        self.mode = AppMode::NewCreature(Box::new(NewCreatureForm::new()));
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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl NewCreatureForm {
    fn new() -> Self {
        Self {
            fields: NewCreatureFields {
                name: single_line_textarea("name", "Name"),
                initiative: single_line_textarea("initiative", "Initiative (optional)"),
                ac: single_line_textarea("AC", "AC (optional)"),
                health: single_line_textarea("health", "Health"),
                count: single_line_textarea("count", "Count (optional)"),
            },
            active_field: NewCreatureField::Name,
            error: None,
        }
    }

    fn active_textarea_mut(&mut self) -> &mut TextArea<'static> {
        match self.active_field {
            NewCreatureField::Name => &mut self.fields.name,
            NewCreatureField::Initiative => &mut self.fields.initiative,
            NewCreatureField::Ac => &mut self.fields.ac,
            NewCreatureField::Health => &mut self.fields.health,
            NewCreatureField::Count => &mut self.fields.count,
        }
    }

    fn parse(&self) -> Result<ParsedNewCreature, String> {
        let name = textarea_value(&self.fields.name).trim().to_string();
        if name.is_empty() {
            return Err("name is required".to_string());
        }

        Ok(ParsedNewCreature {
            name,
            initiative: parse_optional_i32(textarea_value(&self.fields.initiative), "initiative")?,
            ac: parse_optional_positive_i32(textarea_value(&self.fields.ac), "AC")?,
            health: parse_positive_i32(textarea_value(&self.fields.health), "health")?,
            count: parse_optional_positive_usize(textarea_value(&self.fields.count), "count")?
                .unwrap_or(1),
        })
    }
}

impl NewCreatureField {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Initiative,
            Self::Initiative => Self::Ac,
            Self::Ac => Self::Health,
            Self::Health => Self::Count,
            Self::Count => Self::Name,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Name => Self::Count,
            Self::Initiative => Self::Name,
            Self::Ac => Self::Initiative,
            Self::Health => Self::Ac,
            Self::Count => Self::Health,
        }
    }
}

struct ParsedNewCreature {
    name: String,
    initiative: Option<i32>,
    ac: Option<i32>,
    health: i32,
    count: usize,
}

fn single_line_textarea(title: &'static str, placeholder: &'static str) -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue))
            .title(title),
    );
    textarea.set_placeholder_text(placeholder);
    textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));
    textarea
}

pub fn textarea_value(textarea: &TextArea<'_>) -> String {
    textarea.lines().join("\n")
}

fn parse_optional_i32(value: String, label: &str) -> Result<Option<i32>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i32>()
        .map(Some)
        .map_err(|_| format!("{label} must be a number"))
}

fn parse_optional_positive_i32(value: String, label: &str) -> Result<Option<i32>, String> {
    let Some(value) = parse_optional_i32(value, label)? else {
        return Ok(None);
    };
    if value <= 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(Some(value))
}

fn parse_positive_i32(value: String, label: &str) -> Result<i32, String> {
    let value = value.trim();
    let parsed = value
        .parse::<i32>()
        .map_err(|_| format!("{label} must be a positive number"))?;
    if parsed <= 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(parsed)
}

fn parse_optional_positive_usize(value: String, label: &str) -> Result<Option<usize>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{label} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(Some(parsed))
}

fn letter_suffix(mut index: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    chars.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::{App, HealthOperation, letter_suffix};

    #[test]
    fn target_ids_use_selection_when_present() {
        let mut app = App::new();
        let first = app.creatures.id_at(0).unwrap();
        let second = app.creatures.id_at(1).unwrap();

        assert_eq!(app.target_ids(), vec![first]);
        app.hovered = Some(1);
        app.toggle_hovered_selection();
        assert_eq!(app.target_ids(), vec![second]);
    }

    #[test]
    fn rename_is_undoable_and_keeps_target_hovered_after_resort() {
        let mut app = App::new();
        let id = app.hovered_id().unwrap();

        app.open_rename_input();
        if let super::AppMode::RenameInput(input) = &mut app.mode {
            input.textarea.select_all();
            input.textarea.cut();
            input.textarea.insert_str("zzzz");
        }
        app.submit_rename_input();

        assert_eq!(app.creatures.get(id).unwrap().name, "zzzz");
        assert_eq!(app.hovered_id(), Some(id));

        app.undo();
        assert_ne!(app.creatures.get(id).unwrap().name, "zzzz");
        app.redo();
        assert_eq!(app.creatures.get(id).unwrap().name, "zzzz");
    }

    #[test]
    fn undo_redo_restores_clamped_health_exactly() {
        let mut app = App::new();
        let id = app.creatures.id_at(0).unwrap();
        app.selected.clear();
        app.hovered = app.creatures.index_of(id);

        app.open_health_input(HealthOperation::Subtract);
        if let super::AppMode::HealthInput(input) = &mut app.mode {
            input.textarea.insert_str("100");
        }
        app.submit_health_input();
        assert!(app.creatures.get(id).unwrap().get_health() < 0);

        app.open_health_input(HealthOperation::Add);
        if let super::AppMode::HealthInput(input) = &mut app.mode {
            input.textarea.insert_str("999");
        }
        app.submit_health_input();
        let max = app.creatures.get(id).unwrap().get_max_health();
        assert_eq!(app.creatures.get(id).unwrap().get_health(), max);

        app.undo();
        assert!(app.creatures.get(id).unwrap().get_health() < 0);
        app.redo();
        assert_eq!(app.creatures.get(id).unwrap().get_health(), max);
    }

    #[test]
    fn letter_suffixes_continue_after_z() {
        assert_eq!(letter_suffix(0), "A");
        assert_eq!(letter_suffix(25), "Z");
        assert_eq!(letter_suffix(26), "AA");
        assert_eq!(letter_suffix(27), "AB");
    }
}
