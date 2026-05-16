//! Creature data structures for the TTRPG TUI.

/// Individual Creature Data
#[derive(Debug, Default)]
pub struct Creature {
    pub name: String,
    pub initiative: u8,
    pub ac: u8,
    health: i32,
    max_health: u16,
}

impl Creature {
    pub fn new(name: String, initiative: u8, ac: u8, max_health: u16) -> Self {
        Self {
            name,
            initiative,
            ac,
            health: (max_health as i32),
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
        self.max_health as i32
    }

    pub fn modify_health(&mut self, delta: i32) -> i32 {
        self.health += delta;

        let max = self.get_max_health();
        if self.health > max {
            self.health = max;
        }

        self.health
    }
}

#[derive(Debug, Default)]
pub struct Creatures {
    sorted: Vec<Creature>,
}

impl Creatures {
    pub fn add(&mut self, creature: Creature) {
        self.sorted.push(creature);
        self.sorted.sort_by_key(|c| std::cmp::Reverse(c.initiative));
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Creature> {
        self.sorted.iter()
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
        creatures.add(Creature::new("slow".into(), 3, 12, 7));
        creatures.add(Creature::new("fast".into(), 18, 15, 12));
        creatures.add(Creature::new("middle".into(), 10, 13, 9));

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
}
