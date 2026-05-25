use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use chrono::Local;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    action::Action,
    dnd_beyond::{self, CharacterId},
    models::creature::{Creature, Creatures},
};

const SESSIONS_DIR: &str = "sessions";
const SESSION_MANIFEST: &str = "session.ron";
const CAMPAIGN_CONFIG: &str = "campaign.ron";
const ENCOUNTER_EXTENSION: &str = "ron";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionManifest {
    pub date: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionInfo {
    pub dir_name: String,
    pub path: PathBuf,
    pub date: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterInfo {
    pub file_name: String,
    pub path: PathBuf,
    pub name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CampaignConfig {
    #[serde(default)]
    pub creatures: Vec<CampaignCreature>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CampaignCreature {
    Preset(CampaignCreaturePreset),
    DndBeyond(DndBeyondCharacterPreset),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CampaignCreaturePreset {
    pub name: String,
    pub health: i32,
    #[serde(default)]
    pub ac: Option<i32>,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DndBeyondCharacterPreset {
    pub dnd_beyond_character_id: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedEncounter {
    pub name: String,
    #[serde(default)]
    pub creatures: Vec<Creature>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveRequest {
    pub session_dir: String,
    pub encounter_file: String,
    pub encounter: PersistedEncounter,
}

pub fn spawn_writer(
    data_dir: PathBuf,
    mut rx: mpsc::UnboundedReceiver<SaveRequest>,
    action_tx: mpsc::UnboundedSender<Action>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            let data_dir = data_dir.clone();
            let description = format!("{}/{}", request.session_dir, request.encounter_file);
            let result =
                tokio::task::spawn_blocking(move || write_save_request(&data_dir, &request))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result.map_err(|error| error.to_string()));

            if let Err(error) = result {
                let _ = action_tx.send(Action::Error(format!(
                    "Failed to save encounter {description}: {error}"
                )));
            }
        }
    })
}

pub fn sessions_root(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSIONS_DIR)
}

pub fn campaign_config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CAMPAIGN_CONFIG)
}

pub fn ensure_campaign_config(data_dir: &Path) -> color_eyre::Result<()> {
    let path = campaign_config_path(data_dir);
    if !path.exists() {
        write_ron_atomic(&path, &CampaignConfig::default())?;
    }
    Ok(())
}

pub fn load_campaign_config(data_dir: &Path) -> color_eyre::Result<CampaignConfig> {
    ensure_campaign_config(data_dir)?;
    read_ron(&campaign_config_path(data_dir))
}

pub fn list_sessions(data_dir: &Path) -> color_eyre::Result<Vec<SessionInfo>> {
    ensure_campaign_config(data_dir)?;
    let root = sessions_root(data_dir);
    fs::create_dir_all(&root)?;

    let mut sessions = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let manifest_path = path.join(SESSION_MANIFEST);
        let manifest = if manifest_path.exists() {
            read_ron::<SessionManifest>(&manifest_path)
                .unwrap_or_else(|_| manifest_from_dir(&dir_name))
        } else {
            manifest_from_dir(&dir_name)
        };

        sessions.push(SessionInfo {
            dir_name,
            path,
            date: manifest.date,
            name: manifest.name,
        });
    }

    sessions.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.dir_name.cmp(&right.dir_name))
    });
    Ok(sessions)
}

pub fn create_session(data_dir: &Path, name: &str) -> color_eyre::Result<SessionInfo> {
    let name = validate_name(name, "session name")?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let base_dir_name = format!("{}_{}", date, slugify(name));
    let root = sessions_root(data_dir);
    fs::create_dir_all(&root)?;
    let dir_name = unique_dir_name(&root, &base_dir_name);
    let path = root.join(&dir_name);
    fs::create_dir_all(&path)?;

    let manifest = SessionManifest {
        date: date.clone(),
        name: name.to_string(),
    };
    write_ron_atomic(&path.join(SESSION_MANIFEST), &manifest)?;

    Ok(SessionInfo {
        dir_name,
        path,
        date,
        name: name.to_string(),
    })
}

