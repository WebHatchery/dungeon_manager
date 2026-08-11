use super::*;

/// Process claiming an unclaimed tile
pub(super) fn process_claiming(
    dungeon: &mut Dungeon,
    entities: &mut EntityManager,
    player: &mut PlayerState,
    imp_id: EntityId,
    target_pos: TilePos,
    dt: f32,
    _game_data: &GameData,
) {
    let claim_delay = 2.0; // Hardcoded delay for now, could be in config
    let mut task_complete = false;

    if let Some(entity) = entities.get_mut(imp_id) {
        if let Some(creature) = entity.as_creature_mut() {
            // Ensure task is set
            if creature.current_task.is_none() {
                creature.current_task = Some(crate::state::entities::Task::ClaimTile(target_pos));
            }

            creature.task_time += dt;
            if creature.task_time >= claim_delay {
                creature.task_time = 0.0;
                creature.current_task = None;
                task_complete = true;
            }
        }
    }

    if task_complete {
        if let Some(tile) = dungeon.get_tile_mut(target_pos) {
            // Verify it differs from current ownership
            if tile.ownership == Ownership::Unclaimed {
                tile.ownership = Ownership::Player;
                tile.tile_type = tt::CLAIMED_FLOOR.to_string();
                player.claimed_tile_count += 1;
                trace_log!("imps", "Imp claimed tile at {:?}", target_pos);
            }
        }
    }
}

pub(super) fn find_nearest_claimable_tile(
    dungeon: &Dungeon,
    imp_pos: TilePos,
    targeted_tiles: &HashSet<TilePos>,
    player: &PlayerState,
    game_data: &GameData,
) -> Option<TilePos> {
    let mut nearest = None;
    let mut min_dist = f32::MAX;

    for y in 0..dungeon.height {
        for x in 0..dungeon.width {
            let pos = TilePos::new(x as i32, y as i32);
            if let Some(dist) =
                evaluate_claim_target(dungeon, pos, imp_pos, targeted_tiles, player, game_data)
            {
                if dist < min_dist {
                    min_dist = dist;
                    nearest = Some(pos);
                }
            }
        }
    }
    nearest
}

pub(super) fn evaluate_claim_target(
    dungeon: &Dungeon,
    pos: TilePos,
    imp_pos: TilePos,
    targeted_tiles: &HashSet<TilePos>,
    _player: &PlayerState,
    game_data: &GameData,
) -> Option<f32> {
    if pos != imp_pos && targeted_tiles.contains(&pos) {
        return None;
    }

    let tile = dungeon.get_tile(pos)?;

    // Must be Unclaimed floor
    if tile.ownership != Ownership::Unclaimed {
        return None;
    }

    if tile.tile_type != tt::FLOOR && !tile_types::is_claimable(&tile.tile_type, game_data) {
        return None;
    }

    // Must be adjacent to Player owned tile (flood fill style expansion)
    let neighbors = crate::engine::tile_grid::get_cardinal_neighbors(&dungeon.grid, pos);
    let is_adjacent_to_owned = neighbors.iter().any(|n| {
        dungeon
            .get_tile(*n)
            .map(|t| t.ownership == Ownership::Player)
            .unwrap_or(false)
    });

    if !is_adjacent_to_owned {
        return None;
    }

    let dist_sq =
        Pos::new(pos.x, pos.y).euclidean_distance_squared(&Pos::new(imp_pos.x, imp_pos.y));
    Some(dist_sq)
}
