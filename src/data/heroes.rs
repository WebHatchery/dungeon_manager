use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tier: u32,
    pub role: String,
    pub stats: HeroStatsData,
    pub ai: HeroAIData,
    pub combat: HeroCombatData,
    pub abilities: Vec<HeroAbilityData>,
    pub behavior: BehaviorData,
    pub progression: HeroProgressionData,
    pub visual: HeroVisualData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroStatsData {
    pub health: f32,
    pub mana: f32,
    pub attack: f32,
    pub defense: f32,
    pub speed: f32,
    pub sight_radius: u32,
    pub bravery: f32,
    pub dig_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroAIData {
    pub primary_goal: String,
    pub secondary_goals: Vec<String>,
    pub room_priorities: HashMap<String, f32>,
    pub threat_response: ThreatResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatResponse {
    pub retreat_below_health: f32,
    pub call_for_aid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroCombatData {
    pub attack_type: String,
    pub damage_range: [f32; 2],
    pub attack_speed: f32,
    pub armor_type: String,
    pub resistances: HashMap<String, f32>,
}

/// A hero ability: a `trigger` (a small, fixed vocabulary the engine evaluates generically —
/// see `engine::hero_abilities`) gates a list of `effects` using the exact same data-driven
/// effect schema spells use (`data::spells::SpellEffect` — heal/damage/status_apply/etc.,
/// executed by the same `apply_spell_effect` dispatcher). Adding a new ability, or changing what
/// an existing one does, is a `heroes.json` edit — the engine never branches on an ability's id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroAbilityData {
    pub id: String,
    pub cooldown: f32,
    pub trigger: String,
    #[serde(default)]
    pub effects: Vec<crate::data::spells::SpellEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorData {
    pub trap_awareness: f32,
    pub door_break_chance: f32,
    pub light_preference: String,
    pub fear_resistance: f32,
    pub will_fight_to_death: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroProgressionData {
    pub level_range: [u32; 2],
    pub stat_growth_per_level: HashMap<String, f32>,
    pub elite_variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroVisualData {
    // No `sprite` field: the texture loader derives `sprites/heroes/{id}.png`
    // from the roster key, so an authored path was never read. All twenty
    // entries had drifted to `heroes/{id}.png` — a directory that does not
    // exist — without anything noticing. Same for `MonsterVisualData`.
    pub scale: f32,
    pub animations: Vec<String>,
    pub voice_set: String,
}

pub fn load_heroes() -> Result<HashMap<String, HeroData>, Box<dyn Error>> {
    let heroes_vec: Vec<HeroData> =
        macroquad_toolkit::include_json!("../../assets/data/heroes.json")?;

    let mut heroes_map = HashMap::new();
    for hero in heroes_vec {
        heroes_map.insert(hero.id.clone(), hero);
    }

    Ok(heroes_map)
}

#[cfg(test)]
mod tests;
