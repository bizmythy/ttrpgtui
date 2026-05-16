use std::collections::BTreeSet;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use undo::Record;

use crate::creature::{Creature, CreatureId, Creatures};
use crate::edit::{CreatureEdit, HealthChange, InitiativeChange};
use crate::input::{
    AppMode, HealthInput, HealthOperation, InitiativeInput, NewCreatureField, RenameInput,
    letter_suffix, parse_i32, parse_positive_i32, textarea_value,
};

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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
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

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::App;
    use crate::input::{AppMode, HealthOperation, NewCreatureField, textarea_value};

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
        if let AppMode::RenameInput(input) = &mut app.mode {
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
        if let AppMode::HealthInput(input) = &mut app.mode {
            input.textarea.insert_str("100");
        }
        app.submit_health_input();
        assert!(app.creatures.get(id).unwrap().get_health() < 0);

        app.open_health_input(HealthOperation::Add);
        if let AppMode::HealthInput(input) = &mut app.mode {
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
    fn numeric_input_ignores_letters_and_punctuation() {
        let mut app = App::new();

        app.open_health_input(HealthOperation::Add);
        for character in ['1', 'a', '-', '.', '2'] {
            app.route_textarea_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }

        let AppMode::HealthInput(input) = &app.mode else {
            panic!("expected health input mode");
        };
        assert_eq!(textarea_value(&input.textarea), "12");
    }

    #[test]
    fn numeric_input_still_allows_editing_keys() {
        let mut app = App::new();

        app.open_initiative_input();
        for character in ['1', '2'] {
            app.route_textarea_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        app.route_textarea_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        app.route_textarea_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));

        let AppMode::InitiativeInput(input) = &app.mode else {
            panic!("expected initiative input mode");
        };
        assert_eq!(textarea_value(&input.textarea), "13");
    }

    #[test]
    fn new_creature_numeric_fields_filter_input_but_name_does_not() {
        let mut app = App::new();

        app.open_new_creature_form();
        if let AppMode::NewCreature(form) = &mut app.mode {
            form.active_field = NewCreatureField::Name;
        }
        app.route_textarea_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        app.route_textarea_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE));

        if let AppMode::NewCreature(form) = &mut app.mode {
            form.active_field = NewCreatureField::Health;
        }
        app.route_textarea_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        app.route_textarea_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
        app.route_textarea_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));

        let AppMode::NewCreature(form) = &app.mode else {
            panic!("expected new creature mode");
        };
        assert_eq!(textarea_value(&form.fields.name), "a-");
        assert_eq!(textarea_value(&form.fields.health), "4");
    }
}
