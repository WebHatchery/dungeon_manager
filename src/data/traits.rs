//! Monster trait definitions — data-driven behavioral/stat modifiers.
//!
//! Traits are tag strings on `MonsterData.traits` (e.g. "cowardly", "greedy"). Each tag is
//! looked up here for a set of generic modifiers that the engine applies uniformly (see
//! `engine::creature_ai::needs`, `engine::creature_task_logic`, `engine::combat`). Adding a new
//! trait — or changing what an existing one does — is a `traits.json` edit (or a content pack's
//! own trait file, see `data::content_pack`); the engine never branches on a trait's name.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraitData {
    pub id: String,
    #[serde(default)]
    pub description: String,
    /// Added directly to the creature's calculated mood (see `needs::calculate_mood`).
    #[serde(default)]
    pub mood_modifier: f32,
    /// Added to `ai.anger_threshold` before the `mood < threshold` comparison — a positive value
    /// means the creature angers at a higher mood, i.e. sooner.
    #[serde(default)]
    pub anger_threshold_modifier: f32,
    /// Same as `anger_threshold_modifier` but for `ai.desertion_threshold`.
    #[serde(default)]
    pub desertion_threshold_modifier: f32,
    /// Multiplies a matching need's `decay_per_minute` (need name -> multiplier).
    #[serde(default)]
    pub need_decay_multipliers: HashMap<String, f32>,
    /// Multiplies a matching task type's desirability weight (task type -> multiplier).
    #[serde(default)]
    pub task_preference_multipliers: HashMap<String, f32>,
    /// Multiplies combat attack stat.
    #[serde(default = "one")]
    pub attack_multiplier: f32,
    /// Multiplies combat defense stat.
    #[serde(default = "one")]
    pub defense_multiplier: f32,
    /// Multiplies the magnitude of discipline responses (slap/torture/reward mood swings).
    #[serde(default = "one")]
    pub discipline_response_multiplier: f32,
    /// Multiplies the damage of any trap sprung near a creature with this trait
    /// (see `trap_system::nearby_trap_tending_bonus`). Unlike the modifiers
    /// above, this one acts on the *world* rather than on the creature holding
    /// it, which is what lets a creature buff a structure.
    #[serde(default = "one")]
    pub trap_damage_multiplier: f32,
    /// How far that bonus reaches, in tiles. Only meaningful alongside
    /// `trap_damage_multiplier`.
    #[serde(default)]
    pub trap_tending_radius: f32,
    /// Seconds a creature with this trait takes to convert one adjacent plain
    /// wall into `reinforced_wall`, which heroes cannot tunnel through. Zero
    /// (the default) means the creature never does this. Like the trap fields
    /// above, this acts on the world rather than on its holder.
    #[serde(default)]
    pub wall_reinforce_seconds: f32,
    /// Multiplies the *work efficiency of other creatures* within
    /// `command_radius`. The Overseer's aura: it acts on neighbours rather
    /// than on itself or on the map.
    #[serde(default = "one")]
    pub command_efficiency_bonus: f32,
    /// How far that aura reaches, in tiles.
    #[serde(default)]
    pub command_radius: f32,
    /// Multiplies this creature's *own* work efficiency, which covers research
    /// output too since `execute_research` scales by the same term.
    #[serde(default = "one")]
    pub work_efficiency_multiplier: f32,
    /// How many random traits a creature carrying this one is grafted with,
    /// once, on first sight. Zero means it is not a grafting trait.
    #[serde(default)]
    pub graft_count: u32,
    /// Extra attack, as a fraction, at full darkness — scaled by how dark the
    /// creature's tile actually is. The Shadow Stalker's "stronger in
    /// darkness", and the first thing that makes the keeper's *lighting*
    /// choices a tactical decision rather than decoration.
    #[serde(default)]
    pub darkness_attack_bonus: f32,
    /// How much this creature raises the surface world's interest in the
    /// dungeon. Added into `GameState::effective_threat_multiplier`, which sets
    /// both the gap between hero waves and how fast the garrison replenishes —
    /// so a creature with this is a real cost as well as an asset.
    #[serde(default)]
    pub threat_contribution: f32,
    /// Whether this trait is eligible to be rolled onto a grafted creature.
    /// Opt-in: the world-altering traits (`stonebinding`, `commanding`, …)
    /// would be absurd on a random amalgam, so nothing is graftable unless
    /// `traits.json` says so.
    #[serde(default)]
    pub graftable: bool,
}

fn one() -> f32 {
    1.0
}

pub fn load_traits() -> Result<HashMap<String, TraitData>, Box<dyn Error>> {
    let json_content_result = macroquad_toolkit::data_loader::load_json_file_with_fallback_sync(
        "assets/data/traits.json",
        macroquad_toolkit::include_json_str!("../../assets/data/traits.json"),
        macroquad_toolkit::data_loader::JsonFallbackPolicy::ReadError,
    );

    let traits_vec: Vec<TraitData> = json_content_result?;

    let mut traits_map = HashMap::new();
    for trait_data in traits_vec {
        traits_map.insert(trait_data.id.clone(), trait_data);
    }

    Ok(traits_map)
}
