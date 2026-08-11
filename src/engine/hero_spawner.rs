//! Hero spawning system with wave-based attack mechanics
//! Heroes spawn at buildings, are assigned defender/attacker roles,
//! and attack waves launch periodically.

use crate::data::GameData;
use crate::state::entities::{HeroGoal, HeroState};
use crate::state::game_state::GameState;
use crate::state::tile_state::TilePos;

/// Main update function for hero spawning and wave management
pub fn update_hero_spawning(state: &mut GameState, game_data: &GameData, dt: f32) {
    if !state.hero_base.enabled {
        return;
    }

    // Update wave timer and launch attacks
    update_wave_system(state, game_data, dt);

    // Handle hero spawning from buildings
    spawn_heroes_from_buildings(state, game_data, dt);
}

/// Update the wave attack system - countdown timer and launch logic
fn update_wave_system(state: &mut GameState, game_data: &GameData, dt: f32) {
    // Higher threat (mission + difficulty + creatures the surface has noticed)
    // shortens the gap between waves. It scales the *countdown* rather than the
    // interval the countdown starts from, which is what lets it reach wave 1:
    // `HeroBase::new` seeds `time_until_next_wave` from `initial_delay` before a
    // scenario or a difficulty has been chosen, so dividing at the source would
    // leave the first wave the one wave nobody can tune. Scaling here also means
    // a threat change mid-countdown — a creature summoned, or killed — takes
    // effect immediately, matching how the garrison timers in
    // `spawn_heroes_from_buildings` already read it.
    //
    // Consequence worth knowing: `time_until_next_wave` is denominated in
    // unscaled seconds, so real time to the next wave is the field divided by
    // threat. Nothing renders it; the trace below divides before printing.
    let threat = state.effective_threat_multiplier(game_data);

    // Count currently alive attackers to track wave status
    let alive_attackers = state
        .entities
        .heroes()
        .filter(|(_, hero)| !hero.is_defender && hero.wave_assigned > 0 && hero.health > 0.0)
        .count() as u32;

    state.hero_base.active_attackers = alive_attackers;

    // If wave was in progress but all attackers died, mark wave as complete
    if state.hero_base.wave_in_progress && alive_attackers == 0 {
        state.hero_base.wave_in_progress = false;
        state.hero_base.time_until_next_wave = game_data.config.hero_waves.wave_interval;
        trace_log!(
            "heroes",
            "Wave {} defeated! Next wave in {:.0} seconds.",
            state.hero_base.current_wave_number,
            state.hero_base.time_until_next_wave / threat
        );
    }

    // Countdown to next wave
    if !state.hero_base.wave_in_progress {
        state.hero_base.time_until_next_wave -= dt * threat;

        if state.hero_base.time_until_next_wave <= 0.0 {
            // Launch the next wave!
            launch_attack_wave(state, game_data);
        }
    }
}

/// Launch an attack wave - switch non-defender heroes from RestAtSpawn to DestroyHeart
fn launch_attack_wave(state: &mut GameState, _game_data: &GameData) {
    state.hero_base.current_wave_number += 1;
    state.hero_base.wave_in_progress = true;

    let wave_number = state.hero_base.current_wave_number;
    let mut attackers_launched = 0;

    // Collect hero IDs to update (can't mutate while iterating)
    let attacker_ids: Vec<_> = state
        .entities
        .heroes()
        .filter(|(_, hero)| !hero.is_defender && !hero.is_captured)
        .map(|(id, _)| id)
        .collect();

    for hero_id in attacker_ids {
        if let Some(entity) = state.entities.get_mut(hero_id) {
            if let Some(hero_state) = entity.as_hero_mut() {
                // Assign to this wave and switch goal to attack
                hero_state.wave_assigned = wave_number;
                hero_state.current_goal = HeroGoal::DestroyHeart;
                hero_state.target_pos = None; // Force pathfinding recalculation
                hero_state.target_room_id = None;
                hero_state.current_path = None;
                attackers_launched += 1;
            }
        }
    }

    state.hero_base.active_attackers = attackers_launched;

    trace_log!(
        "heroes",
        "Wave {} launched! {} heroes attacking the dungeon!",
        wave_number,
        attackers_launched
    );
}

