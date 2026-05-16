use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

pub fn update(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') | KeyCode::Char('C') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.quit()
        }

        KeyCode::Char('j') | KeyCode::Down => app.creature_table_state.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.creature_table_state.select_previous(),
        KeyCode::Char('l') | KeyCode::Right => app.creature_table_state.select_next_column(),
        KeyCode::Char('h') | KeyCode::Left => app.creature_table_state.select_previous_column(),
        KeyCode::Char('g') => app.creature_table_state.select_first(),
        KeyCode::Char('G') => app.creature_table_state.select_last(),
        _ => {}
    };
}
