use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui_textarea::TextArea;

use super::creature::CreatureId;

pub enum AppMode {
    Normal,
    HealthInput(Box<HealthInput>),
    InitiativeInput(Box<InitiativeInput>),
    RenameInput(Box<RenameInput>),
    NewCreature(Box<NewCreatureForm>),
}

pub struct HealthInput {
    pub operation: HealthOperation,
    pub target_ids: Vec<CreatureId>,
    pub textarea: TextArea<'static>,
    pub error: Option<String>,
}

pub struct InitiativeInput {
    pub target_ids: Vec<CreatureId>,
    pub textarea: TextArea<'static>,
    pub error: Option<String>,
}

pub struct RenameInput {
    pub target_id: CreatureId,
    pub textarea: TextArea<'static>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthOperation {
    Add,
    Subtract,
}

pub struct NewCreatureForm {
    pub fields: NewCreatureFields,
    pub active_field: NewCreatureField,
    pub error: Option<String>,
}

pub struct NewCreatureFields {
    pub name: TextArea<'static>,
    pub initiative: TextArea<'static>,
    pub ac: TextArea<'static>,
    pub health: TextArea<'static>,
    pub count: TextArea<'static>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewCreatureField {
    Name,
    Initiative,
    Ac,
    Health,
    Count,
}

pub struct ParsedNewCreature {
    pub name: String,
    pub initiative: Option<i32>,
    pub ac: Option<i32>,
    pub health: i32,
    pub count: usize,
}

impl HealthInput {
    pub fn new(operation: HealthOperation, target_ids: Vec<CreatureId>) -> Self {
        Self {
            operation,
            target_ids,
            textarea: single_line_textarea("amount", ""),
            error: None,
        }
    }
}

impl InitiativeInput {
    pub fn new(target_ids: Vec<CreatureId>) -> Self {
        Self {
            target_ids,
            textarea: single_line_textarea("initiative", "Initiative"),
            error: None,
        }
    }
}

impl RenameInput {
    pub fn new(target_id: CreatureId, current_name: &str) -> Self {
        let mut textarea = single_line_textarea("name", "Name");
        textarea.insert_str(current_name);
        Self {
            target_id,
            textarea,
            error: None,
        }
    }
}

impl NewCreatureForm {
    pub fn new() -> Self {
        Self {
            fields: NewCreatureFields {
                name: single_line_textarea("name", "Name"),
                initiative: single_line_textarea("initiative", "Initiative (optional)"),
                ac: single_line_textarea("AC", "AC (optional)"),
                health: single_line_textarea("health", "Health"),
                count: single_line_textarea("count", "Count (optional)"),
            },
            active_field: NewCreatureField::Name,
            error: None,
        }
    }

    pub fn active_textarea_mut(&mut self) -> &mut TextArea<'static> {
        match self.active_field {
            NewCreatureField::Name => &mut self.fields.name,
            NewCreatureField::Initiative => &mut self.fields.initiative,
            NewCreatureField::Ac => &mut self.fields.ac,
            NewCreatureField::Health => &mut self.fields.health,
            NewCreatureField::Count => &mut self.fields.count,
        }
    }

    pub fn parse(&self) -> Result<ParsedNewCreature, String> {
        let name = textarea_value(&self.fields.name).trim().to_string();
        if name.is_empty() {
            return Err("name is required".to_string());
        }

        Ok(ParsedNewCreature {
            name,
            initiative: parse_optional_i32(textarea_value(&self.fields.initiative), "initiative")?,
            ac: parse_optional_positive_i32(textarea_value(&self.fields.ac), "AC")?,
            health: parse_positive_i32(textarea_value(&self.fields.health), "health")?,
            count: parse_optional_positive_usize(textarea_value(&self.fields.count), "count")?
                .unwrap_or(1),
        })
    }
}

impl Default for NewCreatureForm {
    fn default() -> Self {
        Self::new()
    }
}

impl NewCreatureField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Initiative,
            Self::Initiative => Self::Ac,
            Self::Ac => Self::Health,
            Self::Health => Self::Count,
            Self::Count => Self::Name,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Name => Self::Count,
            Self::Initiative => Self::Name,
            Self::Ac => Self::Initiative,
            Self::Health => Self::Ac,
            Self::Count => Self::Health,
        }
    }
}

pub fn single_line_textarea(title: &'static str, placeholder: &'static str) -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue))
            .title(title),
    );
    textarea.set_placeholder_text(placeholder);
    textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));
    textarea
}

pub fn textarea_value(textarea: &TextArea<'_>) -> String {
    textarea.lines().join("\n")
}

pub fn parse_i32(value: String, label: &str) -> Result<i32, String> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("{label} must be a number"))
}

pub fn parse_positive_i32(value: String, label: &str) -> Result<i32, String> {
    let parsed = parse_i32(value, label)?;
    if parsed <= 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(parsed)
}

pub fn letter_suffix(mut index: usize) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    chars.iter().rev().collect()
}

fn parse_optional_i32(value: String, label: &str) -> Result<Option<i32>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i32>()
        .map(Some)
        .map_err(|_| format!("{label} must be a number"))
}

fn parse_optional_positive_i32(value: String, label: &str) -> Result<Option<i32>, String> {
    let Some(value) = parse_optional_i32(value, label)? else {
        return Ok(None);
    };
    if value <= 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(Some(value))
}

fn parse_optional_positive_usize(value: String, label: &str) -> Result<Option<usize>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{label} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests {
    use super::letter_suffix;

    #[test]
    fn letter_suffixes_continue_after_z() {
        assert_eq!(letter_suffix(0), "A");
        assert_eq!(letter_suffix(25), "Z");
        assert_eq!(letter_suffix(26), "AA");
        assert_eq!(letter_suffix(27), "AB");
    }
}
