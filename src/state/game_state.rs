//! Main game state container
//! Holds the dungeon grid, entities, rooms, and player state

mod creature_management;

use crate::data::GameData;
use crate::engine::combat::update_status_effects;
use crate::state::entities::{EntityId, EntityManager};
use crate::state::player_state::PlayerState;
use crate::state::projectiles::ProjectileManager;
use crate::state::rival_keeper::RivalKeeperRuntime;
use crate::state::room_manager::RoomManager;
use crate::state::scenario_state::ScenarioRuntimeState;
use crate::state::tile_state::{Ownership, TilePos};
use macroquad_toolkit::rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapType {
    Standard,     // Balanced resources and hazards
    Rich,         // Lots of gold and gems, few hazards
    Hazardous,    // Many water/lava pools, less resources
    Test,         // Fixed seed for testing
    File(String), // Load from specific JSON file
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub dungeon: crate::state::dungeon::Dungeon,
    pub room_manager: RoomManager,
    pub time_elapsed: f32,
    pub tick_accumulator: f32,
    pub camera: crate::state::camera_state::CameraState,
    // Track pending builds
    pub pending_trap_builds: std::collections::HashSet<crate::state::tile_state::TilePos>,

    pub entities: EntityManager,
    pub player: PlayerState,

    /// Where the dungeon is lit, rebuilt each tick from rooms and glowing
    /// tiles. Derived state, so it is not persisted — and shared rather than
    /// rebuilt separately by the renderer and the simulation, which both need
    /// it now that darkness affects combat.
    #[serde(skip)]
    pub light_map: crate::engine::lighting::LightMap,
    pub next_hero_spawn_time: f32,
    pub next_creature_spawn_time: f32,
    pub pay_day_timer: f32,
    pub spawners: Vec<crate::engine::spawner_logic::MonsterSpawner>,
    pub paused: bool,
    pub hero_base: crate::state::hero_base::HeroBase,
    pub game_over: bool,
    pub victory: bool,

    /// Notification system for game events
    pub notifications: crate::state::notifications::NotificationManager,

    /// Dungeon Heart Health
    pub dungeon_heart_health: f32,

    // Markers
    pub attack_marker: Option<TilePos>,
    pub defend_marker: Option<TilePos>,

    /// Active attack projectiles for visual effects
    pub projectiles: ProjectileManager,

    /// In-game cheat menu state
    pub cheat_menu: crate::ui::cheat_menu::CheatMenuState,

    /// Cheat toggles
    pub cheat_fog_enabled: bool,
    pub cheat_immortal_heart: bool,

    #[serde(default)]
    pub active_scenario_id: Option<String>,
    #[serde(default)]
    pub scenario_runtime: Option<ScenarioRuntimeState>,
    #[serde(default)]
    pub campaign_progress: Option<crate::data::campaign::CampaignProgress>,
    #[serde(default)]
    pub rival_keepers: RivalKeeperRuntime,
    #[serde(default)]
    pub tutorial: crate::state::tutorial::TutorialState,
    /// Global difficulty, copied from settings at launch. Scales hero threat on
    /// top of the mission's authored `threat_multiplier`.
    #[serde(default)]
    pub difficulty: crate::state::settings::Difficulty,

    /// The slot this session saves to and loads from — the single owner of
    /// slot identity, replacing the `"slot_1"` literal that used to sit at
    /// eleven call sites. Skipped rather than serialized: which drawer a game
    /// was filed in is a property of the session, not of the game, and storing
    /// it would let a save copied into slot 2 insist it belongs in slot 1.
    #[serde(skip)]
    pub active_slot: crate::state::save_system::SaveSlot,

    /// Open slot browser, drawn over the pause menu. An overlay rather than a
    /// `GamePhase` because the running game has to stay somewhere while the
    /// player picks, and `GamePhase::Playing` is where it lives.
    #[serde(skip)]
    pub slot_browser: Option<crate::state::interaction::SlotBrowser>,
}

impl GameState {
    pub fn new(width: usize, height: usize, game_data: &GameData) -> Self {
        Self::new_with_map_type(width, height, game_data, MapType::Standard)
    }

