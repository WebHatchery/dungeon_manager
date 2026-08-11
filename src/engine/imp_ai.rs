//! Imp AI - Digging and wandering behavior
//! Stateless service for imp-specific behavior

mod claiming;

use claiming::{find_nearest_claimable_tile, process_claiming};

use crate::data::GameData;
use crate::engine::pathfinding::{find_path, Heuristic, PathfindingGrid, Pos};
use crate::engine::tile_types::{self, types as tt};
use crate::state::dungeon::Dungeon;
use crate::state::entities::{EntityId, EntityManager};
use crate::state::player_state::PlayerState;
use crate::state::tile_state::{Ownership, TilePos};
use std::collections::HashSet;

/// Update all imp digging and movement behavior
pub fn update_imp_digging(
    dungeon: &mut Dungeon,
    entities: &mut EntityManager,
    player: &mut PlayerState,
    game_data: &GameData,
    dt: f32,
) {
    // Get all imp IDs
    let mut imp_ids: Vec<EntityId> = entities
        .creatures()
        .filter(|(_, creature)| creature.creature_id == "imp" && creature.health > 0.0)
        .map(|(id, _)| id)
        .collect();

    // Sort by ID for deterministic behavior to prevent task flapping
    imp_ids.sort();

    // Track tiles being targeted by other imps to prevent multiple imps on one tile
    let mut targeted_tiles = collect_targeted_tiles(entities, &imp_ids);

    for imp_id in imp_ids {
        // Get imp position
        let imp_pos = match entities.get(imp_id) {
            Some(e) => e.pos,
            None => continue,
        };

        // Check if imp has a path
        let should_check_dig = {
            let entity = entities.get(imp_id).unwrap();
            let creature = entity.as_creature().unwrap();
            creature.current_path.is_none()
        };

        // Remove THIS imp's targets from the exclusion set so it can 'find' its own job
        if let Some(entity) = entities.get(imp_id) {
            if let Some(creature) = entity.as_creature() {
                if let Some(task) = &creature.current_task {
                    match task {
                        crate::state::entities::Task::Dig(pos) => {
                            targeted_tiles.remove(pos);
                        }
                        crate::state::entities::Task::ClaimTile(pos) => {
                            targeted_tiles.remove(pos);
                        }
                        _ => {}
                    }
                }
                if let Some(path) = &creature.current_path {
                    if let Some(last_pos) = path.last() {
                        targeted_tiles.remove(last_pos);
                    }
                }
            }
            targeted_tiles.remove(&imp_pos);
        }

        // If imp has no path, find nearest marked tile
        if should_check_dig {
            process_idle_imp(
                dungeon,
                entities,
                player,
                imp_id,
                imp_pos,
                &targeted_tiles,
                game_data,
                dt,
            );
        }

        // Re-add potentially excluded target for next iterations
        update_targeted_tiles(entities, imp_id, imp_pos, &mut targeted_tiles);

        // Handle movement along path
        crate::engine::movement::process_entity_movement(entities, imp_id, dt, 1.0);
    }
}

/// Collect all tiles currently targeted by imps
fn collect_targeted_tiles(entities: &EntityManager, imp_ids: &[EntityId]) -> HashSet<TilePos> {
    let mut targeted_tiles = HashSet::new();
    for &other_id in imp_ids {
        add_imp_targets(entities, other_id, &mut targeted_tiles);
    }
    targeted_tiles
}

fn add_imp_targets(
    entities: &EntityManager,
    imp_id: EntityId,
    targeted_tiles: &mut HashSet<TilePos>,
) {
    let entity = match entities.get(imp_id) {
        Some(e) => e,
        None => return,
    };
    let creature = match entity.as_creature() {
        Some(c) => c,
        None => return,
    };

    if let Some(task) = &creature.current_task {
        match task {
            crate::state::entities::Task::Dig(pos) => {
                targeted_tiles.insert(*pos);
            }
            crate::state::entities::Task::ClaimTile(pos) => {
                targeted_tiles.insert(*pos);
            }
            _ => {}
        }
    }

    if let Some(path) = &creature.current_path {
        if let Some(last_pos) = path.last() {
            targeted_tiles.insert(*last_pos);
        }
    }

    if creature.current_path.is_none() && creature.current_task.is_none() {
        targeted_tiles.insert(entity.pos);
    }
}

