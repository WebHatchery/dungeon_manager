//! Balance Integration Tests
#![allow(dead_code)]
//!
//! These tests verify that game balance values are within acceptable ranges.
//! Run with: cargo test --test balance_tests
//!
//! Tests are organized by system:
//! - Resource economy
//! - Combat balance
//! - Wave scaling
//! - Creature costs
//! - Room effects

// ============================================================================
// Data Loading (duplicated from balance_calculator for test isolation)
// ============================================================================

mod data {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize)]
    pub struct MonsterData {
        pub id: String,
        pub name: String,
        pub stats: CreatureStats,
        /// Trait tags. Needed so balance checks can tell a creature bought for
        /// its body from one bought for an aura or a research multiplier.
        #[serde(default)]
        pub traits: Vec<String>,
        #[serde(default)]
        pub combat: Option<CombatData>,
        #[serde(default)]
        pub economy: Option<EconomyData>,
    }

    #[derive(Debug, Deserialize)]
    pub struct CreatureStats {
        pub health: f32,
        pub attack: f32,
        pub defense: f32,
        pub speed: f32,
    }

    #[derive(Debug, Deserialize)]
    pub struct CombatData {
        #[serde(default)]
        pub damage_range: Option<[f32; 2]>,
        #[serde(default)]
        pub attack_speed: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct EconomyData {
        #[serde(default)]
        pub wage_per_minute: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct HeroData {
        pub id: String,
        pub name: String,
        pub stats: CreatureStats,
        #[serde(default)]
        pub tier: Option<u32>,
        #[serde(default)]
        pub combat: Option<CombatData>,
    }

    #[derive(Debug, Deserialize)]
    pub struct RoomData {
        pub id: String,
        pub name: String,
        #[serde(default)]
        pub build: Option<RoomBuild>,
        #[serde(default)]
        pub effects: Option<RoomEffects>,
    }

    #[derive(Debug, Deserialize)]
    pub struct RoomBuild {
        #[serde(default)]
        pub cost_per_tile: i32,
        #[serde(default)]
        pub min_tiles: Option<u32>,
        #[serde(default)]
        pub max_tiles: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct RoomEffects {
        #[serde(default)]
        pub food_generation_per_second: Option<f32>,
        #[serde(default)]
        pub mana_generation_per_second: Option<f32>,
        #[serde(default)]
        pub gold_storage_capacity: Option<i32>,
        #[serde(default)]
        pub research_speed: Option<f32>,
        #[serde(default)]
        pub training_xp_per_second: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct TrapData {
        pub id: String,
        pub name: String,
        pub cost: i32,
        #[serde(default)]
        pub build_time: Option<f32>,
        #[serde(default)]
        pub effects: Option<TrapEffects>,
    }

    #[derive(Debug, Deserialize)]
    pub struct TrapEffects {
        #[serde(default)]
        pub damage: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct GameConfig {
        pub player_starting_resources: ResourceConfig,
        pub player_initial_capacity: CapacityConfig,
        pub hero_waves: WaveConfig,
        pub combat: CombatConfig,
    }

    #[derive(Debug, Deserialize)]
    pub struct ResourceConfig {
        pub gold: i32,
        pub mana: i32,
        pub food: i32,
    }

    #[derive(Debug, Deserialize)]
    pub struct CapacityConfig {
        pub max_gold: i32,
        pub max_mana: i32,
    }

    #[derive(Debug, Deserialize)]
    pub struct WaveConfig {
        pub initial_delay: f32,
        pub wave_interval: f32,
        pub wave_scaling_multiplier: f32,
        #[serde(default)]
        pub spawn_rate_decay: Option<f32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct CombatConfig {
        pub attack_stat_bonus: f32,
        pub defense_reduction: f32,
        #[serde(default)]
        pub creature_level_multiplier: Option<f32>,
        #[serde(default)]
        pub hero_level_multiplier: Option<f32>,
    }

    pub fn load_monsters() -> HashMap<String, MonsterData> {
        let json = macroquad_toolkit::include_json_str!("../assets/data/monsters.json");
        let list: Vec<MonsterData> =
            serde_json::from_str(json).expect("Failed to parse monsters.json");
        list.into_iter().map(|m| (m.id.clone(), m)).collect()
    }

    pub fn load_heroes() -> HashMap<String, HeroData> {
        let json = macroquad_toolkit::include_json_str!("../assets/data/heroes.json");
        let list: Vec<HeroData> = serde_json::from_str(json).expect("Failed to parse heroes.json");
        list.into_iter().map(|h| (h.id.clone(), h)).collect()
    }

    pub fn load_rooms() -> HashMap<String, RoomData> {
        let json = macroquad_toolkit::include_json_str!("../assets/data/rooms.json");
        let list: Vec<RoomData> = serde_json::from_str(json).expect("Failed to parse rooms.json");
        list.into_iter().map(|r| (r.id.clone(), r)).collect()
    }

    pub fn load_traps() -> HashMap<String, TrapData> {
        let json = macroquad_toolkit::include_json_str!("../assets/data/traps.json");
        let list: Vec<TrapData> = serde_json::from_str(json).expect("Failed to parse traps.json");
        list.into_iter().map(|t| (t.id.clone(), t)).collect()
    }

    pub fn load_config() -> GameConfig {
        let json = macroquad_toolkit::include_json_str!("../assets/data/game_config.json");
        serde_json::from_str(json).expect("Failed to parse game_config.json")
    }
}

// ============================================================================
// Test Modules
// ============================================================================

#[path = "balance_tests/combat.rs"]
mod combat;
#[path = "balance_tests/content.rs"]
mod content;
#[path = "balance_tests/economy.rs"]
mod economy;