    pub fn new_for_scenario(game_data: &GameData, scenario_id: &str) -> Self {
        let Some(scenario) = game_data.scenarios.get(scenario_id) else {
            return Self::new(
                game_data.config.map_size.width,
                game_data.config.map_size.height,
                game_data,
            );
        };

        let width = scenario
            .map
            .width
            .unwrap_or(game_data.config.map_size.width);
        let height = scenario
            .map
            .height
            .unwrap_or(game_data.config.map_size.height);
        let mut state = Self::new_with_map_type(
            width,
            height,
            game_data,
            MapType::File(
                game_data
                    .resolve_map_path(&scenario.map.path)
                    .to_string_lossy()
                    .to_string(),
            ),
        );

        state.active_scenario_id = Some(scenario.meta.id.clone());
        state.tutorial = crate::state::tutorial::TutorialState::for_new_scenario();
        state.scenario_runtime = Some(ScenarioRuntimeState::from_definition(scenario));
        state.rival_keepers.merge_from_scenario(scenario);
        state.apply_scenario_setup(game_data, scenario_id);
        state
    }

    pub fn new_campaign_start(game_data: &GameData, campaign_id: &str) -> Self {
        let Some(campaign) = game_data.campaigns.get(campaign_id) else {
            return Self::new(
                game_data.config.map_size.width,
                game_data.config.map_size.height,
                game_data,
            );
        };

        let progress = crate::data::campaign::CampaignProgress::new(campaign);
        let scenario_id = campaign
            .missions
            .iter()
            .find(|mission| mission.id == progress.active_mission)
            .map(|mission| mission.scenario_id.as_str())
            .unwrap_or("dark_beginnings");

        let mut state = Self::new_for_scenario(game_data, scenario_id);
        state.apply_campaign_unlocks(&progress.persistent_unlocks);
        state.campaign_progress = Some(progress);
        state
    }

    pub fn new_with_map_type(
        width: usize,
        height: usize,
        game_data: &GameData,
        map_type: MapType,
    ) -> Self {
        let mut entities = EntityManager::new();
        let mut dungeon;
        let mut map_rival_keepers = RivalKeeperRuntime::default();

        match &map_type {
            MapType::File(path) => {
                println!("Loading map from: {}", path);
                dungeon = match crate::state::map_loader::load_map(path, game_data, &mut entities) {
                    Ok(loaded) => {
                        map_rival_keepers =
                            crate::state::map_loader::load_rival_keeper_runtime(path)
                                .unwrap_or_default();
                        loaded
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to load map: {}. Falling back to standard generation.",
                            e
                        );
                        crate::state::dungeon::Dungeon::new(
                            width,
                            height,
                            game_data,
                            MapType::Standard,
                        )
                    }
                };
            }
            _ => {
                dungeon =
                    crate::state::dungeon::Dungeon::new(width, height, game_data, map_type.clone());
                // Procedurally-generated maps place a dungeon heart but don't
                // claim it; do so now so the generated map is actually the
                // player's (the File loader does this inside load_map).
                crate::state::map_loader::claim_map_heart_areas(&mut dungeon);
            }
        }

        let mut room_manager = RoomManager::new();

        // Detect and register starting rooms (either generated or loaded)
        room_manager.detect_and_update_rooms(&mut dungeon, game_data);

        let spawners = crate::state::spawning::detect_monster_spawners(&dungeon, game_data);

        let mut state = Self {
            dungeon,
            room_manager,
            active_slot: crate::state::save_system::SaveSlot::default(),
            slot_browser: None,
            time_elapsed: 0.0,
            tick_accumulator: 0.0,
            camera: crate::state::camera_state::CameraState::new(
                width as f32 / 2.0,
                height as f32 / 2.0,
            ),
            pending_trap_builds: HashSet::new(),

            entities,
            player: PlayerState::new(game_data),
            light_map: crate::engine::lighting::LightMap::default(),
            next_hero_spawn_time: 0.0,
            next_creature_spawn_time: 0.0,
            pay_day_timer: 0.0,
            spawners,
            paused: false,
            hero_base: crate::state::hero_base::HeroBase::new(game_data),
            game_over: false,
            victory: false,
            notifications: crate::state::notifications::NotificationManager::new(),
            dungeon_heart_health: game_data.config.dungeon.heart_max_health,
            attack_marker: None,
            defend_marker: None,
            projectiles: ProjectileManager::new(),
            cheat_menu: crate::ui::cheat_menu::CheatMenuState::default(),
            cheat_fog_enabled: false,
            cheat_immortal_heart: false,
            active_scenario_id: None,
            scenario_runtime: None,
            campaign_progress: None,
            rival_keepers: map_rival_keepers,
            tutorial: crate::state::tutorial::TutorialState::default(),
            difficulty: crate::state::settings::Difficulty::default(),
        };

