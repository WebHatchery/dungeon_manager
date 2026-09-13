//! Creature path construction and the adjacent-tile fallback for blocked goals.

use crate::data::GameData;
use crate::engine::creature_targets::pick_wander_position;
use crate::engine::pathfinding::{find_path, Heuristic, PathfindingGrid, Pos};
use crate::engine::tile_types;
use crate::state::dungeon::Dungeon;
use crate::state::entities::{EntityId, EntityManager, Task};
use crate::state::room_manager::RoomManager;
use crate::state::tile_state::TilePos;

pub(super) fn pathfind_to_target(
    creature_id: EntityId,
    current_pos: TilePos,
    mut target_pos: Option<TilePos>,
    dungeon: &Dungeon,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
) {
    if target_pos.is_none() {
        let is_idle = entities
            .get(creature_id)
            .and_then(|entity| entity.as_creature())
            .is_some_and(|creature| matches!(creature.current_task, Some(Task::Idle)));
        if is_idle {
            target_pos = pick_wander_position(dungeon, current_pos, room_manager, game_data);
        }
    }

    let Some(target) = target_pos else {
        return;
    };
    if current_pos == target {
        return;
    }

    let mut pf_grid = PathfindingGrid::new(dungeon.width, dungeon.height);
    for y in 0..dungeon.height {
        for x in 0..dungeon.width {
            let tile_pos = TilePos::new(x as i32, y as i32);
            if let Some(tile) = dungeon.get_tile(tile_pos) {
                pf_grid.set_walkable(
                    Pos::new(x as i32, y as i32),
                    tile_types::is_tile_walkable(tile, game_data),
                );
            }
        }
    }

    let start = Pos::new(current_pos.x, current_pos.y);
    let goal = Pos::new(target.x, target.y);
    let mut path_result = find_path(start, goal, &pf_grid, Heuristic::Manhattan, false);

    // A creature targeting a room tile stops beside it when the target itself
    // is a wall or structure rather than failing the entire task.
    if path_result.is_none() && !pf_grid.is_walkable(goal) {
        let mut neighbors = Vec::new();
        for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let neighbor = Pos::new(target.x + dx, target.y + dy);
            if neighbor.x >= 0
                && neighbor.x < dungeon.width as i32
                && neighbor.y >= 0
                && neighbor.y < dungeon.height as i32
                && pf_grid.is_walkable(neighbor)
            {
                neighbors.push(neighbor);
            }
        }
        neighbors.sort_by_key(|pos| (pos.x - start.x).abs() + (pos.y - start.y).abs());
        for neighbor in neighbors {
            if let Some(path) = find_path(start, neighbor, &pf_grid, Heuristic::Manhattan, false) {
                path_result = Some(path);
                break;
            }
        }
    }

    if let Some(path) = path_result {
        if let Some(creature) = entities
            .get_mut(creature_id)
            .and_then(|entity| entity.as_creature_mut())
        {
            creature.current_path = Some(
                path.waypoints
                    .iter()
                    .map(|pos| TilePos::new(pos.x, pos.y))
                    .collect(),
            );
        }
    }
}
