use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub cost: SpellCost,
    pub targeting: TargetingData,
    pub effects: Vec<SpellEffect>,
    pub cooldown: f32,
    pub visual: SpellVisualData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCost {
    #[serde(default)]
    pub mana: i32,
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub health: i32,
    // `souls` used to sit here: no spell in dungeon_spells.json ever authored
    // one, and the game has no soul resource for it to spend. Deleted rather
    // than left as an aspiration (CODE_STANDARDS.md on unused code).
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetingData {
    #[serde(rename = "type")]
    pub target_type: String,
    #[serde(default)]
    pub range: u32,
    #[serde(default)]
    pub area_radius: u32,
    #[serde(default)]
    pub requires_visibility: bool,
    #[serde(default)]
    pub valid_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellEffect {
    #[serde(rename = "type")]
    pub effect_type: String,
    #[serde(default)]
    pub amount: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_tile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_tile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellVisualData {
    pub icon: String,
}

pub fn load_spells() -> Result<HashMap<String, SpellData>, Box<dyn Error>> {
    let spells_vec: Vec<SpellData> =
        macroquad_toolkit::include_json!("../../assets/data/dungeon_spells.json")?;

    let mut spells_map = HashMap::new();
    for spell in spells_vec {
        spells_map.insert(spell.id.clone(), spell);
    }

    Ok(spells_map)
}

#[cfg(test)]
mod tests;