/// Update the targeted tiles set for next iterations
fn update_targeted_tiles(
    entities: &EntityManager,
    imp_id: EntityId,
    imp_pos: TilePos,
    targeted_tiles: &mut HashSet<TilePos>,
) {
    targeted_tiles.insert(imp_pos);
    if let Some(entity) = entities.get(imp_id) {
        if let Some(creature) = entity.as_creature() {
            if let Some(path) = &creature.current_path {
                if let Some(last_pos) = path.last() {
                    targeted_tiles.insert(*last_pos);
                }
            }
        }
    }
}

/// Process an idle imp - find work or wander
fn process_idle_imp(
    dungeon: &mut Dungeon,
    entities: &mut EntityManager,
    player: &mut PlayerState,
    imp_id: EntityId,
    imp_pos: TilePos,
    targeted_tiles: &HashSet<TilePos>,
    game_data: &GameData,
    dt: f32,
) {
    // 1. Check existing task first
    let mut current_task = None;
    if let Some(entity) = entities.get(imp_id) {
        if let Some(creature) = entity.as_creature() {
            current_task = creature.current_task.clone();
        }
    }

    if let Some(task) = current_task {
        match task {
            crate::state::entities::Task::PickupResource(id) => {
                process_pickup(dungeon, entities, player, imp_id, id);
                return;
            }
            crate::state::entities::Task::ClaimTile(pos) => {
                if imp_pos == pos {
                    process_claiming(dungeon, entities, player, imp_id, pos, dt, game_data);
                } else {
                    pathfind_to_target(dungeon, entities, imp_id, imp_pos, pos, false);
                }
                return;
            }
            _ => {}
        }
    }

    // 2. Priority 1: Pickup Gold (if space)
    if let Some(pile_id) = find_nearest_resource_pile(entities, imp_pos, player) {
        if let Some(entity) = entities.get_mut(imp_id) {
            if let Some(creature) = entity.as_creature_mut() {
                creature.current_task = Some(crate::state::entities::Task::PickupResource(pile_id));
            }
        }
        process_pickup(dungeon, entities, player, imp_id, pile_id);
        return;
    }

    // 3. Priority 2: Work (Digging or Claiming - closest first)
    let nearest_dig = find_nearest_marked_tile(dungeon, imp_pos, targeted_tiles, player, game_data);
    let nearest_claim =
        find_nearest_claimable_tile(dungeon, imp_pos, targeted_tiles, player, game_data);

    let chosen_work = match (nearest_dig, nearest_claim) {
        (Some(dig_pos), Some(claim_pos)) => {
            let dig_dist = imp_pos.manhattan_distance(&dig_pos);
            let claim_dist = imp_pos.manhattan_distance(&claim_pos);
            if dig_dist <= claim_dist {
                Some((dig_pos, true)) // true = dig
            } else {
                Some((claim_pos, false)) // false = claim
            }
        }
        (Some(dig_pos), None) => Some((dig_pos, true)),
        (None, Some(claim_pos)) => Some((claim_pos, false)),
        (None, None) => None,
    };

    if let Some((target_pos, is_digging)) = chosen_work {
        if is_digging {
            // Process Digging
            if let Some(entity) = entities.get_mut(imp_id) {
                if let Some(creature) = entity.as_creature_mut() {
                    creature.current_task = Some(crate::state::entities::Task::Dig(target_pos));
                }
            }
            if imp_pos.manhattan_distance(&target_pos) <= 1 {
                process_digging(dungeon, entities, player, imp_id, target_pos, dt, game_data);
            } else {
                if let Some(entity) = entities.get_mut(imp_id) {
                    if let Some(creature) = entity.as_creature_mut() {
                        creature.task_time = 0.0;
                    }
                }
                // Determine best standing spot
                let neighbors =
                    crate::engine::tile_grid::get_cardinal_neighbors(&dungeon.grid, target_pos);
                let mut best_target = None;
                let mut min_dist = f32::MAX;
                for target in neighbors {
                    if let Some(tile) = dungeon.get_tile(target) {
                        if tile_types::is_walkable(&tile.tile_type, game_data) {
                            let d = imp_pos.distance_to(&target);
                            if d < min_dist {
                                min_dist = d;
                                best_target = Some(target);
                            }
                        }
                    }
                }
                if let Some(move_target) = best_target {
                    pathfind_to_target(dungeon, entities, imp_id, imp_pos, move_target, false);
                } else {
                    // accessible spot not found, try to path directly to it (maybe adjacent is not walkable but we are next to it?)
                    if let Some(entity) = entities.get_mut(imp_id) {
                        if let Some(creature) = entity.as_creature_mut() {
                            creature.current_task = None;
                        }
                    }
                    wander_randomly(dungeon, entities, imp_id, imp_pos, game_data);
                }
            }
        } else {
            // Process Claiming
            if let Some(entity) = entities.get_mut(imp_id) {
                if let Some(creature) = entity.as_creature_mut() {
                    creature.current_task =
                        Some(crate::state::entities::Task::ClaimTile(target_pos));
                }
            }
            if imp_pos == target_pos {
                process_claiming(dungeon, entities, player, imp_id, target_pos, dt, game_data);
            } else {
                if let Some(entity) = entities.get_mut(imp_id) {
                    if let Some(creature) = entity.as_creature_mut() {
                        creature.task_time = 0.0;
                    }
                }
                pathfind_to_target(dungeon, entities, imp_id, imp_pos, target_pos, false);
            }
        }
        return;
    }

    // 5. Wander
    wander_randomly(dungeon, entities, imp_id, imp_pos, game_data);
}

