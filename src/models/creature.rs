//! Creature data structures for the TTRPG TUI.

use std::{
    cmp::Reverse,
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};

static NEXT_CREATURE_ID: AtomicU64 = AtomicU64::new(1);

/// Stable identifier for a creature.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct CreatureId(u64);

impl CreatureId {
    pub fn new() -> Self {
        Self(NEXT_CREATURE_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn get(self) -> u64 {
        self.0
    }

    fn is_assigned(self) -> bool {
        self.0 != 0
    }

    fn reserve_next_after(self) {
        let next = self.0.saturating_add(1);
        let mut current = NEXT_CREATURE_ID.load(Ordering::Relaxed);
        while current < next {
            match NEXT_CREATURE_ID.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
    }
}

impl fmt::Display for CreatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Individual Creature Data
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Creature {
    pub id: CreatureId,
    pub name: String,
    pub initiative: Option<i32>,
    pub ac: Option<i32>,
    #[serde(default)]
    pub description: String,
    health: i32,
    max_health: i32,
}

impl Creature {
    pub fn new(
        name: impl Into<String>,
        initiative: Option<i32>,
        ac: Option<i32>,
        max_health: i32,
    ) -> Self {
        Self {
            id: CreatureId::default(),
            name: name.into(),
            initiative,
            ac,
            description: String::new(),
            health: max_health,
            max_health,
        }
    }
}

/// Health operations
impl Creature {
    pub fn get_health(&self) -> i32 {
        self.health
    }

    pub fn get_max_health(&self) -> i32 {
        self.max_health
    }

    pub fn set_health(&mut self, health: i32) {
        self.health = health;
    }

    pub fn set_initiative(&mut self, initiative: Option<i32>) {
        self.initiative = initiative;
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    pub fn modify_health(&mut self, delta: i32) -> i32 {
        self.health += delta;

        let max = self.get_max_health();
        if self.health > max {
            self.health = max;
        }

        self.health
    }

    pub fn is_down(&self) -> bool {
        self.health <= 0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Creatures {
    sorted: Vec<Creature>,
}

impl Creatures {
    pub fn add(&mut self, creature: Creature) -> CreatureId {
        self.add_with_id(creature)
    }

    pub fn add_existing(&mut self, creature: Creature) -> CreatureId {
        self.add_with_id(creature)
    }

    fn add_with_id(&mut self, mut creature: Creature) -> CreatureId {
        if !creature.id.is_assigned() {
            creature.id = CreatureId::new();
        }

        let id = creature.id;
        id.reserve_next_after();
        self.sorted.push(creature);
        self.sort();
        id
    }

    pub fn remove_by_ids(&mut self, ids: &[CreatureId]) -> Vec<Creature> {
        let mut removed = Vec::new();
        self.sorted.retain(|creature| {
            if ids.contains(&creature.id) {
                removed.push(creature.clone());
                false
            } else {
                true
            }
        });
        removed.sort_by_key(|creature| creature.id);
        removed
    }

    pub fn get(&self, id: CreatureId) -> Option<&Creature> {
        self.sorted.iter().find(|creature| creature.id == id)
    }

    pub fn get_mut(&mut self, id: CreatureId) -> Option<&mut Creature> {
        self.sorted.iter_mut().find(|creature| creature.id == id)
    }

    pub fn by_display_index(&self, index: usize) -> Option<&Creature> {
        self.sorted.get(index)
    }

    pub fn id_at(&self, index: usize) -> Option<CreatureId> {
        self.by_display_index(index).map(|creature| creature.id)
    }

    pub fn index_of(&self, id: CreatureId) -> Option<usize> {
        self.sorted.iter().position(|creature| creature.id == id)
    }

    pub fn ids_in_display_order(&self) -> Vec<CreatureId> {
        self.sorted.iter().map(|creature| creature.id).collect()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Creature> {
        self.sorted.iter()
    }

    pub fn to_vec(&self) -> Vec<Creature> {
        self.sorted.clone()
    }

    pub fn from_vec(creatures: Vec<Creature>) -> Self {
        let mut collection = Self::default();
        for creature in creatures {
            collection.add_existing(creature);
        }
        collection
    }

    pub fn len(&self) -> usize {
        self.sorted.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sorted.is_empty()
    }

    pub fn contains(&self, id: CreatureId) -> bool {
        self.get(id).is_some()
    }

    pub fn sort(&mut self) {
        self.sorted.sort_by(|left, right| {
            (
                left.initiative.is_none(),
                Reverse(left.initiative.unwrap_or(i32::MIN)),
                left.name.to_lowercase(),
                left.id,
            )
                .cmp(&(
                    right.initiative.is_none(),
                    Reverse(right.initiative.unwrap_or(i32::MIN)),
                    right.name.to_lowercase(),
                    right.id,
                ))
        });
    }
}

impl<'a> IntoIterator for &'a Creatures {
    type Item = &'a Creature;
    type IntoIter = std::slice::Iter<'a, Creature>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{Creature, CreatureId, Creatures};

    #[test]
    fn iter_returns_creatures_in_descending_initiative_order_without_consuming() {
        let mut creatures = Creatures::default();
        creatures.add(Creature::new("slow", Some(3), Some(12), 7));
        creatures.add(Creature::new("fast", Some(18), Some(15), 12));
        creatures.add(Creature::new("middle", Some(10), Some(13), 9));

        let names: Vec<&str> = creatures
            .iter()
            .map(|creature| creature.name.as_str())
            .collect();
        assert_eq!(names, vec!["fast", "middle", "slow"]);

        let names_after_iterating: Vec<&str> = (&creatures)
            .into_iter()
            .map(|creature| creature.name.as_str())
            .collect();
        assert_eq!(names_after_iterating, vec!["fast", "middle", "slow"]);
    }

    #[test]
    fn creatures_without_initiative_sort_last() {
        let mut creatures = Creatures::default();
        creatures.add(Creature::new("unknown", None, None, 7));
        creatures.add(Creature::new("negative", Some(-2), None, 7));
        creatures.add(Creature::new("high", Some(18), None, 7));

        let names: Vec<&str> = creatures
            .iter()
            .map(|creature| creature.name.as_str())
            .collect();
        assert_eq!(names, vec!["high", "negative", "unknown"]);
    }

    #[test]
    fn same_initiative_creatures_sort_by_name_then_id() {
        let mut creatures = Creatures::default();
        let first_alpha = creatures.add(Creature::new("alpha", Some(10), None, 7));
        creatures.add(Creature::new("charlie", Some(10), None, 7));
        creatures.add(Creature::new("bravo", Some(10), None, 7));
        let second_alpha = creatures.add(Creature::new("alpha", Some(10), None, 7));

        let rows: Vec<(&str, CreatureId)> = creatures
            .iter()
            .map(|creature| (creature.name.as_str(), creature.id))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("alpha", first_alpha),
                ("alpha", second_alpha),
                (
                    "bravo",
                    creatures.iter().find(|c| c.name == "bravo").unwrap().id
                ),
                (
                    "charlie",
                    creatures.iter().find(|c| c.name == "charlie").unwrap().id
                ),
            ]
        );
    }

    #[test]
    fn optional_ac_survives_remove_and_readd() {
        let mut creatures = Creatures::default();
        let id = creatures.add(Creature::new("armored", Some(10), Some(17), 7));

        let removed = creatures.remove_by_ids(&[id]);
        assert_eq!(removed[0].ac, Some(17));
        creatures.add_existing(removed[0].clone());

        assert_eq!(
            creatures.get(id).map(|creature| creature.ac),
            Some(Some(17))
        );
    }

    #[test]
    fn clamped_healing_and_negative_health_are_supported() {
        let mut creature = Creature::new("test", Some(1), None, 10);
        creature.modify_health(-15);
        assert_eq!(creature.get_health(), -5);
        assert!(creature.is_down());

        creature.modify_health(100);
        assert_eq!(creature.get_health(), 10);
    }
}
