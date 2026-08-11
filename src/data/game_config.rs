//! Game configuration loaded from JSON

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameConfig {
    pub player_starting_resources: PlayerStartingResources,
    pub player_initial_capacity: PlayerInitialCapacity,
    pub map_size: MapSizeConfig,
    pub timing: TimingConfig,
    pub hero_waves: HeroWaveConfig,
    pub creature_ai: CreatureAIConfig,
    pub fog_of_war: FogOfWarConfig,
    pub economy: EconomyConfig,
    pub combat: CombatConfig,
    pub combat_ranges: CombatRangesConfig,
    pub task_execution: TaskExecutionConfig,
    pub imp_behavior: ImpBehaviorConfig,
    pub spawning: SpawningConfig,
    pub traps: TrapConfig,
    pub dungeon: DungeonConfig,
    pub conversion: ConversionConfig,
    pub resource_generation: ResourceGenerationConfig,
    pub status_effects: StatusEffectsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStartingResources {
    pub gold: i32,
    pub mana: i32,
    pub food: i32,
    pub materials: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerInitialCapacity {
    pub max_gold: i32,
    pub max_mana: i32,
    pub max_food: i32,
    pub max_materials: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapSizeConfig {
    pub width: usize,
    pub height: usize,
}

impl Default for MapSizeConfig {
    fn default() -> Self {
        Self {
            width: 50,
            height: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimingConfig {
    pub pay_day_interval: f32,
    pub initial_creature_spawn_delay: f32,
    pub creature_spawn_min_interval: f32,
    pub creature_spawn_max_interval: f32,
    /// Seconds of unpaused play between autosaves.
    pub autosave_interval: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeroWaveConfig {
    pub initial_delay: f32,
    pub wave_interval: f32,
    pub defender_ratio: f32,
    pub wave_scaling_multiplier: f32,
    pub spawn_rate_decay: f32,
    pub min_spawn_rate: f32,
    pub spawn_search_radius: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatureAIConfig {
    pub need_critical_threshold: f32,
    pub need_desert_threshold: f32,
    pub mood_attention_threshold: f32,
    pub need_attention_threshold: f32,
    pub gold_carrying_threshold: i32,
    pub training_mood_threshold: f32,
    pub research_desirability: f32,
    pub wander_radius: i32,
    pub wander_attempts: i32,
    pub marker_distance_threshold: i32,
    pub slap_cooldown: f32,
    pub base_mood_efficiency: f32,
    pub mood_penalties: MoodPenaltiesConfig,
    pub task_desirability: TaskDesirabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoodPenaltiesConfig {
    pub low_health_threshold: f32,
    pub low_health_penalty: f32,
    pub angry_penalty: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDesirabilityConfig {
    pub base: f32,
    pub gold_deposit: f32,
    pub skip_deposit: f32,
    pub training_high_satisfaction: f32,
    pub training_low_satisfaction: f32,
    pub wage_collection: f32,
    pub satisfaction_threshold: f32,
    pub need_modifier: f32,
}

impl Default for TaskDesirabilityConfig {
    fn default() -> Self {
        Self {
            base: 1.0,
            gold_deposit: 1.5,
            skip_deposit: 0.1,
            training_high_satisfaction: 1.5,
            training_low_satisfaction: 0.5,
            wage_collection: 2.0,
            satisfaction_threshold: 60.0,
            need_modifier: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FogOfWarConfig {
    pub enabled: bool,
    pub sight_radius: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EconomyConfig {
    pub room_sell_refund_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatConfig {
    pub creature_level_multiplier: f32,
    pub creature_health_per_level: f32,
    pub hero_level_multiplier: f32,
    pub hero_health_per_level: f32,
    pub building_base_defense: f32,
    pub building_attack_speed: f32,
    pub attack_stat_bonus: f32,
    pub defense_reduction: f32,
    pub counterattack_death_chance: f32,
    pub counterattack_level_threshold: i32,
    pub building_xp_reward: i32,
    pub xp_per_victim_level: i32,
    pub max_creature_level: u32,
    pub level_up_health_bonus: f32,
    pub xp_requirement_base: u32,
    pub xp_requirement_multiplier: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskExecutionConfig {
    pub sleep_satisfaction_rate: f32,
    pub food_consumption_rate: f32,
    pub food_satisfaction_multiplier: f32,
    pub gold_deposit_satisfaction_rate: f32,
    pub training_timer_threshold: f32,
    pub xp_per_training: f32,
    pub level_up_exp_multiplier: f32,
    pub level_up_health_multiplier: f32,
    pub work_timer_threshold: f32,
    pub wage_satisfaction_rate: f32,
    pub research_production_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImpBehaviorConfig {
    pub dig_completion_delay: f32,
    pub gem_priority_bonus: f32,
    pub gold_vein_reward: i32,
    pub gem_seam_reward: i32,
    pub mana_crystal_reward: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpawningConfig {
    pub monster_spawner_interval: f32,
    pub max_monsters_per_spawner: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrapConfig {
    pub default_cooldown: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatRangesConfig {
    pub melee: i32,
    pub ranged: i32,
    pub magic: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DungeonConfig {
    pub heart_max_health: f32,
    pub initial_imp_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversionConfig {
    pub skeleton_rate: f32,
    pub torture_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGenerationConfig {
    pub base_gold_veins: usize,
    pub base_gem_seams: usize,
    pub base_mana_veins: usize,
    pub scattered_gold_density: f32,
    pub scattered_mana_density: f32,
    pub base_map_area: usize,
}

impl Default for ResourceGenerationConfig {
    fn default() -> Self {
        Self {
            base_gold_veins: 2,
            base_gem_seams: 4,
            base_mana_veins: 2,
            scattered_gold_density: 360.0,
            scattered_mana_density: 90.0,
            base_map_area: 2500,
        }
    }
}

/// Maps a monster combat ability id (`combat.abilities` in monsters.json) to the status
/// effect it procs on a landed hit.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatusEffectsConfig {
    pub ability_effects: HashMap<String, AbilityEffectData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityEffectData {
    /// One of "poison", "burn", "freeze", "stun" (see `state::entities::StatusEffect` and
    /// `engine::combat::update_status_effects` for what each type actually does).
    pub status_type: String,
    pub duration: f32,
    /// Poison/burn: damage per second. Freeze: movement speed multiplier while active
    /// (e.g. 0.5 = 50% slow). Stun: unused, any value works.
    pub strength: f32,
    /// Chance in [0.0, 1.0] to proc on a landed hit.
    pub proc_chance: f32,
}

pub fn load_game_config() -> Result<GameConfig, Box<dyn Error>> {
    let json_content = macroquad_toolkit::include_json_str!("../../assets/data/game_config.json");
    let config: GameConfig = serde_json::from_str(json_content)?;
    Ok(config)
}
