use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::App,
    input::{AppMode, HealthOperation},
};

pub fn update(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::Normal => update_normal(app, key_event),
        AppMode::HealthInput(_) => update_health_input(app, key_event),
        AppMode::InitiativeInput(_) => update_initiative_input(app, key_event),
        AppMode::RenameInput(_) => update_rename_input(app, key_event),
        AppMode::NewCreature(_) => update_new_creature(app, key_event),
    }
}

fn update_normal(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => app.clear_selection(),
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') | KeyCode::Char('C') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.quit()
        }

        KeyCode::Char('j') | KeyCode::Down => app.move_next(),
        KeyCode::Char('k') | KeyCode::Up => app.move_previous(),
        KeyCode::Char('g') => app.move_first(),
        KeyCode::Char('G') => app.move_last(),
        KeyCode::Char(' ') => app.toggle_hovered_selection(),
        KeyCode::Char('=') | KeyCode::Char('+') => app.open_health_input(HealthOperation::Add),
        KeyCode::Char('-') | KeyCode::Char('_') => app.open_health_input(HealthOperation::Subtract),
        KeyCode::Char('i') => app.open_initiative_input(),
        KeyCode::Char('n') => app.open_new_creature_form(),
        KeyCode::Char('u') => app.undo(),
        KeyCode::Char('r') | KeyCode::Char('R')
            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.redo()
        }
        KeyCode::Char('r') => app.open_rename_input(),
        _ => {}
    };
}

fn update_health_input(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Enter => app.submit_health_input(),
        _ => app.route_textarea_key(key_event),
    }
}

fn update_initiative_input(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Enter => app.submit_initiative_input(),
        _ => app.route_textarea_key(key_event),
    }
}

fn update_rename_input(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Enter => app.submit_rename_input(),
        _ => app.route_textarea_key(key_event),
    }
}

fn update_new_creature(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Enter => app.submit_new_creature_form(),
        KeyCode::BackTab => app.focus_previous_new_creature_field(),
        KeyCode::Tab => app.focus_next_new_creature_field(),
        _ => app.route_textarea_key(key_event),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::update;
    use crate::{
        app::App,
        input::{AppMode, HealthOperation},
    };

    #[test]
    fn space_toggles_hovered_selection() {
        let mut app = App::new();
        let id = app.hovered_id().unwrap();

        update(
            &mut app,
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
        );
        assert!(app.selected.contains(&id));

        update(
            &mut app,
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
        );
        assert!(!app.selected.contains(&id));
    }

    #[test]
    fn plus_opens_add_health_input_for_current_target() {
        let mut app = App::new();
        let id = app.hovered_id().unwrap();

        update(
            &mut app,
            KeyEvent::new(KeyCode::Char('+'), KeyModifiers::SHIFT),
        );

        let AppMode::HealthInput(input) = app.mode else {
            panic!("expected health input mode");
        };
        assert_eq!(input.operation, HealthOperation::Add);
        assert_eq!(input.target_ids, vec![id]);
    }

    #[test]
    fn r_opens_rename_only_without_multiselect() {
        let mut app = App::new();

        update(
            &mut app,
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
        );
        assert!(matches!(app.mode, AppMode::RenameInput(_)));

        app.cancel_input();
        app.toggle_hovered_selection();
        update(
            &mut app,
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
        );
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn left_and_right_do_not_move_between_columns_or_rows() {
        let mut app = App::new();
        let hovered = app.hovered;

        update(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        update(&mut app, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));

        assert_eq!(app.hovered, hovered);
    }

    #[test]
    fn escape_clears_selection_without_quitting() {
        let mut app = App::new();
        app.toggle_hovered_selection();

        update(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(app.selected.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn movement_wraps_between_first_and_last_rows() {
        let mut app = App::new();
        app.move_last();

        update(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.hovered, Some(0));

        update(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.hovered, app.creatures.len().checked_sub(1));
    }

    #[test]
    fn i_updates_initiative_and_keeps_target_hovered_after_resort() {
        let mut app = App::new();
        app.move_last();
        let id = app.hovered_id().unwrap();

        update(
            &mut app,
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
        );
        if let AppMode::InitiativeInput(input) = &mut app.mode {
            input.textarea.insert_str("99");
        } else {
            panic!("expected initiative input mode");
        }
        update(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.creatures.get(id).unwrap().initiative, Some(99));
        assert_eq!(app.hovered_id(), Some(id));
    }
}
