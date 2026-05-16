use ratatui::widgets::TableState;

use crate::creature::{Creature, Creatures};

/// Application.
#[derive(Debug, Default)]
pub struct App {
    /// should the application exit?
    pub should_quit: bool,
    /// Creature states
    pub creatures: Creatures,
    /// Creature table state
    pub creature_table_state: TableState,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        let mut app = Self::default();
        app.creature_table_state.select_first();
        app.creature_table_state.select_first_column();

        // TEMP: test data
        app.creatures.add(Creature::new("john", 2, 12, 35));
        app.creatures.add(Creature::new("jane", 1, 10, 25));
        app.creatures.add(Creature::new("horace", 3, 15, 40));

        app
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set should_quit to true to quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
