//! Player state - resources, research, and unlocks
//! Tracks the player's economy, technology tree, and game progress

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Accumulator for fractional resource gains
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceAccumulator {
    pub gold: f32,
    pub mana: f32,
    pub food: f32,
    pub materials: f32,
}

/// Player's current resources and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    // Economic resources
    pub gold: i32,
    pub mana: i32,
    pub food: i32,
    pub max_gold: i32,
    pub max_mana: i32,
    pub max_food: i32,
    pub materials: i32,
    pub max_materials: i32,
    /// Warnings the player has already been given, by key.
    ///
    /// Several conditions are checked every tick — an overflowing treasury,
    /// a spawner cut off from the heart — so reporting them unconditionally
    /// would bury the screen. See `warn_once`.
    #[serde(default)]
    pub warned_keys: std::collections::HashSet<String>,
    /// Messages from engine layers that hold `&mut PlayerState` but have no
    /// access to the notification manager — imp digging, task execution.
    /// `GameState` drains these each tick.
    ///
    /// Before this the only way to tell the player anything from down there
    /// was an `eprintln!` they would never read. Not persisted: a message
    /// queued at the instant of a save means nothing on load.
    #[serde(skip)]
    pub pending_messages: Vec<String>,

    // Fractional resource accumulator
    #[serde(default)]
    pub accumulators: ResourceAccumulator,

    // Research and unlocks
    pub unlocked_rooms: HashSet<String>,
    pub unlocked_creatures: HashSet<String>,
    pub unlocked_spells: HashSet<String>,
    #[serde(default)]
    pub unlocked_traps: HashSet<String>,
    #[serde(default)]
    pub trap_inventory: HashMap<String, u32>,
    #[serde(default)]
    pub trap_manufacturing_progress: HashMap<String, f32>,
    pub completed_technologies: HashSet<String>,
    pub research_points: i32,
    pub active_research: Option<String>,
    pub research_progress: f32,

    // Game state
    pub dungeon_heart_health: f32,
    pub max_creatures: usize,
    pub current_creature_count: usize,
    pub claimed_tile_count: usize,

    // Spell cooldowns (spell_id -> remaining time in seconds)
    pub spell_cooldowns: HashMap<String, f32>,

    // Statistics
    pub kills: HashMap<String, u32>,
    pub deaths: HashMap<String, u32>,
    pub gold_mined: u32,
    pub spells_cast: HashMap<String, u32>,
    pub game_time: f32,

    #[serde(default)]
    pub graveyard_corpses: u32,
    #[serde(default)]
    pub scavenger_conversion_progress: HashMap<usize, f32>,
}

impl PlayerState {
    /// Create a new player state with starting values, auto-unlocking all defined rooms
    pub fn new(game_data: &crate::data::GameData) -> Self {
        let mut unlocked_rooms = HashSet::new();
        // Base rooms available at start
        unlocked_rooms.insert("lair".to_string());
        unlocked_rooms.insert("hatchery".to_string());
        unlocked_rooms.insert("treasury".to_string());
        unlocked_rooms.insert("library".to_string());
        unlocked_rooms.insert("dungeon_heart".to_string()); // Always needed

        let mut unlocked_creatures = HashSet::new();
        unlocked_creatures.insert("imp".to_string());

        let mut unlocked_spells = HashSet::new();
        unlocked_spells.insert("summon_imps".to_string());
        unlocked_spells.insert("lightning_strike".to_string());
        unlocked_spells.insert("heal".to_string());
        unlocked_spells.insert("speed_boost".to_string());
        // unlocked_spells.insert("create_food".to_string()); // Removed as it wasn't in json

        let mut unlocked_traps = HashSet::new();
        unlocked_traps.insert("door".to_string());

        Self {
            gold: game_data.config.player_starting_resources.gold,
            mana: game_data.config.player_starting_resources.mana,
            food: game_data.config.player_starting_resources.food,
            max_gold: game_data.config.player_initial_capacity.max_gold,
            max_mana: game_data.config.player_initial_capacity.max_mana,

            max_food: game_data.config.player_initial_capacity.max_food,
            materials: game_data.config.player_starting_resources.materials,
            max_materials: game_data.config.player_initial_capacity.max_materials,
            warned_keys: std::collections::HashSet::new(),
            pending_messages: Vec::new(),

            accumulators: ResourceAccumulator::default(),

            unlocked_rooms,
            unlocked_creatures,
            unlocked_spells,
            unlocked_traps,
            trap_inventory: HashMap::new(),
            trap_manufacturing_progress: HashMap::new(),
            completed_technologies: HashSet::new(),
            research_points: 0,
            active_research: None,
            research_progress: 0.0,

            dungeon_heart_health: 100.0,
            max_creatures: 20,
            current_creature_count: 0,
            claimed_tile_count: 0,

            spell_cooldowns: HashMap::new(),

            kills: HashMap::new(),
            deaths: HashMap::new(),
            gold_mined: 0,
            spells_cast: HashMap::new(),
            game_time: 0.0,
            graveyard_corpses: 0,
            scavenger_conversion_progress: HashMap::new(),
        }
    }

    /// Add resources, respecting max limits
    pub fn add_resources(&mut self, gold: i32, mana: i32, food: i32, materials: i32) {
        self.gold = (self.gold + gold).min(self.max_gold);
        self.mana = (self.mana + mana).min(self.max_mana);
        self.food = (self.food + food).min(self.max_food);
        self.materials = (self.materials + materials).min(self.max_materials);
    }

    /// Queue a message for the player from a layer that cannot reach the
    /// notification manager directly.
    pub fn notify(&mut self, message: impl Into<String>) {
        self.pending_messages.push(message.into());
    }

