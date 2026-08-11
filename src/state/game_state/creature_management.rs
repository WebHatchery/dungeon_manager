use super::*;

impl GameState {
    pub fn get_tile(&self, pos: TilePos) -> Option<&crate::state::tile_state::TileState> {
        self.dungeon.get_tile(pos)
    }

    pub fn get_tile_mut(
        &mut self,
        pos: TilePos,
    ) -> Option<&mut crate::state::tile_state::TileState> {
        self.dungeon.get_tile_mut(pos)
    }

    pub(super) fn update_creature_ai_and_movement(&mut self, game_data: &GameData, dt: f32) {
        use crate::engine::creature_ai;

        // Delegate to the creature_ai module
        creature_ai::update_creatures(
            &self.dungeon,
            &mut self.entities,
            &self.room_manager,
            game_data,
            dt,
            self.attack_marker,
            self.defend_marker,
            |task, room_manager, entities| {
                GameState::get_task_target_position_static(task, room_manager, entities)
            },
        );
    }

    /// Static version of get_task_target_position that doesn't require &self
    pub(super) fn get_task_target_position_static(
        task: &crate::state::entities::Task,
        room_manager: &crate::state::room_manager::RoomManager,
        entities: &crate::state::entities::EntityManager,
    ) -> Option<TilePos> {
        use crate::state::entities::Task;

        match task {
            Task::Sleep(room_id) | Task::Eat(room_id) => room_manager
                .rooms
                .iter()
                .find(|r| r.id == *room_id)
                .map(|room: &crate::engine::room_validator::Room| room.get_center()),
            Task::Work(_, pos) => Some(*pos),
            Task::Train(room_id)
            | Task::Research(room_id)
            | Task::DepositGold(room_id)
            | Task::CollectWages(room_id) => room_manager
                .rooms
                .iter()
                .find(|r| r.id == *room_id)
                .map(|room: &crate::engine::room_validator::Room| room.get_center()),
            Task::Dig(pos) => Some(*pos),
            Task::ClaimTile(pos) => Some(*pos),
            Task::PickupResource(target_id) => entities.get(*target_id).map(|e| e.pos),
            Task::MoveTo(pos) => Some(*pos),
            Task::Attack(entity_id) => entities.get(*entity_id).map(|e| e.pos),
            Task::Idle | Task::Flee => None,
        }
    }

    pub(super) fn perform_creature_task(
        &mut self,
        creature_id: EntityId,
        game_data: &GameData,
        dt: f32,
    ) {
        use crate::engine::task_system;

        // Delegate to task_system module
        let result = task_system::execute_task(
            creature_id,
            &mut self.entities,
            &self.room_manager,
            &self.player,
            game_data,
            dt,
        );

        // Apply state changes from task result
        if result.gold_change.abs() > 0.001 {
            self.player
                .add_resources_precise(result.gold_change, 0.0, 0.0, 0.0);
        }
        if result.food_change.abs() > 0.001 {
            self.player
                .add_resources_precise(0.0, 0.0, result.food_change, 0.0);
        }
        if result.materials_change.abs() > 0.001 {
            self.player
                .add_resources_precise(0.0, 0.0, 0.0, result.materials_change);
        }
        if let Some(trap_id) = result.manufactured_trap {
            self.player.add_trap_inventory(trap_id.clone(), 1);
            self.notifications
                .info(format!("Manufactured {} crate.", trap_id));
        }
        if result.research_change > 0.0 {
            if let Some(active_tech_id) = &self.player.active_research {
                // Determine cost
                let cost = if let Some(tech) = game_data.technologies.get(active_tech_id) {
                    tech.cost
                } else {
                    100.0 // Fallback
                };

                if let Some(completed) =
                    self.player
                        .update_research(result.research_change, cost, dt)
                {
                    // Research completed!
                    if let Some(tech) = game_data.technologies.get(&completed) {
                        self.player.complete_research(tech);
                        eprintln!("Research Complete: {}", tech.name);
                        self.notifications
                            .success(format!("Research Complete: {}", tech.name));
                    }
                }
            }
        }
        if let Some(tile_pos) = result.claimed_tile {
            if let Some(tile) = self.get_tile_mut(tile_pos) {
                tile.tile_type = crate::engine::tile_types::types::CLAIMED_FLOOR.to_string();
                tile.ownership = Ownership::Player;
                tile.marked_for_dig = false;
                self.player.claimed_tile_count += 1;
            }
        }
    }

