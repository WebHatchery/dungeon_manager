use crate::data::GameData;
use crate::engine::room_validator::{self, Room};
use crate::state::dungeon::Dungeon;
use crate::state::tile_state::{Ownership, TilePos};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomManager {
    pub rooms: Vec<Room>,
    pub next_room_id: usize,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Vec::new(),
            next_room_id: 0,
        }
    }

    pub fn detect_and_update_rooms(&mut self, dungeon: &mut Dungeon, game_data: &GameData) {
        // Clear room_id from all tiles
        for row in &mut dungeon.grid {
            for tile in row {
                tile.room_id = None;
            }
        }

        self.rooms.clear();

        // Collect all detected rooms first (without mutating grid)
        let mut detected_rooms = Vec::new();

        // Detect rooms for each room type from game data
        let room_types: Vec<&String> = game_data.rooms.keys().collect();

        for room_type in room_types {
            // Use reused logic from room_validator
            let connected_rooms = room_validator::detect_rooms(&dungeon.grid, room_type);

            for room_tiles in connected_rooms {
                if let Some(room_data) = game_data.rooms.get(room_type) {
                    if let Err(_error) = room_validator::validate_room_shape(&room_tiles, room_data)
                    {
                        continue;
                    }

                    // Create room
                    let room_id = self.next_room_id;
                    self.next_room_id += 1;

                    let work_size = room_data.ai.work_size.unwrap_or([1, 1]); // Default to 1x1 if not specified (e.g. special rooms)
                    let work_slots = room_validator::calculate_work_slots(&room_tiles, work_size);

                    let mut room = Room::new(
                        room_id,
                        room_type.to_string(),
                        room_tiles.clone(),
                        work_slots,
                    );
                    room.active = true;

                    let quality = room_validator::calculate_room_quality(&room, room_data);
                    room.quality = quality;

                    // Calculate efficiency
                    room.efficiency =
                        room_validator::calculate_efficiency(&room, &dungeon.grid, game_data);

                    detected_rooms.push((room, room_tiles));
                }
            }
        }

        // Now apply room_ids to tiles (separate mutable pass)
        for (room, room_tiles) in detected_rooms {
            let room_id = room.id;

            for &pos in &room_tiles {
                if let Some(tile) = dungeon.get_tile_mut(pos) {
                    tile.room_id = Some(room_id);
                }
            }

            self.rooms.push(room);
        }
    }

    /// Generate food from hatcheries based on their size
    pub fn generate_food_from_hatcheries(&self, dt: f32, game_data: &GameData) -> f32 {
        let mut total_food_generated = 0.0;

        for room in &self.rooms {
            if room.room_type == "hatchery" && room.active {
                // Each hatchery tile generates 1 food per second * efficiency
                let productivity = crate::engine::room_validator::room_data_for(room, game_data)
                    .map(|data| {
                        crate::engine::room_validator::room_productivity_multiplier(room, data)
                    })
                    .unwrap_or(1.0);
                let food_rate = room.tiles.len() as f32 * room.efficiency * productivity;
                total_food_generated += food_rate * dt;
            }
        }
        total_food_generated
    }

    /// Count available (unclaimed) lair tiles
    pub fn count_available_lair_tiles(&self, dungeon: &Dungeon) -> usize {
        let mut count = 0;
        for row in &dungeon.grid {
            for tile in row {
                if tile.tile_type == "lair"
                    && tile.ownership == Ownership::Player
                    && tile.claimed_by_entity.is_none()
                {
                    count += 1;
                }
            }
        }
        count
    }

    /// Find an available lair tile and claim it for the given entity
    pub fn find_and_claim_lair_tile(
        &self,
        dungeon: &mut Dungeon,
        entity_id: crate::state::entities::EntityId,
    ) -> Option<TilePos> {
        // Find first available lair tile
        let mut claimed_pos = None;
        for row in &mut dungeon.grid {
            for tile in row {
                if tile.tile_type == "lair"
                    && tile.ownership == Ownership::Player
                    && tile.claimed_by_entity.is_none()
                {
                    tile.claimed_by_entity = Some(entity_id);
                    claimed_pos = Some(tile.pos);
                    break;
                }
            }
            if claimed_pos.is_some() {
                break;
            }
        }
        claimed_pos
    }

    /// Release lair tile claimed by an entity (when creature dies/leaves)
    pub fn release_lair_tile(
        &self,
        dungeon: &mut Dungeon,
        entity_id: crate::state::entities::EntityId,
    ) {
        for row in &mut dungeon.grid {
            for tile in row {
                if tile.claimed_by_entity == Some(entity_id) {
                    tile.claimed_by_entity = None;
                    eprintln!("Released lair tile at {:?}", tile.pos);
                    return;
                }
            }
        }
    }

    pub fn get_room_at(&self, pos: TilePos) -> Option<&Room> {
        self.rooms
            .iter()
            .find(|&room| room.tiles.contains(&pos))
            .map(|v| v as _)
    }
}