fn process_pickup(
    dungeon: &Dungeon,
    entities: &mut EntityManager,
    player: &mut PlayerState,
    imp_id: EntityId,
    target_id: EntityId,
) {
    // Check if target exists
    let pile_pos = if let Some(entity) = entities.get(target_id) {
        if let crate::state::entities::EntityType::ResourcePile(_) = entity.entity_type {
            entity.pos
        } else {
            // Invalid target (not a pile)
            if let Some(imp_entity) = entities.get_mut(imp_id) {
                if let Some(creature) = imp_entity.as_creature_mut() {
                    creature.current_task = None;
                }
            }
            return;
        }
    } else {
        // Pile gone
        if let Some(imp_entity) = entities.get_mut(imp_id) {
            if let Some(creature) = imp_entity.as_creature_mut() {
                creature.current_task = None;
            }
        }
        return;
    };

    let imp_pos = entities.get(imp_id).map(|e| e.pos).unwrap();

    if imp_pos == pile_pos {
        // Collect
        let amount_collected = if let Some(entity) = entities.get_mut(target_id) {
            if let crate::state::entities::EntityType::ResourcePile(ref mut state) =
                &mut entity.entity_type
            {
                let space = player.max_gold - player.gold;
                let to_take = state.amount.min(space);
                state.amount -= to_take;
                to_take
            } else {
                0
            }
        } else {
            0
        };

        if amount_collected > 0 {
            player.add_resources(amount_collected, 0, 0, 0);
            trace_log!("imps", "Imp picked up {} gold.", amount_collected);
        }

        // Check if pile empty
        let remove = if let Some(entity) = entities.get(target_id) {
            if let crate::state::entities::EntityType::ResourcePile(state) = &entity.entity_type {
                state.amount <= 0
            } else {
                false
            }
        } else {
            false
        };

        if remove {
            entities.remove(target_id);
        }

        // Task done
        if let Some(imp_entity) = entities.get_mut(imp_id) {
            if let Some(creature) = imp_entity.as_creature_mut() {
                creature.current_task = None;
            }
        }
    } else {
        // Move to it
        pathfind_to_target(dungeon, entities, imp_id, imp_pos, pile_pos, true);
    }
}