pub fn list_encounters(session: &SessionInfo) -> color_eyre::Result<Vec<EncounterInfo>> {
    let mut encounters = Vec::new();
    fs::create_dir_all(&session.path)?;

    for entry in fs::read_dir(&session.path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file()
            || path
                .file_name()
                .is_some_and(|name| name == OsStr::new(SESSION_MANIFEST))
        {
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some(ENCOUNTER_EXTENSION) {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let fallback_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(deslugify)
            .unwrap_or_else(|| file_name.clone());
        let name = read_ron::<PersistedEncounter>(&path)
            .map(|encounter| encounter.name)
            .unwrap_or(fallback_name);

        encounters.push(EncounterInfo {
            file_name,
            path,
            name,
        });
    }

    encounters.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    Ok(encounters)
}

pub fn create_encounter(
    data_dir: &Path,
    session: &SessionInfo,
    name: &str,
) -> color_eyre::Result<EncounterInfo> {
    let name = validate_name(name, "encounter name")?;
    fs::create_dir_all(&session.path)?;
    let base_file_stem = slugify(name);
    let file_name = unique_file_name(&session.path, &base_file_stem, ENCOUNTER_EXTENSION);
    let path = session.path.join(&file_name);
    let encounter = PersistedEncounter {
        name: name.to_string(),
        creatures: campaign_creatures(data_dir)?,
    };
    write_ron_atomic(&path, &encounter)?;

    Ok(EncounterInfo {
        file_name,
        path,
        name: name.to_string(),
    })
}

pub fn load_encounter(encounter: &EncounterInfo) -> color_eyre::Result<PersistedEncounter> {
    read_ron(&encounter.path)
}

pub fn write_save_request(data_dir: &Path, request: &SaveRequest) -> color_eyre::Result<()> {
    let path = sessions_root(data_dir)
        .join(&request.session_dir)
        .join(&request.encounter_file);
    write_ron_atomic(&path, &request.encounter)
}

pub fn encounter_from_creatures(name: String, creatures: &Creatures) -> PersistedEncounter {
    PersistedEncounter {
        name,
        creatures: creatures.to_vec(),
    }
}

fn campaign_creatures(data_dir: &Path) -> color_eyre::Result<Vec<Creature>> {
    load_campaign_config(data_dir)?
        .creatures
        .into_iter()
        .map(campaign_creature)
        .collect()
}

fn campaign_creature(preset: CampaignCreature) -> color_eyre::Result<Creature> {
    match preset {
        CampaignCreature::Preset(preset) => Ok(Creature::from(preset)),
        CampaignCreature::DndBeyond(preset) => {
            dnd_beyond::fetch_character(CharacterId(preset.dnd_beyond_character_id))
        }
    }
}

impl From<CampaignCreaturePreset> for Creature {
    fn from(preset: CampaignCreaturePreset) -> Self {
        let mut creature = Creature::new(preset.name, None, preset.ac, preset.health);
        creature.set_description(preset.description);
        creature
    }
}

fn read_ron<T>(path: &Path) -> color_eyre::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let contents = fs::read_to_string(path)?;
    Ok(ron::from_str(&contents)?)
}

fn write_ron_atomic<T>(path: &Path, value: &T) -> color_eyre::Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let pretty = PrettyConfig::new();
    let contents = ron::ser::to_string_pretty(value, pretty)?;
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("ron")
    ));
    fs::write(&tmp_path, contents)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn manifest_from_dir(dir_name: &str) -> SessionManifest {
    let (date, name) = dir_name
        .split_once('_')
        .map(|(date, name)| (date.to_string(), deslugify(name)))
        .unwrap_or_else(|| ("unknown-date".to_string(), deslugify(dir_name)));
    SessionManifest { date, name }
}

fn validate_name<'a>(name: &'a str, label: &str) -> color_eyre::Result<&'a str> {
    let name = name.trim();
    if name.is_empty() {
        color_eyre::eyre::bail!("{label} is required");
    }
    Ok(name)
}

fn unique_dir_name(root: &Path, base: &str) -> String {
    unique_name(base, |candidate| root.join(candidate).exists())
}

fn unique_file_name(root: &Path, base_stem: &str, extension: &str) -> String {
    let base = format!("{base_stem}.{extension}");
    unique_name(&base, |candidate| root.join(candidate).exists())
}

fn unique_name(base: &str, exists: impl Fn(&str) -> bool) -> String {
    if !exists(base) {
        return base.to_string();
    }

    let (stem, extension) = base
        .rsplit_once('.')
        .map(|(stem, extension)| (stem.to_string(), Some(extension.to_string())))
        .unwrap_or_else(|| (base.to_string(), None));

    for suffix in 2.. {
        let candidate = if let Some(extension) = &extension {
            format!("{stem}-{suffix}.{extension}")
        } else {
            format!("{stem}-{suffix}")
        };
        if !exists(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in input.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('-');
            previous_was_separator = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug.to_string()
    }
}

fn deslugify(input: &str) -> String {
    input
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::creature::Creature;

    #[test]
    fn session_and_encounter_round_trip_through_ron_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let session = create_session(temp_dir.path(), "Friday Night").unwrap();
        let encounter = create_encounter(temp_dir.path(), &session, "Goblin Ambush").unwrap();

        assert!(campaign_config_path(temp_dir.path()).exists());

        let sessions = list_sessions(temp_dir.path()).unwrap();
        assert_eq!(sessions[0].name, "Friday Night");

        let encounters = list_encounters(&sessions[0]).unwrap();
        assert_eq!(encounters[0].name, "Goblin Ambush");

        let mut creatures = Creatures::default();
        creatures.add(Creature::new("goblin", Some(12), Some(15), 7));
        write_save_request(
            temp_dir.path(),
            &SaveRequest {
                session_dir: session.dir_name,
                encounter_file: encounter.file_name,
                encounter: encounter_from_creatures("Goblin Ambush".to_string(), &creatures),
            },
        )
        .unwrap();

        let reloaded = load_encounter(&encounters[0]).unwrap();
        assert_eq!(reloaded.name, "Goblin Ambush");
        assert_eq!(reloaded.creatures[0].name, "goblin");
    }

    #[test]
    fn new_encounters_are_seeded_from_campaign_config_without_initiative() {
        let temp_dir = tempfile::tempdir().unwrap();
        write_ron_atomic(
            &campaign_config_path(temp_dir.path()),
            &CampaignConfig {
                creatures: vec![CampaignCreature::Preset(CampaignCreaturePreset {
                    name: "Mira".to_string(),
                    health: 24,
                    ac: Some(16),
                    description: "party cleric".to_string(),
                })],
            },
        )
        .unwrap();

        let session = create_session(temp_dir.path(), "Friday Night").unwrap();
        let encounter = create_encounter(temp_dir.path(), &session, "Goblin Ambush").unwrap();
        let persisted = load_encounter(&encounter).unwrap();

        assert_eq!(persisted.creatures.len(), 1);
        let creature = &persisted.creatures[0];
        assert_eq!(creature.name, "Mira");
        assert_eq!(creature.get_health(), 24);
        assert_eq!(creature.get_max_health(), 24);
        assert_eq!(creature.ac, Some(16));
        assert_eq!(creature.initiative, None);
        assert_eq!(creature.description, "party cleric");
    }
}
