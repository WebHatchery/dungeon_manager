use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: f32, // Research points needed
    pub prerequisites: Vec<String>,
    pub unlocks: UnlockData,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockData {
    #[serde(default)]
    pub rooms: Vec<String>,
    #[serde(default)]
    pub spells: Vec<String>,
    #[serde(default)]
    pub creatures: Vec<String>,
    #[serde(default)]
    pub traps: Vec<String>,
}

/// The technology that unlocks `room_id`, if any.
///
/// `unlocks.rooms` is what `PlayerState::complete_research` actually acts on,
/// so it is the only honest answer to "what does this room need?". Rooms used
/// to carry a parallel `requirements.research` list that nothing enforced and
/// the sidebar displayed: six rooms showed no requirement while being
/// tech-locked, and the scavenger room named `ritual_tech` when `logistics` was
/// the real gate. Deriving the answer from the tech tree means the tooltip
/// cannot drift away from the rule.
pub fn tech_unlocking_room<'a>(
    room_id: &str,
    technologies: &'a HashMap<String, TechData>,
) -> Option<&'a TechData> {
    technologies
        .values()
        .find(|tech| tech.unlocks.rooms.iter().any(|room| room == room_id))
}

pub fn load_technologies() -> Result<HashMap<String, TechData>, Box<dyn Error>> {
    let json_content_result = macroquad_toolkit::data_loader::load_json_file_with_fallback_sync(
        "assets/data/technologies.json",
        macroquad_toolkit::include_json_str!("../../assets/data/technologies.json"),
        macroquad_toolkit::data_loader::JsonFallbackPolicy::ReadError,
    );

    let techs_vec: Vec<TechData> = json_content_result?;

    let mut techs_map = HashMap::new();
    for tech in techs_vec {
        techs_map.insert(tech.id.clone(), tech);
    }

    Ok(techs_map)
}

#[cfg(test)]
mod tests;