    /// Queue `message` the first time `key` is seen, and not again until
    /// [`clear_warning`](Self::clear_warning).
    ///
    /// Started as a single bool for the overflowing treasury; the spawner has
    /// three more conditions of the same shape, and five bools would have been
    /// worse than one set.
    pub fn warn_once(&mut self, key: &str, message: impl Into<String>) {
        if self.warned_keys.insert(key.to_string()) {
            self.notify(message);
        }
    }

    /// Called when a warned-about condition has demonstrably cleared, so its
    /// next occurrence is worth reporting again.
    pub fn clear_warning(&mut self, key: &str) {
        self.warned_keys.remove(key);
    }

    /// Check whether the player has enough gold and mana for a purchase.
    pub fn can_afford(&self, gold: i32, mana: i32) -> bool {
        self.gold >= gold && self.mana >= mana
    }

    /// Spend gold and mana if available.
    pub fn spend(&mut self, gold: i32, mana: i32) -> bool {
        if !self.can_afford(gold, mana) {
            return false;
        }

        self.gold -= gold;
        self.mana -= mana;
        true
    }

    /// Add resources with precision, accumulating fractional amounts
    pub fn add_resources_precise(&mut self, gold: f32, mana: f32, food: f32, materials: f32) {
        // Accumulate fractional partials
        self.accumulators.gold += gold;
        self.accumulators.mana += mana;
        self.accumulators.food += food;
        self.accumulators.materials += materials;

        // Extract integer parts
        let gold_int = self.accumulators.gold.trunc() as i32;
        let mana_int = self.accumulators.mana.trunc() as i32;
        let food_int = self.accumulators.food.trunc() as i32;
        let materials_int = self.accumulators.materials.trunc() as i32;

        // Reduce accumulators by the extracted integer parts
        self.accumulators.gold -= gold_int as f32;
        self.accumulators.mana -= mana_int as f32;
        self.accumulators.food -= food_int as f32;
        self.accumulators.materials -= materials_int as f32;

        // Add to main resources (can be negative consumption)
        self.add_resources(gold_int, mana_int, food_int, materials_int);
    }

    /// Check if a room type is unlocked
    pub fn is_room_unlocked(&self, room_id: &str) -> bool {
        self.unlocked_rooms.contains(room_id)
    }

    /// Check if a spell is unlocked
    pub fn is_spell_unlocked(&self, spell_id: &str) -> bool {
        self.unlocked_spells.contains(spell_id)
    }

    /// Unlock a room type
    pub fn unlock_room(&mut self, room_id: String) {
        self.unlocked_rooms.insert(room_id);
    }

    /// Unlock a creature type
    pub fn unlock_creature(&mut self, creature_id: String) {
        self.unlocked_creatures.insert(creature_id);
    }

    /// Unlock a spell
    pub fn unlock_spell(&mut self, spell_id: String) {
        self.unlocked_spells.insert(spell_id);
    }

    /// Unlock a trap or door
    pub fn unlock_trap(&mut self, trap_id: String) {
        self.unlocked_traps.insert(trap_id);
    }

    pub fn is_trap_unlocked(&self, trap_id: &str) -> bool {
        self.unlocked_traps.contains(trap_id)
    }

    pub fn trap_inventory_count(&self, trap_id: &str) -> u32 {
        self.trap_inventory.get(trap_id).copied().unwrap_or(0)
    }

    pub fn add_trap_inventory(&mut self, trap_id: String, amount: u32) {
        *self.trap_inventory.entry(trap_id).or_insert(0) += amount;
    }

    pub fn consume_trap_inventory(&mut self, trap_id: &str, amount: u32) -> bool {
        let available = self.trap_inventory_count(trap_id);
        if available < amount {
            return false;
        }

        if available == amount {
            self.trap_inventory.remove(trap_id);
        } else if let Some(count) = self.trap_inventory.get_mut(trap_id) {
            *count -= amount;
        }
        true
    }

    // Completed technologies are stored explicitly in `completed_technologies`
    // (persisted with PlayerState) — see `is_tech_completed` / `complete_research`
    // above. The research UI (`ui/sidebar`) gates each tech on
    // `prerequisites.all(|req| is_tech_completed(req))`, so the tree is a real
    // stored graph rather than something inferred from unlocked rooms/spells.

    /// Start researching a technology
    pub fn start_research(&mut self, research_id: String) {
        self.active_research = Some(research_id);
        self.research_progress = 0.0;
    }

    /// Update research progress
    pub fn update_research(
        &mut self,
        research_rate: f32,
        tech_cost: f32,
        dt: f32,
    ) -> Option<String> {
        if let Some(ref research_id) = self.active_research {
            self.research_progress += research_rate * dt;

            if self.research_progress >= tech_cost {
                let completed = research_id.clone();
                self.active_research = None;
                self.research_progress = 0.0;
                return Some(completed);
            }
        }
        None
    }

    /// Check if a technology is completed
    pub fn is_tech_completed(&self, tech_id: &str) -> bool {
        self.completed_technologies.contains(tech_id)
    }

    /// Complete a research and unlock rewards
    pub fn complete_research(&mut self, tech: &crate::data::TechData) {
        self.completed_technologies.insert(tech.id.clone());

        // Unlock rooms
        for room in &tech.unlocks.rooms {
            self.unlock_room(room.clone());
        }

        // Unlock spells
        for spell in &tech.unlocks.spells {
            self.unlock_spell(spell.clone());
        }

        // Unlock creatures
        for creature in &tech.unlocks.creatures {
            self.unlock_creature(creature.clone());
        }

        for trap in &tech.unlocks.traps {
            self.unlock_trap(trap.clone());
        }
    }

    /// Record a spell cast
    pub fn record_spell_cast(&mut self, spell_id: String) {
        *self.spells_cast.entry(spell_id).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests;