        // Recalculate max gold and other room-based stats
        state.detect_and_update_rooms(game_data);

        // Spawn starting imps only if none exist (loaded map might have them)
        let imp_count = state
            .entities
            .all()
            .filter_map(|e| e.as_creature())
            .filter(|c| c.creature_id == "imp")
            .count();

        if imp_count == 0 {
            state.spawn_starting_imps(game_data, game_data.config.dungeon.initial_imp_count);
        }

        state
    }

    pub fn update(&mut self, dt: f32, game_data: &GameData) {
        self.time_elapsed += dt;
        self.tick_accumulator += dt;
        self.camera.update(dt); // Update smooth camera zoom
        self.notifications.update(dt); // Update notification display timers

        // Update attack projectiles and resolve impacts
        let impacts = self.projectiles.update(dt);
        for impact in impacts {
            // Resolve deferred combat impact
            crate::engine::combat::apply_projectile_impact(
                &impact,
                &mut self.entities,
                game_data,
                self.time_elapsed,
            );
        }

        // Smooth movement interpolation
        for entity in self.entities.all_mut() {
            let target_x = entity.pos.x as f32;
            let target_z = entity.pos.y as f32;

            let dx = target_x - entity.visual_pos.0;
            let dz = target_z - entity.visual_pos.1;

            // Simple lerp
            let speed = crate::config::MOVEMENT_LERP_SPEED * dt;
            entity.visual_pos.0 += dx * speed;
            entity.visual_pos.1 += dz * speed;

            // Snap if close
            if dx.abs() < 0.01 && dz.abs() < 0.01 {
                entity.visual_pos.0 = target_x;
                entity.visual_pos.1 = target_z;
            }
        }

        // Fixed timestep simulation (10 ticks per second)
        while self.tick_accumulator >= crate::config::TICK_RATE {
            self.tick(game_data, crate::config::TICK_RATE);
            self.tick_accumulator -= crate::config::TICK_RATE;
        }
    }

    fn tick(&mut self, game_data: &GameData, dt: f32) {
        // Update spell cooldowns
        crate::engine::spell_effects::update_spell_cooldowns(self, dt);

        // Evaluate and fire data-driven hero abilities
        crate::engine::hero_abilities::update_hero_abilities(self, game_data, dt);

        // Imp spawning is now handled by the Summon Imp spell

        // Update imps first (they dig tiles)
        crate::engine::imp_ai::update_imp_digging(
            &mut self.dungeon,
            &mut self.entities,
            &mut self.player,
            game_data,
            dt,
        );

        // Drain anything the engine layers wanted the player to know. They
        // hold `&mut PlayerState` but not the notification manager, so this is
        // their only route to the screen.
        let pending = std::mem::take(&mut self.player.pending_messages);
        for message in pending {
            self.notifications.warning(message);
        }

        // Update creature AI and movement
        self.update_creature_ai_and_movement(game_data, dt);

        // Execute creature tasks (Work, Eat, Sleep, etc.)
        let creature_ids: Vec<crate::state::entities::EntityId> =
            self.entities.creatures().map(|(id, _)| id).collect();
        for id in creature_ids {
            self.perform_creature_task(id, game_data, dt);
        }

        // Hero spawning via Hero Base
        crate::engine::hero_spawner::update_hero_spawning(self, game_data, dt);

        // Scenario scripted events
        crate::engine::scenario_events::update_scenario_events(self, game_data);

        // Tutorial objective progression
        crate::engine::tutorial_system::update_tutorial(self, game_data);

        // Rival keeper planning and reinforcement behavior
        crate::engine::rival_keeper_ai::update_rival_keeper_ai(self, game_data, dt);

        // NPC Spawner Update
        crate::engine::spawner_logic::SpawnerSystem::update(
            &mut self.spawners,
            &self.dungeon.grid,
            &mut self.entities,
            game_data,
            dt,
        );

        // Creature spawning
        self.next_creature_spawn_time -= dt;
        if self.next_creature_spawn_time <= 0.0 {
            self.spawn_random_creature(game_data);
            let spawn_range = game_data.config.timing.creature_spawn_max_interval
                - game_data.config.timing.creature_spawn_min_interval;
            self.next_creature_spawn_time =
                game_data.config.timing.creature_spawn_min_interval + rng::rand() * spawn_range;
        }

        // Pay Day Logic
        self.pay_day_timer += dt;
        if self.pay_day_timer >= game_data.config.timing.pay_day_interval {
            self.pay_day_timer = 0.0;
            self.trigger_pay_day();
        }

        // Update hero AI
        let hero_entities: Vec<EntityId> = self.entities.heroes().map(|(id, _)| id).collect();

        if self.hero_base.enabled {
            crate::engine::hero_spawner::update_hero_spawning(self, game_data, dt);
        }

        self.process_dungeon_heart_attacks(game_data, dt);
        for hero_id in hero_entities {
            // Update AI first
            if let Some(entity) = self.entities.get(hero_id) {
                if let Some(hero_state) = entity.as_hero() {
                    // Skip AI for captured heroes
                    if hero_state.is_captured {
                        continue;
                    }

                    let mut hero_state_clone = hero_state.clone();
                    crate::engine::hero_ai::update_hero_ai(
                        entity,
                        &mut hero_state_clone,
                        self,
                        game_data,
                    );

                    // HANDLE GOLD PICKUP
                    // If hero is on a tile with a gold pile, pick it up
                    // This is done here because we need mutable access to entities (to remove the pile)
                    let hero_pos = entity.pos;
                    let mut picked_up_value = 0;
                    let mut pile_to_remove = None;

                    // Scan for pile at hero position
                    // We can't iterate self.entities mutably while iterating hero_ids easily,
                    // but we can query by position if we had a spatial map, or just scan all (slow but safe for now)
                    // Or better: scan all piles? Piles are entities.
                    // Doing a full scan inside a loop is O(N*M). Optimization: Only do this if hero goal is StealGold?
                    // Let's do it if hero_state_clone.current_goal is StealGold or Explore

                    let can_pickup = matches!(
                        hero_state_clone.current_goal,
                        crate::state::entities::HeroGoal::StealGold(_)
                            | crate::state::entities::HeroGoal::Explore
                    );

                    if can_pickup {
                        for (other_id, other) in self.entities.entities() {
                            if *other_id == hero_id {
                                continue;
                            }
                            if other.pos == hero_pos {
                                if let crate::state::entities::EntityType::ResourcePile(pile) =
                                    &other.entity_type
                                {
                                    if pile.resource_type == "gold" {
                                        picked_up_value = pile.amount;
                                        pile_to_remove = Some(*other_id);
                                        break; // Pick up one at a time
                                    }
                                }
                            }
                        }
                    }

                    if let Some(pile_id) = pile_to_remove {
                        self.entities.remove(pile_id);
                        hero_state_clone.gold_stolen += picked_up_value;
                        eprintln!("Hero {} stole gold pile worth {}", hero_id, picked_up_value);
                    }

                    // Update the entity with the new hero state
                    if let Some(entity_mut) = self.entities.get_mut(hero_id) {
                        if let Some(hero_state_mut) = entity_mut.as_hero_mut() {
                            *hero_state_mut = hero_state_clone;
                        }
                    }
                }
            }

            // Handle hero digging logic
            // Handle hero digging logic
            let (carved_wall, should_move) = crate::engine::hero_digging::process_hero_digging(
                &mut self.entities,
                &self.dungeon,
                game_data,
                hero_id,
                dt,
            );

            if let Some(pos) = carved_wall {
                if let Some(tile) = self.dungeon.get_tile_mut(pos) {
                    tile.tile_type = "claimed_floor".to_string();
                    tile.ownership = crate::state::tile_state::Ownership::Unclaimed;
                }
            }

            // Process movement for hero if not digging
            if should_move {
                crate::engine::movement::process_entity_movement(
                    &mut self.entities,
                    hero_id,
                    dt,
                    self.hero_base.hero_speed_multiplier(),
                );
            }
        }

        // Resolve combat
        self.resolve_combat(game_data, dt);

        // Update status effects for all entities
        let all_entity_ids: Vec<EntityId> = self.entities.all().map(|e| e.id).collect();
        for entity_id in all_entity_ids {
            if let Some(entity) = self.entities.get_mut(entity_id) {
                update_status_effects(entity, dt);
            }
        }

        // Generate food from hatcheries
        self.generate_food_from_hatcheries(dt);

        // Check for starving creatures that need to desert
        crate::engine::creature_needs::handle_creature_desertion(
            &mut self.entities,
            &self.room_manager,
            &mut self.dungeon,
            game_data,
        );

        // Handle dead heroes - check for prison capture
        crate::engine::prison_system::handle_prison_captures(
            &mut self.entities,
            &self.room_manager,
            &mut self.notifications,
            game_data,
        );

        // Progress prison conversions
        crate::engine::prison_system::progress_prison_conversions(
            &mut self.entities,
            &self.room_manager,
            &mut self.notifications,
            game_data,
            dt,
        );

        // Process room-specific mechanics that are not generic work tasks
        crate::engine::special_rooms::process_special_rooms(self, game_data, dt);

        // Where the dungeon is lit. Rebuilt before anything reads it, since
        // both the renderer and darkness-sensitive creatures consume it.
        self.light_map = crate::engine::lighting::build_light_map(self, game_data);
        crate::engine::lighting::cache_creature_darkness(self);

        // Overseers raising the work rate of whoever is standing near them.
        // Must run before task execution reads command_bonus.
        crate::engine::command_aura::apply_command_auras(self, game_data);

        // Grafted creatures rolling the traits that make them individuals
        crate::engine::grafting::graft_random_traits(self, game_data);

        // Creatures evolving once their authored conditions are met
        crate::engine::mutation::apply_mutations(self, game_data);

        // Stone Wardens shoring up walls into rock heroes cannot tunnel
        crate::engine::wall_reinforcement::reinforce_walls(self, game_data, dt);

        // Remove dead entities and release their lair space (but not captured heroes)
        let dead_ids: Vec<EntityId> = self
            .entities
            .all()
            .filter(|e| {
                if !e.is_alive() {
                    // Don't remove captured heroes
                    if let Some(hero) = e.as_hero() {
                        return !hero.is_captured;
                    }
                    return true;
                }
                false
            })
            .map(|e| e.id)
            .collect();

        for entity_id in dead_ids {
            // Check if it's a structure before removing
            let mut structure_pos = None;
            if let Some(entity) = self.entities.get(entity_id) {
                if let crate::state::entities::EntityType::Structure(_) = entity.entity_type {
                    structure_pos = Some(entity.pos);
                }
            }

            self.release_lair_tile(entity_id);
            self.entities.remove(entity_id);

            // If a structure died, destroy the tile
            if let Some(pos) = structure_pos {
                if let Some(tile) = self.dungeon.get_tile_mut(pos) {
                    // Turn into rubble or floor
                    trace_log!("combat", "Structure at {:?} destroyed", pos);
                    tile.tile_type = "claimed_floor".to_string(); // Or specific rubble tile if exists
                    tile.ownership = Ownership::Unclaimed; // Reset ownership? Or keep Player/Hero?
                                                           // Usually destroying enemy room makes it neutral or effectively 'floor'
                }

                // Also remove from hero_base list, banking whatever its
                // destruction was authored to cost the hero faction.
                if let Some(building_idx) =
                    self.hero_base.buildings.iter().position(|b| b.pos == pos)
                {
                    let building = self.hero_base.buildings.remove(building_idx);
                    if let Some(data) = game_data.hero_buildings.get(&building.building_type) {
                        self.hero_base
                            .apply_destruction_effect(&data.destruction_effect);
                        let message = match data.destruction_effect.describe() {
                            Some(effect) => format!("{} destroyed — {effect}.", data.name),
                            None => format!("{} destroyed.", data.name),
                        };
                        self.notifications.success(message);
                    }
                }
            }
        }

        // Update fog of war based on claimed tiles and creature positions
        self.update_fog_of_war_system(game_data);

        // Update trap construction
        crate::engine::trap_system::process_trap_construction(
            &mut self.dungeon,
            &mut self.player,
            &mut self.pending_trap_builds,
            game_data,
            dt,
        );

        // Process trap triggers (when heroes step on traps)
        let trap_results = crate::engine::trap_system::process_trap_triggers(
            &mut self.dungeon,
            &mut self.entities,
            game_data,
            dt,
        );

        // Notify about trap triggers
        for result in trap_results {
            self.notifications.info(format!(
                "{} triggered! ({:.0} damage)",
                result.trap_type, result.damage_dealt
            ));
        }

        // Update creature count (only player creatures, excluding imps and wild monsters)
        self.player.current_creature_count =
            self.count_player_creatures(game_data) - self.count_imps();

        // Check for Game Over
        self.check_game_over_conditions(game_data);
    }
}
