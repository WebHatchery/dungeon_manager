//! Tile type constants and helper functions
//! Centralizes all tile type string literals to avoid magic strings

/// Tile type string constants
pub mod types {
    // Resource tiles
    pub const GOLD_VEIN: &str = "gold_vein";
    pub const GEM_SEAM: &str = "gem_seam";
    pub const MANA_CRYSTAL: &str = "mana_crystal";

    // Floor tiles
    pub const FLOOR: &str = "floor";
    pub const CLAIMED_FLOOR: &str = "claimed_floor";
    pub const REINFORCED_WALL: &str = "reinforced_wall";
    pub const BRIDGE: &str = "bridge";

    // Special tiles
    pub const DUNGEON_HEART: &str = "dungeon_heart";
    pub const MONSTER_SPAWNER: &str = "monster_spawner";
}

use crate::data::GameData;
use crate::state::tile_state::TileState;

/// Check if a tile type is a wall/blocking tile
pub fn is_wall(tile_type: &str, game_data: &GameData) -> bool {
    // Treat reinforced wall as wall, plus anything that blocks vision (rocks, earth, resources)
    tile_type == types::REINFORCED_WALL
        || game_data
            .tiles
            .get(tile_type)
            .map(|t| t.blocks_vision)
            .unwrap_or(false)
}

/// Check if a tile type is diggable
pub fn is_diggable(tile_type: &str, game_data: &GameData) -> bool {
    game_data
        .tiles
        .get(tile_type)
        .map(|t| t.diggable)
        .unwrap_or(false)
}

/// Check if a tile type is claimable by the player
pub fn is_claimable(tile_type: &str, game_data: &GameData) -> bool {
    game_data
        .tiles
        .get(tile_type)
        .map(|t| t.claimable)
        .unwrap_or(false)
}

/// Check if a tile type is walkable for creatures
pub fn is_walkable(tile_type: &str, game_data: &GameData) -> bool {
    // If it's a room, it's walkable (unless room data says otherwise, but tiles here)
    // If it's a known tile, check !blocks_movement
    if is_room(tile_type, game_data) {
        return true;
    }

    // Hero buildings are walkable tiles (floor with building on top)
    if game_data.hero_buildings.contains_key(tile_type) {
        return true;
    }

    game_data
        .tiles
        .get(tile_type)
        .map(|t| !t.blocks_movement)
        .unwrap_or(false) // unknown tiles not walkable
}

pub fn is_tile_walkable(tile: &TileState, game_data: &GameData) -> bool {
    if let Some(trap) = &tile.trap {
        if trap.constructed
            && trap.active
            && trap.locked
            && game_data
                .traps
                .get(&trap.trap_type)
                .map(|trap_data| trap_data.effects.blocks_movement)
                .unwrap_or(false)
        {
            return false;
        }
    }

    is_walkable(&tile.tile_type, game_data)
}

/// Check if a tile type is a room tile
pub fn is_room(tile_type: &str, game_data: &GameData) -> bool {
    game_data.rooms.contains_key(tile_type)
}

/// Check if a tile type can have a room built on it
pub fn can_build_room(tile_type: &str, game_data: &GameData) -> bool {
    game_data
        .tiles
        .get(tile_type)
        .map(|t| t.supports_rooms)
        .unwrap_or(false)
}

/// Check if a tile type is secure (blocks vision/movement)
pub fn is_secure(tile_type: &str, game_data: &GameData) -> bool {
    // Old implementation included SOLID_ROCK, EARTH, REINFORCED_WALL, BEDROCK
    // Excluded resources.
    // We can check blocks_movement && !diggable? No, earth is diggable.
    // Secure means "solid wall".
    if tile_type == types::REINFORCED_WALL {
        return true;
    }

    game_data
        .tiles
        .get(tile_type)
        .map(|t| t.blocks_movement) // Simplification: if it blocks movement, it's secure enough?
        // Resources block movement too.
        .unwrap_or(false)
}

/// Check if a tile type blocks line of sight
pub fn blocks_vision(tile_type: &str, game_data: &GameData) -> bool {
    game_data
        .tiles
        .get(tile_type)
        .map(|t| t.blocks_vision)
        .unwrap_or(false)
}
