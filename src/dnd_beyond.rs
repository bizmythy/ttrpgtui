use serde::Deserialize;

use crate::models::creature::Creature;

const CHARACTER_URL_PREFIX: &str = "https://character-service.dndbeyond.com/character/v5/character";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterId(pub u64);

pub fn fetch_character(id: CharacterId) -> color_eyre::Result<Creature> {
    let url = format!("{CHARACTER_URL_PREFIX}/{}", id.0);
    let response: CharacterResponse = reqwest::blocking::get(url)?.error_for_status()?.json()?;

    if !response.success {
        color_eyre::eyre::bail!(
            "D&D Beyond returned an unsuccessful response for character {}",
            id.0
        );
    }

    Ok(response.data.into_creature())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterResponse {
    success: bool,
    data: CharacterData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterData {
    name: String,
    base_hit_points: i32,
    bonus_hit_points: Option<i32>,
    override_hit_points: Option<i32>,
    #[serde(default)]
    armor_class: Option<i32>,
    #[serde(default)]
    race: Option<Race>,
    #[serde(default)]
    classes: Vec<Class>,
}

impl CharacterData {
    fn into_creature(self) -> Creature {
        let max_health = self
            .override_hit_points
            .unwrap_or_else(|| self.base_hit_points + self.bonus_hit_points.unwrap_or_default());
        let description = self.description();
        let mut creature = Creature::new(self.name, None, self.armor_class, max_health);
        creature.set_description(description);
        creature
    }

    fn description(&self) -> String {
        let ancestry = self.race.as_ref().map(|race| race.full_name.as_str());
        let classes = self
            .classes
            .iter()
            .map(Class::description)
            .collect::<Vec<_>>();

        match (ancestry, classes.is_empty()) {
            (Some(ancestry), false) => format!("{} {}", ancestry, classes.join(" / ")),
            (Some(ancestry), true) => ancestry.to_string(),
            (None, false) => classes.join(" / "),
            (None, true) => "D&D Beyond character".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Race {
    full_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Class {
    level: i32,
    definition: ClassDefinition,
}

impl Class {
    fn description(&self) -> String {
        format!("{} {}", self.definition.name, self.level)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClassDefinition {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dnd_beyond_response_uses_reported_armor_class_only() {
        let response: CharacterResponse = serde_json::from_value(serde_json::json!({
            "success": true,
            "data": {
                "name": "Example Hero",
                "baseHitPoints": 42,
                "bonusHitPoints": 3,
                "overrideHitPoints": null,
                "race": { "fullName": "Human" },
                "classes": [{ "level": 6, "definition": { "name": "Fighter" } }]
            }
        }))
        .unwrap();

        let creature = response.data.into_creature();
        assert_eq!(creature.name, "Example Hero");
        assert_eq!(creature.get_max_health(), 45);
        assert_eq!(creature.ac, None);
        assert_eq!(creature.description, "Human Fighter 6");
    }

    #[test]
    fn dnd_beyond_response_preserves_reported_armor_class_when_present() {
        let response: CharacterResponse = serde_json::from_value(serde_json::json!({
            "success": true,
            "data": {
                "name": "Example Hero",
                "baseHitPoints": 42,
                "bonusHitPoints": null,
                "overrideHitPoints": 50,
                "armorClass": 17
            }
        }))
        .unwrap();

        let creature = response.data.into_creature();
        assert_eq!(creature.get_max_health(), 50);
        assert_eq!(creature.ac, Some(17));
    }
}
