use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileData {
    pub id: String,
    pub name: String,
    pub category: String,
    pub diggable: bool,
    pub claimable: bool,
    pub blocks_movement: bool,
    pub blocks_vision: bool,
    pub supports_rooms: bool,
    pub resources: Option<ResourceData>,
    pub durability: Option<u32>,
    pub damage_per_second: Option<f32>,
    pub speed_modifier: Option<f32>,
    pub cost: Option<i32>,
    pub visual: VisualData,
    pub special: Option<SpecialData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceData {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub amount: i32,
    #[serde(default)]
    pub mine_value: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualData {
    pub sprite: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fogged_sprite: Option<String>,
    #[serde(default)]
    pub animated: bool,
    pub light: Option<LightData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightData {
    pub color: [u8; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialData {
    pub aura: Option<AuraData>,
    // No `cannot_be_modified`: the only tile that declared it
    // (`ancient_rune_floor`) already authors `diggable: false` and
    // `claimable: false`, which the engine does read. A second flag saying the
    // same thing is a drift hazard, not a feature.
    pub triggers_event: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraData {
    pub radius: u32,
    pub effects: HashMap<String, f32>,
}

pub fn load_tiles() -> Result<HashMap<String, TileData>, Box<dyn Error>> {
    let json_content = macroquad_toolkit::include_json_str!("../../assets/data/tiles.json");
    let tiles_vec: Vec<TileData> = serde_json::from_str(json_content)?;

    let mut tiles_map = HashMap::new();
    for tile in tiles_vec {
        tiles_map.insert(tile.id.clone(), tile);
    }

    Ok(tiles_map)
}