/// Find nearest resource pile
fn find_nearest_resource_pile(
    entities: &EntityManager,
    imp_pos: TilePos,
    player: &PlayerState,
) -> Option<EntityId> {
    if player.gold >= player.max_gold {
        return None;
    }

    let mut nearest = None;
    let mut min_dist = f32::MAX;

    for (id, entity) in entities.entities() {
        if let crate::state::entities::EntityType::ResourcePile(pile) = &entity.entity_type {
            if pile.resource_type == "gold" {
                let dist = imp_pos.distance_to(&entity.pos);
                if dist < min_dist {
                    min_dist = dist;
                    nearest = Some(*id);
                }
            }
        }
    }
    nearest
}

/// Find the nearest marked tile not targeted by other imps
fn find_nearest_marked_tile(
    dungeon: &Dungeon,
    imp_pos: TilePos,
    targeted_tiles: &HashSet<TilePos>,
    player: &PlayerState,
    game_data: &GameData,
) -> Option<TilePos> {
    let mut nearest_marked = None;
    let mut min_dist = f32::MAX;

    for y in 0..dungeon.height {
        for x in 0..dungeon.width {
            let pos = TilePos::new(x as i32, y as i32);
            if let Some(dist) =
                evaluate_dig_target(dungeon, pos, imp_pos, targeted_tiles, player, game_data)
            {
                if dist < min_dist {
                    min_dist = dist;
                    nearest_marked = Some(pos);
                }
            }
        }
    }
    nearest_marked
}

/// Evaluate a potential dig target, returns distance score if valid
fn evaluate_dig_target(
    dungeon: &Dungeon,
    pos: TilePos,
    imp_pos: TilePos,
    targeted_tiles: &HashSet<TilePos>,
    player: &PlayerState,
    game_data: &GameData,
) -> Option<f32> {
    if pos != imp_pos && targeted_tiles.contains(&pos) {
        return None;
    }

    let tile = dungeon.get_tile(pos)?;
    if !tile.marked_for_dig {
        return None;
    }

    let dist_sq =
        Pos::new(pos.x, pos.y).euclidean_distance_squared(&Pos::new(imp_pos.x, imp_pos.y));

    if tile.tile_type == tt::GEM_SEAM {
        if player.gold >= player.max_gold {
            return None;
        }
        // Prioritize gem seams by adding a large bonus to distance (making them less preferred for immediate selection)
        // Actually this is inverted - we want lower score for closer/better tiles
        // The original code adds 10000 to deprioritize gem seams unless nothing else is available
        return Some(dist_sq + game_data.config.imp_behavior.gem_priority_bonus);
    }

    Some(dist_sq)
}

/// Process digging at a marked tile
fn process_digging(
    dungeon: &mut Dungeon,
    entities: &mut EntityManager,
    player: &mut PlayerState,
    imp_id: EntityId,
    marked_pos: TilePos,
    dt: f32,
    game_data: &GameData,
) {
    // Check if dig delay is complete
    let mut task_complete = false;
    let dig_delay = game_data.config.imp_behavior.dig_completion_delay;
    if let Some(entity) = entities.get_mut(imp_id) {
        if let Some(creature) = entity.as_creature_mut() {
            // Set task if not set (ensures we "claim" the tile)
            if creature.current_task.is_none() {
                creature.current_task = Some(crate::state::entities::Task::Dig(marked_pos));
            }

            creature.task_time += dt;
            if creature.task_time >= dig_delay {
                creature.task_time = 0.0;
                creature.current_task = None; // Task done
                task_complete = true;
            }
        }
    }

    if task_complete {
        complete_dig(dungeon, Some(entities), player, marked_pos, game_data);
    }
}

