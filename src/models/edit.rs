use undo::Edit;

use super::creature::{Creature, CreatureId, Creatures};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthChange {
    id: CreatureId,
    before: i32,
    after: i32,
}

impl HealthChange {
    pub fn new(id: CreatureId, before: i32, after: i32) -> Self {
        Self { id, before, after }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitiativeChange {
    id: CreatureId,
    before: Option<i32>,
    after: Option<i32>,
}

impl InitiativeChange {
    pub fn new(id: CreatureId, before: Option<i32>, after: Option<i32>) -> Self {
        Self { id, before, after }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreatureEdit {
    AdjustHealth {
        changes: Vec<HealthChange>,
    },
    SetInitiative {
        changes: Vec<InitiativeChange>,
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
            Self::SetInitiative { changes } => {
                for change in changes {
                    if let Some(creature) = target.get_mut(change.id) {
                        creature.set_initiative(change.after);
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
            Self::SetInitiative { changes } => {
                for change in changes {
                    if let Some(creature) = target.get_mut(change.id) {
                        creature.set_initiative(change.before);
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
