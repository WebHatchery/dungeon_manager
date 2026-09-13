use crate::data::GameData;
use crate::state::dungeon::Dungeon;
use crate::state::entities::{EntityId, EntityManager};
use crate::state::tile_state::TilePos;

/// Shared movement logic for entities following a path/timer system.
///
/// `speed_multiplier` scales the entity's own speed for effects the mover
/// itself knows nothing about — heroes slow down when their stables burn, and
/// this function has no business knowing that, so the caller supplies it.
pub fn process_entity_movement(
    entities: &mut EntityManager,
    entity_id: EntityId,
    dungeon: &Dungeon,
    game_data: &GameData,
    dt: f32,
    speed_multiplier: f32,
) -> Option<TilePos> {
    let movement_data = if let Some(entity) = entities.get(entity_id) {
        match &entity.entity_type {
            crate::state::entities::EntityType::Creature(c) => {
                Some((c.current_path.clone(), c.move_timer, c.movement_speed))
            }
            crate::state::entities::EntityType::Hero(h) => {
                Some((h.current_path.clone(), h.move_timer, h.movement_speed))
            }
            _ => None,
        }
    } else {
        None
    };

    let (current_path, move_timer, movement_speed) = movement_data?;
    let terrain_multiplier = entities
        .get(entity_id)
        .map(|entity| terrain_speed_multiplier(entity.pos, dungeon, game_data))
        .unwrap_or(1.0);

    let mut should_move = false;
    let mut next_waypoint = None;
    let mut new_move_timer = move_timer;

    if let Some(path) = current_path {
        if !path.is_empty() {
            new_move_timer += dt;
            let move_interval =
                1.0 / (movement_speed * speed_multiplier * terrain_multiplier).max(0.01);

            if new_move_timer >= move_interval {
                new_move_timer = 0.0;
                should_move = true;
                next_waypoint = path.first().copied();
            }
        }
    }

    if should_move {
        if let Some(next_pos) = next_waypoint {
            if let Some(entity) = entities.get_mut(entity_id) {
                // Update position and state
                entity.pos = next_pos;

                match &mut entity.entity_type {
                    crate::state::entities::EntityType::Creature(c) => {
                        c.move_timer = 0.0;
                        if let Some(ref mut path) = c.current_path {
                            if !path.is_empty() {
                                path.remove(0);
                            }
                            if path.is_empty() {
                                c.current_path = None;
                            }
                        }
                    }
                    crate::state::entities::EntityType::Hero(h) => {
                        h.move_timer = 0.0;
                        if let Some(ref mut path) = h.current_path {
                            if !path.is_empty() {
                                path.remove(0);
                            }
                            if path.is_empty() {
                                h.current_path = None;
                            }
                        }
                    }
                    _ => {}
                }
                return Some(next_pos);
            }
        }
    } else {
        // Update timer only
        if let Some(entity) = entities.get_mut(entity_id) {
            match &mut entity.entity_type {
                crate::state::entities::EntityType::Creature(c) => c.move_timer = new_move_timer, // Use calculated new timer
                crate::state::entities::EntityType::Hero(h) => h.move_timer = new_move_timer,
                _ => {}
            }
        }
    }

    None
}

/// Resolve the authored movement effect of the tile an entity currently
/// occupies. Blocking hazards still normally prevent pathing, but scripted
/// movement and a newly changed tile can leave an entity standing on one.
pub fn terrain_speed_multiplier(pos: TilePos, dungeon: &Dungeon, game_data: &GameData) -> f32 {
    dungeon
        .get_tile(pos)
        .and_then(|tile| game_data.tiles.get(&tile.tile_type))
        .and_then(|tile| tile.speed_modifier)
        .unwrap_or(1.0)
        .max(0.05)
}
