use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::models::creature::Creature;

/// Messages that flow through the application's event/update loop.
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    LoadEncounter {
        session_dir: String,
        encounter_file: String,
        encounter_name: String,
        creatures: Vec<Creature>,
    },

    ClearSelection,
    MoveNext,
    MovePrevious,
    MoveFirst,
    MoveLast,
    ToggleSelection,
    OpenAddHealth,
    OpenSubtractHealth,
    OpenInitiativeInput,
    OpenNewCreatureForm,
    OpenRenameInput,
    Undo,
    Redo,

    CancelInput,
    SubmitHealthInput,
    SubmitInitiativeInput,
    SubmitRenameInput,
    SubmitNewCreatureForm,
    FocusNextNewCreatureField,
    FocusPreviousNewCreatureField,
    TextInput(KeyEvent),
}