    /// Release lair tile claimed by an entity (when creature dies/leaves)
    pub(super) fn release_lair_tile(&mut self, entity_id: crate::state::entities::EntityId) {
        self.room_manager
            .release_lair_tile(&mut self.dungeon, entity_id);
    }

    /// Generate food from hatcheries based on their size
    /// Generate food from hatcheries based on their size
    pub(super) fn generate_food_from_hatcheries(&mut self, dt: f32) {
        let total_food_generated = self.room_manager.generate_food_from_hatcheries(dt);
        if total_food_generated > 0.0 {
            self.player
                .add_resources_precise(0.0, 0.0, total_food_generated, 0.0);
        }
    }

    /// Find the dungeon heart tile position
    /// Combined hero-threat multiplier: the mission's authored
    /// `threat_multiplier` (1.0 if none) scaled by the chosen difficulty. Higher
    /// = the surface presses harder (faster garrison spawns, more frequent
    /// waves). Clamped to a small positive floor so it can safely divide timers.
    /// The mission dial times the difficulty scale, raised by any creatures the
    /// surface world has noticed.
    ///
    /// The creature term is what makes "attracts attention" a mechanic: this
    /// value sets both the gap between hero waves and how fast the hero
    /// garrison replenishes, so keeping a Void-Touched costs you time.
    pub fn effective_threat_multiplier(&self, game_data: &GameData) -> f32 {
        let scenario = self
            .scenario_runtime
            .as_ref()
            .map(|runtime| runtime.active_rules.threat_multiplier)
            .unwrap_or(1.0);
        (scenario * self.difficulty.threat_scale() * self.creature_threat_factor(game_data))
            .max(0.1)
    }

    /// `1.0` plus the summed `threat_contribution` of every living creature,
    /// capped so a large horde cannot run the wave clock away entirely.
    pub(super) fn creature_threat_factor(&self, game_data: &GameData) -> f32 {
        const MAX_CREATURE_THREAT: f32 = 1.0;

        let drawn: f32 = self
            .entities
            .creatures()
            .filter(|(_, creature)| creature.health > 0.0)
            .filter_map(|(_, creature)| game_data.monsters.get(&creature.creature_id))
            .flat_map(|data| data.traits.iter())
            .filter_map(|tag| game_data.traits.get(tag))
            .map(|t| t.threat_contribution)
            .sum();

        1.0 + drawn.clamp(0.0, MAX_CREATURE_THREAT)
    }

    pub fn find_dungeon_heart_position(&self) -> Option<TilePos> {
        for row in &self.dungeon.grid {
            for tile in row {
                if tile.tile_type == "dungeon_heart" && tile.ownership == Ownership::Player {
                    return Some(tile.pos);
                }
            }
        }
        None
    }

    /// Count how many imps are currently spawned
    pub fn count_imps(&self) -> usize {
        self.entities
            .creatures()
            .filter(|(_, creature)| creature.creature_id == "imp")
            .count()
    }

    /// Maximum number of imps allowed (reads from monster data)
    pub fn max_imps(game_data: &GameData) -> usize {
        game_data
            .monsters
            .get("imp")
            .map(|m| m.spawn.max_population as usize)
            .unwrap_or(10)
    }

    /// Count only player-owned creatures (dungeon faction, excluding imps)
    pub fn count_player_creatures(&self, game_data: &GameData) -> usize {
        self.entities
            .creatures()
            .filter(|(_, creature)| {
                if let Some(monster_data) = game_data.monsters.get(&creature.creature_id) {
                    monster_data.faction == "dungeon"
                } else {
                    false
                }
            })
            .count()
    }

    // Process hero digging logic with flattened control flow.
    // Returns (carved_wall_pos, should_move).

    /// Check for victory or defeat conditions
    pub fn check_game_over_conditions(&mut self, game_data: &GameData) {
        crate::engine::objectives::update_victory_and_defeat(self, game_data);
    }
}
