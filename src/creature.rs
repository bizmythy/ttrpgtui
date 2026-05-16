//! Creature data structures for the TTRPG TUI.

use std::{cmp::Reverse, fmt};

/// Stable identifier for a creature.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CreatureId(u64);

impl CreatureId {
    pub fn get(self) -> u64 {
        self.0
    }

    fn is_assigned(self) -> bool {
        self.0 != 0
    }
}

impl fmt::Display for CreatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Individual Creature Data
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Creature {
    pub id: CreatureId,
    pub name: String,
    pub initiative: Option<i32>,
    pub ac: Option<i32>,
    health: i32,
    max_health: i32,
    order: u64,
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
            health: max_health,
            max_health,
            order: 0,
        }
    }

    pub fn order(&self) -> u64 {
        self.order
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

#[derive(Debug, Default)]
pub struct Creatures {
    sorted: Vec<Creature>,
    next_id: u64,
    next_order: u64,
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
            self.next_id += 1;
            creature.id = CreatureId(self.next_id);
        } else {
            self.next_id = self.next_id.max(creature.id.get());
        }

        if creature.order == 0 {
            self.next_order += 1;
            creature.order = self.next_order;
        } else {
            self.next_order = self.next_order.max(creature.order);
        }

        let id = creature.id;
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
        removed.sort_by_key(|creature| creature.order);
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
        self.sorted.sort_by_key(|creature| {
            (
                creature.initiative.is_none(),
                Reverse(creature.initiative.unwrap_or(i32::MIN)),
                creature.order,
            )
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
    use super::{Creature, Creatures};

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
    fn same_initiative_creatures_keep_insertion_order() {
        let mut creatures = Creatures::default();
        creatures.add(Creature::new("first", Some(10), None, 7));
        creatures.add(Creature::new("second", Some(10), None, 7));
        creatures.add(Creature::new("third", Some(10), None, 7));

        let names: Vec<&str> = creatures
            .iter()
            .map(|creature| creature.name.as_str())
            .collect();
        assert_eq!(names, vec!["first", "second", "third"]);
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