/// What one dig of this tile type yields.
///
/// Read from the tile's own `resources.mine_value` in `tiles.json`, falling
/// back to the `imp_behavior.*_reward` config constant for tiles that do not
/// declare one. The gem seam authored `mine_value: 25` while the config
/// authored `gem_seam_reward: 25` — the same number in two files, which is the
/// duplication behind every drift found in this codebase so far. The tile's
/// own number wins, so a richer seam becomes a data edit instead of a
/// second global constant.
fn mine_yield_per_dig(tile_type: &str, game_data: &GameData) -> i32 {
    let config = &game_data.config.imp_behavior;
    let fallback = match tile_type {
        x if x == tt::GOLD_VEIN => config.gold_vein_reward,
        x if x == tt::GEM_SEAM => config.gem_seam_reward,
        x if x == tt::MANA_CRYSTAL => config.mana_crystal_reward,
        _ => 0,
    };

    game_data
        .tiles
        .get(tile_type)
        .and_then(|tile| tile.resources.as_ref())
        .and_then(|resources| resources.mine_value)
        .unwrap_or(fallback)
}

/// Complete digging a tile and award resources
pub(crate) fn complete_dig(
    dungeon: &mut Dungeon,
    entities: Option<&mut EntityManager>,
    player: &mut PlayerState,
    marked_pos: TilePos,
    game_data: &GameData,
) {
    if let Some(tile) = dungeon.get_tile_mut(marked_pos) {
        if !tile.marked_for_dig {
            return;
        }

        // Per-dig yield comes from the tile's own data where it declares one.
        let per_dig = mine_yield_per_dig(&tile.tile_type, game_data);

        // Check if tile has resources
        let (gold_gained, mana_gained, is_gem_seam) = match tile.tile_type.as_str() {
            x if x == tt::GOLD_VEIN => {
                let gold = tile
                    .resources_remaining
                    .map_or(per_dig, |r| per_dig.min(r as i32));
                (gold, 0, false)
            }
            // Infinite: the seam is never consumed, which is what makes gems
            // the slow-but-endless gold source rather than a richer vein.
            x if x == tt::GEM_SEAM => (per_dig, 0, true),
            x if x == tt::MANA_CRYSTAL => {
                let mana = tile
                    .resources_remaining
                    .map_or(per_dig, |r| per_dig.min(r as i32));
                (0, mana, false)
            }
            _ => (0, 0, false),
        };

        if is_gem_seam {
            // Gem seams stay marked - imps will keep mining them continuously
            // Do NOT unmark the tile, do NOT convert to floor
            player.add_resources(gold_gained, 0, 0, 0);
            trace_log!(
                "imps",
                "Imp mined gem seam at {:?}, gained {} gold",
                marked_pos,
                gold_gained
            );
            return; // Exit early to preserve tile state
        }

        // Safety check: if gem seam and gold full, do nothing (should be handled by target selection but good for safety)
        if is_gem_seam && player.gold >= player.max_gold {
            return;
        }

        // For non-infinite resources:
        // Convert to unclaimed floor. Traced unconditionally, because plain
        // earth yields neither gold nor mana and the yield-specific traces
        // below skip it — which made the single most common imp action
        // invisible even with `DUNGEON_MANAGER_LOG=imps`.
        trace_log!(
            "imps",
            "Imp finished digging {:?} at {:?}",
            tile.tile_type,
            marked_pos
        );
        tile.tile_type = tt::FLOOR.to_string();
        tile.ownership = Ownership::Unclaimed;
        tile.marked_for_dig = false;
        tile.resources_remaining = None;
        tile.room_id = None;
        // Tile is now unclaimed floor, imps must claim it separately

        // Give resources to player
        if gold_gained > 0 {
            let space = player.max_gold - player.gold;
            if space < gold_gained {
                // Fill remaining space
                if space > 0 {
                    player.add_resources(space, 0, 0, 0);
                }

                // Spawn pile for excess
                let excess = gold_gained - space;
                if let Some(entities_mgr) = entities {
                    let pile_state =
                        crate::state::entities::ResourcePileState::new("gold".to_string(), excess);
                    entities_mgr.spawn_resource_pile(marked_pos, pile_state);
                    player.warn_once(
                        "treasury_full",
                        "Treasury full — gold is piling up on the floor. Build more storage.",
                    );
                } else {
                    eprintln!(
                        "Warning: No EntityManager passed to complete_dig, lost {} gold",
                        excess
                    );
                }
            } else {
                player.add_resources(gold_gained, 0, 0, 0);
                // The vault swallowed the whole haul, so the next overflow is
                // news again.
                player.clear_warning("treasury_full");
            }
        } else if mana_gained > 0 {
            player.mana = (player.mana + mana_gained).min(player.max_mana);
            trace_log!(
                "imps",
                "Imp mined mana crystal at {:?}, gained {} mana",
                marked_pos,
                mana_gained
            );
        }
    }
}

