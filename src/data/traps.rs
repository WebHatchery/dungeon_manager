//! Trap data definitions loaded from JSON

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrapData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub cost: i32,
    pub build_time: f32,
    pub effects: TrapEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrapEffects {
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub blocks_movement: bool,
    #[serde(default)]
    pub lockable: bool,
    #[serde(default)]
    pub trigger_type: String,
    #[serde(default)]
    pub area: bool,
    #[serde(default)]
    pub alert_radius: f32,
    #[serde(default)]
    pub cooldown_multiplier: Option<f32>,
    #[serde(default)]
    pub cooldown: Option<f32>,
    #[serde(default)]
    pub area_radius: Option<f32>,
    #[serde(default)]
    pub single_radius: Option<f32>,
}

pub fn load_traps() -> Result<HashMap<String, TrapData>, Box<dyn Error>> {
    let json_content = macroquad_toolkit::include_json_str!("../../assets/data/traps.json");
    let traps_vec: Vec<TrapData> = serde_json::from_str(json_content)?;

    let mut traps_map = HashMap::new();
    for trap in traps_vec {
        traps_map.insert(trap.id.clone(), trap);
    }

    Ok(traps_map)
}
