use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct HeroBuildingData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub hp: i32,
    pub width: i32,
    pub height: i32,
    pub spawn_triggers: Vec<SpawnTrigger>,
    pub destruction_effect: DestructionEffect,
    pub visual: BuildingVisual,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpawnTrigger {
    pub hero_id: String,
    pub spawn_rate_seconds: f32,
    pub max_active: usize,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum DestructionEffect {
    #[serde(rename = "win_game")]
    WinGame,
    #[serde(rename = "reduce_spawn_rate")]
    ReduceSpawnRate { percent: i32 },
    #[serde(rename = "reduce_hero_speed")]
    ReduceHeroSpeed { percent: i32 },
    #[serde(rename = "reduce_hero_stats")]
    ReduceHeroStats { attack: i32, defense: i32 },
    #[serde(rename = "open_path")]
    OpenPath,
}

impl DestructionEffect {
    /// What razing this building buys the keeper, in the player's terms.
    ///
    /// The notification used to read "Armory destroyed." and stop there, so
    /// the effect landed invisibly and there was no way to learn that
    /// levelling the armoury had blunted every hero on the map.
    pub fn describe(&self) -> Option<String> {
        match self {
            Self::WinGame => Some("the heroes' seat of power has fallen".to_string()),
            Self::ReduceSpawnRate { percent } => {
                Some(format!("hero reinforcements slowed by {percent}%"))
            }
            Self::ReduceHeroSpeed { percent } => Some(format!("heroes slowed by {percent}%")),
            Self::ReduceHeroStats { attack, defense } => Some(format!(
                "every hero loses {attack} attack and {defense} defence"
            )),
            // The tile becoming open floor *is* the effect; there is nothing
            // to report that the player cannot already see.
            Self::OpenPath => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct BuildingVisual {
    pub tile: String,
}

pub fn load_hero_buildings() -> Result<HashMap<String, HeroBuildingData>, Box<dyn Error>> {
    let json_content = {
        #[cfg(target_arch = "wasm32")]
        {
            macroquad_toolkit::include_json_str!("../../assets/data/hero_buildings.json")
                .to_string()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::read_to_string("assets/data/hero_buildings.json").unwrap_or_else(|_| {
                macroquad_toolkit::include_json_str!("../../assets/data/hero_buildings.json")
                    .to_string()
            })
        }
    };

    // The JSON is an array of objects
    let buildings_list: Vec<HeroBuildingData> = serde_json::from_str(&json_content)?;

    let mut buildings_map = HashMap::new();
    for building in buildings_list {
        buildings_map.insert(building.id.clone(), building);
    }

    Ok(buildings_map)
}