/// Pathfind to a target position
fn pathfind_to_target(
    dungeon: &Dungeon,
    entities: &mut EntityManager,
    imp_id: EntityId,
    imp_pos: TilePos,
    target_pos: TilePos,
    include_marked: bool,
) {
    let mut pf_grid = PathfindingGrid::new(dungeon.width, dungeon.height);

    // Mark walkable tiles
    for y in 0..dungeon.height {
        for x in 0..dungeon.width {
            let tile_pos = TilePos::new(x as i32, y as i32);
            if let Some(tile) = dungeon.get_tile(tile_pos) {
                let walkable = if include_marked {
                    tile.ownership == Ownership::Player
                        || tile.marked_for_dig
                        || (tile.ownership == Ownership::Unclaimed && tile.tile_type == tt::FLOOR)
                } else {
                    tile.ownership == Ownership::Player
                        || (tile.ownership == Ownership::Unclaimed && tile.tile_type == tt::FLOOR)
                };
                let pf_pos = Pos::new(x as i32, y as i32);
                pf_grid.set_walkable(pf_pos, walkable);
            }
        }
    }

    // Find path
    let start = Pos::new(imp_pos.x, imp_pos.y);
    let goal = Pos::new(target_pos.x, target_pos.y);

    if let Some(path) = find_path(start, goal, &pf_grid, Heuristic::Manhattan, false) {
        let waypoints: Vec<TilePos> = path
            .waypoints
            .into_iter()
            .map(|p| TilePos::new(p.x, p.y))
            .collect();

        if let Some(entity) = entities.get_mut(imp_id) {
            if let Some(creature) = entity.as_creature_mut() {
                creature.current_path = Some(waypoints);
            }
        }
    }
}

/// Pick a random walkable tile and wander to it
fn wander_randomly(
    dungeon: &Dungeon,
    entities: &mut EntityManager,
    imp_id: EntityId,
    imp_pos: TilePos,
    game_data: &GameData,
) {
    let wander_radius = game_data.config.creature_ai.wander_radius;
    let wander_attempts = game_data.config.creature_ai.wander_attempts;
    let mut attempts = 0;
    let mut wander_pos = None;

    while attempts < wander_attempts && wander_pos.is_none() {
        let dx = macroquad_toolkit::rng::gen_range(-wander_radius, wander_radius + 1);
        let dy = macroquad_toolkit::rng::gen_range(-wander_radius, wander_radius + 1);
        let candidate = TilePos::new(imp_pos.x + dx, imp_pos.y + dy);

        if let Some(tile) = dungeon.get_tile(candidate) {
            if tile_types::is_walkable(&tile.tile_type, game_data) {
                wander_pos = Some(candidate);
            }
        }
        attempts += 1;
    }

    if let Some(target_pos) = wander_pos {
        if imp_pos != target_pos {
            pathfind_to_target(dungeon, entities, imp_id, imp_pos, target_pos, false);
        }
    }
}

// process_imp_movement removed in favor of shared movement::process_entity_movement