/// Spawn heroes from buildings, respecting max_active limits
fn spawn_heroes_from_buildings(state: &mut GameState, game_data: &GameData, dt: f32) {
    let mut heroes_to_spawn = Vec::new();

    // First, count current heroes by type
    let mut hero_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (_, hero) in state.entities.heroes() {
        *hero_counts.entry(hero.hero_id.clone()).or_insert(0) += 1;
    }

    // Harder difficulty / higher mission threat replenishes the garrison faster;
    // razed spawn buildings slow it back down, permanently.
    let threat =
        state.effective_threat_multiplier(game_data) / state.hero_base.spawn_interval_multiplier();

    // Iterate over buildings to check timers
    for building in &mut state.hero_base.buildings {
        // Check if building is still alive (physical entity exists)
        let is_alive = if let Some(eid) = building.entity_id {
            state.entities.get(eid).is_some()
        } else {
            false
        };

        if !is_alive {
            continue; // Destroyed buildings don't spawn
        }

        for timer in &mut building.spawn_timers {
            timer.time_until_spawn -= dt * threat;

            if timer.time_until_spawn <= 0.0 {
                // Get current count for this hero type
                let current_count = hero_counts.get(&timer.hero_id).copied().unwrap_or(0);

                // Calculate scaling factors based on wave number
                let wave = state.hero_base.current_wave_number as f32;

                // Scale max_active linearly with waves: +1 slot per 2 waves, plus a small base multiplier
                let max_active_base =
                    if let Some(bd) = game_data.hero_buildings.get(&building.building_type) {
                        bd.spawn_triggers
                            .iter()
                            .find(|t| t.hero_id == timer.hero_id)
                            .map(|t| t.max_active)
                            .unwrap_or(10)
                    } else {
                        10
                    };

                // Formula: Base + Wave * scaling. Example: Wave 5 with base 10 -> 10 + 7.5 = 17 heroes
                let wave_scaling = game_data.config.hero_waves.wave_scaling_multiplier;
                let scaled_max_active = (max_active_base as f32 + (wave * wave_scaling)) as usize;

                // Scale spawn rate: reduce by 10% per wave, capped at minimum 5s
                let spawn_rate_base =
                    if let Some(bd) = game_data.hero_buildings.get(&building.building_type) {
                        bd.spawn_triggers
                            .iter()
                            .find(|t| t.hero_id == timer.hero_id)
                            .map(|t| t.spawn_rate_seconds)
                            .unwrap_or(60.0)
                    } else {
                        60.0
                    };

                // Formula: Rate * decay^Wave. Example: Wave 5 with 60s -> 60 * 0.59 = 35s
                let decay = game_data.config.hero_waves.spawn_rate_decay;
                let min_rate = game_data.config.hero_waves.min_spawn_rate;
                let scaling_factor = decay.powf(wave);
                let scaled_spawn_rate = (spawn_rate_base * scaling_factor).max(min_rate);

                timer.time_until_spawn = scaled_spawn_rate;

                // Only spawn if under max capacity
                if current_count < scaled_max_active {
                    heroes_to_spawn.push((timer.hero_id.clone(), building.pos));
                    // Update count for subsequent checks in this frame
                    *hero_counts.entry(timer.hero_id.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Spawn the heroes with defender/attacker assignment
    for (hero_id, pos) in heroes_to_spawn {
        spawn_hero_at(state, &hero_id, pos, game_data);
    }
}

fn spawn_hero_at(
    state: &mut GameState,
    hero_id: &str,
    building_pos: TilePos,
    game_data: &GameData,
) {
    if let Some(hero_data) = game_data.heroes.get(hero_id) {
        let spawn_pos = find_spawn_pos(state, building_pos, game_data);

        // Generate visual variation seed
        let visual_seed = macroquad_toolkit::rng::random_u64();
        let mut hero_state = HeroState::new(
            hero_id.to_string(),
            1, // Level 1
            hero_data.stats.health,
            hero_data.stats.mana,
            spawn_pos,
            hero_data.stats.dig_time,
            visual_seed,
        );

        // Determine if this hero should be a defender based on current ratio
        let (defender_count, attacker_count) = count_defender_attacker_ratio(state);
        let total_heroes = defender_count + attacker_count;

        // Assign as defender if we need more defenders to maintain ratio
        if total_heroes == 0 {
            // First hero is a defender
            hero_state.is_defender = true;
        } else {
            let current_defender_ratio = defender_count as f32 / total_heroes as f32;
            hero_state.is_defender =
                current_defender_ratio < game_data.config.hero_waves.defender_ratio;
        }

        state.entities.spawn_hero(spawn_pos, hero_state.clone());
        trace_log!(
            "heroes",
            "Spawned {} at {:?} ({})",
            hero_id,
            spawn_pos,
            if hero_state.is_defender {
                "defender"
            } else {
                "attacker"
            }
        );
    }
}

/// Count current defenders vs attackers
fn count_defender_attacker_ratio(state: &GameState) -> (usize, usize) {
    let mut defenders = 0;
    let mut attackers = 0;

    for (_, hero) in state.entities.heroes() {
        if hero.is_defender {
            defenders += 1;
        } else {
            attackers += 1;
        }
    }

    (defenders, attackers)
}

fn find_spawn_pos(state: &GameState, center: TilePos, game_data: &GameData) -> TilePos {
    let (w, h) = crate::engine::tile_grid::get_grid_dimensions(&state.dungeon.grid);
    let search_radius = game_data.config.hero_waves.spawn_search_radius;

    for r in 1..=search_radius {
        for dy in -r..=r {
            for dx in -r..=r {
                if let Some(pos) = try_spawn_position(state, game_data, center, dx, dy, r, w, h) {
                    return pos;
                }
            }
        }
    }

    center
}

fn try_spawn_position(
    state: &GameState,
    game_data: &GameData,
    center: TilePos,
    dx: i32,
    dy: i32,
    r: i32,
    w: usize,
    h: usize,
) -> Option<TilePos> {
    if dx * dx + dy * dy > r * r {
        return None;
    }

    let x = center.x + dx;
    let y = center.y + dy;

    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return None;
    }

    let pos = TilePos::new(x, y);

    if is_valid_spawn_tile(state, game_data, pos) {
        Some(pos)
    } else {
        None
    }
}

fn is_valid_spawn_tile(state: &GameState, game_data: &GameData, pos: TilePos) -> bool {
    let tile = match state.dungeon.get_tile(pos) {
        Some(t) => t,
        None => return false,
    };

    let blocks = game_data
        .tiles
        .get(&tile.tile_type)
        .map(|td| td.blocks_movement)
        .unwrap_or(false);

    !blocks && state.entities.at_position(pos).count() == 0
}

#[cfg(test)]
mod tests;
