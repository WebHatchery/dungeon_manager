use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: String,
    pub build: BuildData,
    pub requirements: RequirementsData,
    pub effects: EffectsData,
    pub scaling: ScalingData,
    pub ai: AIData,
    pub visual: RoomVisualData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildData {
    pub cost_per_tile: i32,
    pub mana_cost: i32,
    pub min_tiles: u32,
    pub max_tiles: u32,
    pub dig_required: bool,
    pub requires_claimed: bool,
    pub can_overlap: bool,
    pub allowed_terrain: Vec<String>,
    pub construction_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementsData {
    // `research` used to live here: a parallel list of tech ids that nothing
    // enforced and the sidebar displayed, so it drifted from the tech tree in
    // 7 of 18 rooms. The unlock lives in `technologies.json`'s `unlocks.rooms`
    // and is read via `technologies::tech_unlocking_room`.
    pub global_rooms_required: Vec<String>,
    pub max_instances: u32,
    pub forbidden_if: Vec<String>,
}

/// Rate multipliers default to 1.0 so an unset room runs at the global rate
/// rather than producing nothing.
fn default_rate_multiplier() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectsData {
    #[serde(default)]
    pub mana_generation_per_second: f32,
    #[serde(default)]
    pub happiness_modifier: i32,
    #[serde(default)]
    pub sleep_recovery_rate: f32,
    #[serde(default)]
    pub food_generation_per_second: f32,
    #[serde(default)]
    pub gold_storage_capacity: i32,
    /// Multiplier on `config.task_execution.xp_per_training` for a room in the
    /// `train` task family. Replaces a dead `xp_per_minute` that no file set
    /// and nothing read; mirrors [`EffectsData::research_rate`].
    #[serde(default = "default_rate_multiplier")]
    pub training_rate: f32,
    /// Multiplier on `config.task_execution.research_production_rate` for a
    /// room in the `research` task family. Named to match the key `rooms.json`
    /// has always authored — the struct previously declared
    /// `research_per_minute`, which no file set, so the library's `1.0` was
    /// dropped by serde and every research room ran at the global rate.
    #[serde(default = "default_rate_multiplier")]
    pub research_rate: f32,
    #[serde(default)]
    pub torture_power: f32,
    #[serde(default)]
    pub creature_defense_modifier: f32,
    #[serde(default)]
    pub mana_storage_capacity: i32,
    /// Rate at which heroes are converted to creatures in prison (progress per second)
    #[serde(default)]
    pub hero_conversion_rate: f32,
    #[serde(default)]
    pub corpse_storage: i32,
    /// Mana yielded per body rendered down, scaled by room efficiency. A room
    /// declaring this is a Soul Furnace as far as the engine is concerned.
    #[serde(default)]
    pub mana_per_corpse: f32,
    #[serde(default)]
    pub spawns_vampires: bool,
    #[serde(default)]
    pub grouping_point: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingData {
    pub per_tile_multiplier: f32,
    pub size_thresholds: Vec<SizeThreshold>,
    pub shape_penalties: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeThreshold {
    pub tiles: u32,
    pub multiplier: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIData {
    pub task_type: String,
    pub desirability: f32,
    pub max_creatures: u32,
    pub preferred_creatures: Vec<String>,
    pub forbidden_creatures: Vec<String>,
    #[serde(default)]
    pub work_size: Option<[u32; 2]>,
    pub entry_conditions: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomVisualData {
    pub floor_sprite: String,
    pub wall_sprite: String,
    pub object_spawn: Vec<ObjectSpawn>,
    pub light: LightEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectSpawn {
    pub object: String,
    pub density: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightEffect {
    pub color: [u8; 3],
    pub intensity: f32,
    pub flicker: bool,
}

/// The `rooms.json` id a runtime `room_type` string refers to.
///
/// The two agree for every room but the training hall, which the build UI, the
/// task system and the tile grid all know as `training_room` while the data
/// calls it `training_hall`. Three call sites used to re-derive this inline;
/// keep it here so the next place that needs it finds one answer.
pub fn room_data_id(room_type: &str) -> &str {
    match room_type {
        "training_room" => "training_hall",
        other => other,
    }
}

/// The runtime `room_type` / `tile_type` a `rooms.json` id is placed on the
/// grid as. The inverse of [`room_data_id`].
pub fn room_tile_type(data_id: &str) -> &str {
    match data_id {
        "training_hall" => "training_room",
        other => other,
    }
}

pub fn load_rooms() -> Result<HashMap<String, RoomData>, Box<dyn Error>> {
    let rooms_vec: Vec<RoomData> =
        macroquad_toolkit::include_json!("../../assets/data/rooms.json")?;

    let mut rooms_map = HashMap::new();
    for room in rooms_vec {
        rooms_map.insert(room.id.clone(), room);
    }

    // Load special rooms (Dungeon Heart etc)
    let special_rooms_vec: Vec<RoomData> =
        macroquad_toolkit::include_json!("../../assets/data/special_rooms.json")?;

    for room in special_rooms_vec {
        rooms_map.insert(room.id.clone(), room);
    }

    Ok(rooms_map)
}
