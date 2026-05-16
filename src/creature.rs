//! Creature data structures for the TTRPG TUI.

/// Individual Creature Data
#[derive(Debug, Default)]
pub struct Creature {
    pub name: String,
    pub initiative: u8,
    pub ac: u8,
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
        creatures.add(Creature {
            name: "slow".into(),
            initiative: 3,
            ac: 12,
        });
        creatures.add(Creature {
            name: "fast".into(),
            initiative: 18,
            ac: 15,
        });
        creatures.add(Creature {
            name: "middle".into(),
            initiative: 10,
            ac: 13,
        });

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
