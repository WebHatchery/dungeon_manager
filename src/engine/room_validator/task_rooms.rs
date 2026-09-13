//! Room lookup by authored task family.

use super::{creature_can_enter_room, room_data_for, Room};
use crate::data::GameData;
use crate::state::tile_state::TilePos;

/// Find the nearest active room whose data declares `ai.task_type`.
pub fn find_nearest_room_for_task(
    rooms: &[Room],
    task_type: &str,
    pos: TilePos,
    game_data: &GameData,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for room in rooms.iter().filter(|room| room.active) {
        if !room_data_for(room, game_data).is_some_and(|data| data.ai.task_type == task_type) {
            continue;
        }
        consider_closest(&mut best, room, pos);
    }
    best
}

/// Task-family lookup with the same authored creature entry restrictions as
/// the room-type lookup.
pub fn find_nearest_room_for_task_and_creature(
    rooms: &[Room],
    task_type: &str,
    pos: TilePos,
    creature_id: &str,
    creature_level: u32,
    creature_mood: f32,
    game_data: &GameData,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for room in rooms {
        if !room.active
            || !room_data_for(room, game_data).is_some_and(|data| data.ai.task_type == task_type)
            || !creature_can_enter_room(room, creature_id, creature_level, creature_mood, game_data)
        {
            continue;
        }
        consider_closest(&mut best, room, pos);
    }
    best
}

fn consider_closest(best: &mut Option<(usize, f32)>, room: &Room, pos: TilePos) {
    let center = room.get_center();
    let dx = (center.x - pos.x) as f32;
    let dy = (center.y - pos.y) as f32;
    let distance = (dx * dx + dy * dy).sqrt();
    if best.is_none_or(|(_, best_distance)| distance < best_distance) {
        *best = Some((room.id, distance));
    }
}
