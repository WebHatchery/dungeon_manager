//! Room validation and quality calculation
//! Stateless service for detecting rooms, validating shapes, and calculating quality

use crate::data::rooms::RoomData;
use crate::data::GameData;
use crate::engine::tile_grid::{get_tile, Grid};
use crate::engine::tile_types;
use crate::state::tile_state::{Ownership, TilePos};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// A detected room instance in the game world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: usize,
    pub room_type: String,
    pub tiles: HashSet<TilePos>,
    pub quality: f32,
    pub efficiency: f32,
    pub active: bool,
    pub work_slots: Vec<TilePos>,
}

impl Room {
    /// Create a new room instance
    pub fn new(
        id: usize,
        room_type: String,
        tiles: HashSet<TilePos>,
        work_slots: Vec<TilePos>,
    ) -> Self {
        Self {
            id,
            room_type,
            tiles,
            quality: 0.0,
            efficiency: 1.0,
            active: false,
            work_slots,
        }
    }

    /// Get the center position of the room (average of all tile positions)
    pub fn get_center(&self) -> TilePos {
        if self.tiles.is_empty() {
            return TilePos::new(0, 0);
        }

        let sum_x: i32 = self.tiles.iter().map(|pos| pos.x).sum();
        let sum_y: i32 = self.tiles.iter().map(|pos| pos.y).sum();
        let count = self.tiles.len() as i32;

        TilePos::new(sum_x / count, sum_y / count)
    }
}

/// Calculate valid work slots for a room based on work station size
/// Uses a greedy packing algorithm to find non-overlapping rectangles
pub fn calculate_work_slots(tiles: &HashSet<TilePos>, work_size: [u32; 2]) -> Vec<TilePos> {
    let mut slots = Vec::new();
    let width = work_size[0] as i32;
    let height = work_size[1] as i32;

    // If work size is 1x1, every tile is a potential slot (optimization)
    if width == 1 && height == 1 {
        // For 1x1, we can just use all tiles, or maybe a subset?
        // Let's use all tiles for now, but sort them for consistency
        let mut sorted_tiles: Vec<TilePos> = tiles.iter().cloned().collect();
        sorted_tiles.sort_by_key(|a| (a.y, a.x));
        return sorted_tiles;
    }

    // For larger slots, we need to pack them
    // Sort tiles to try filling from top-left
    let mut sorted_tiles: Vec<TilePos> = tiles.iter().cloned().collect();
    sorted_tiles.sort_by_key(|a| (a.y, a.x));

    let mut occupied = HashSet::new();

    for start_pos in &sorted_tiles {
        if occupied.contains(start_pos) {
            continue;
        }

        // Check if a rectangle of size width x height fits here
        let mut fits = true;
        let mut slot_tiles = Vec::new();

        for dy in 0..height {
            for dx in 0..width {
                let check_pos = TilePos::new(start_pos.x + dx, start_pos.y + dy);
                if !tiles.contains(&check_pos) || occupied.contains(&check_pos) {
                    fits = false;
                    break;
                }
                // Also ensure the tile is secure/valid? check walls?
                // The room validation already checks for walls, but internal obstructions might exist
                // Logic already implies tiles are part of the room, so they are floor.
                slot_tiles.push(check_pos);
            }
            if !fits {
                break;
            }
        }

        if fits {
            // Mark these tiles as occupied
            for tile in slot_tiles {
                occupied.insert(tile);
            }

            // The slot position is the center of the rectangle
            // For even sizes it's between tiles, so we pick: x + (width-1)/2, y + (height-1)/2
            let slot_center_x = start_pos.x + ((width - 1) / 2);
            let slot_center_y = start_pos.y + ((height - 1) / 2);

            slots.push(TilePos::new(slot_center_x, slot_center_y));
        }
    }

    slots
}

/// Metrics describing the shape of a room
#[derive(Debug, Clone)]
pub struct ShapeMetrics {
    /// Width to height ratio (or inverse if height is larger)
    pub aspect_ratio: f32,
    /// How circular/square the room is (0.0 = line, 1.0 = perfect circle)
    pub compactness: f32,
    /// Whether the room has disconnected regions
    pub is_fragmented: bool,
    /// Whether the room is a thin corridor (aspect ratio > 4:1)
    pub is_thin_corridor: bool,
}

/// Detect all rooms of a specific type in the grid
/// Uses flood-fill to find contiguous regions of claimed tiles
pub fn detect_rooms(grid: &Grid, room_type: &str) -> Vec<HashSet<TilePos>> {
    let mut visited = HashSet::new();
    let mut rooms = Vec::new();

    // Scan grid for tiles of this room type
    for row in grid {
        for tile in row {
            // Skip if already visited or not matching room type
            if visited.contains(&tile.pos) {
                continue;
            }

            // Only detect rooms on tiles that match the room type
            if tile.tile_type == room_type && tile.ownership == Ownership::Player {
                let room_tiles = flood_fill(grid, tile.pos, room_type, &mut visited);
                if !room_tiles.is_empty() {
                    rooms.push(room_tiles);
                }
            }
        }
    }

    rooms
}

/// Calculate the efficiency of a room based on its enclosure by walls/doors
/// Returns a value between 0.0 and 1.0
pub fn calculate_efficiency(room: &Room, grid: &Grid, game_data: &GameData) -> f32 {
    let mut total_perimeter_segments = 0.0f32;
    let mut secured_segments = 0.0f32;

    for tile_pos in &room.tiles {
        let neighbors = [
            TilePos::new(tile_pos.x + 1, tile_pos.y),
            TilePos::new(tile_pos.x - 1, tile_pos.y),
            TilePos::new(tile_pos.x, tile_pos.y + 1),
            TilePos::new(tile_pos.x, tile_pos.y - 1),
        ];

        for neighbor in neighbors {
            if room.tiles.contains(&neighbor) {
                continue;
            }

            total_perimeter_segments += 1.0;
            if is_perimeter_secured(neighbor, grid, game_data) {
                secured_segments += 1.0;
            }
        }
    }

    if total_perimeter_segments == 0.0 {
        return 1.0;
    }

    (secured_segments / total_perimeter_segments).max(0.25)
}

/// Check if a perimeter position is secured (by wall, door, or map edge)
fn is_perimeter_secured(pos: TilePos, grid: &Grid, game_data: &GameData) -> bool {
    if pos.x < 0 || pos.y < 0 || (pos.y as usize) >= grid.len() || (pos.x as usize) >= grid[0].len()
    {
        return true; // Map edge counts as secured
    }

    let tile = &grid[pos.y as usize][pos.x as usize];

    // Check for walls
    if tile_types::is_secure(&tile.tile_type, game_data) {
        return true;
    }

    // Check for constructed objects that block movement (like doors)
    if let Some(trap) = &tile.trap {
        if trap.constructed {
            if let Some(trap_data) = game_data.traps.get(&trap.trap_type) {
                if trap_data.effects.blocks_movement {
                    return true;
                }
            }
        }
    }

    false
}

/// Flood fill algorithm to find all contiguous tiles from a starting position
fn flood_fill(
    grid: &Grid,
    start: TilePos,
    target_type: &str,
    visited: &mut HashSet<TilePos>,
) -> HashSet<TilePos> {
    let mut result = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);

    while let Some(pos) = queue.pop_front() {
        // Skip if already visited
        if visited.contains(&pos) {
            continue;
        }

        // Check if this tile exists and is valid
        if let Some(tile) = get_tile(grid, pos) {
            // Only include player-owned claimed tiles of the same type
            if tile.ownership != Ownership::Player || tile.tile_type != target_type {
                continue;
            }

            // Mark as visited and add to result
            visited.insert(pos);
            result.insert(pos);

            // Add neighbors to queue (4-way connectivity for rooms)
            let neighbors = [
                TilePos::new(pos.x + 1, pos.y),
                TilePos::new(pos.x - 1, pos.y),
                TilePos::new(pos.x, pos.y + 1),
                TilePos::new(pos.x, pos.y - 1),
            ];

            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    result
}

/// Validate that a set of tiles meets the requirements for a room type
pub fn validate_room_shape(tiles: &HashSet<TilePos>, room_data: &RoomData) -> Result<(), String> {
    let size = tiles.len() as u32;

    // Check minimum size
    if size < room_data.build.min_tiles {
        return Err(format!(
            "Room too small: {} tiles (minimum: {})",
            size, room_data.build.min_tiles
        ));
    }

    // Check maximum size
    if size > room_data.build.max_tiles {
        return Err(format!(
            "Room too large: {} tiles (maximum: {})",
            size, room_data.build.max_tiles
        ));
    }

    // Check if tiles are contiguous (no fragmentation)
    if !is_contiguous(tiles) {
        return Err("Room tiles must be contiguous".to_string());
    }

    Ok(())
}

/// Check if a set of tiles forms a contiguous region
fn is_contiguous(tiles: &HashSet<TilePos>) -> bool {
    if tiles.is_empty() {
        return false;
    }

    // Start flood fill from first tile
    let start = *tiles.iter().next().unwrap();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);

    while let Some(pos) = queue.pop_front() {
        if visited.contains(&pos) {
            continue;
        }

        visited.insert(pos);

        // Check 4-way neighbors
        let neighbors = [
            TilePos::new(pos.x + 1, pos.y),
            TilePos::new(pos.x - 1, pos.y),
            TilePos::new(pos.x, pos.y + 1),
            TilePos::new(pos.x, pos.y - 1),
        ];

        for neighbor in neighbors {
            if tiles.contains(&neighbor) && !visited.contains(&neighbor) {
                queue.push_back(neighbor);
            }
        }
    }

    // All tiles should be reachable from the start
    visited.len() == tiles.len()
}

/// Calculate shape metrics for a set of tiles
pub fn calculate_shape_metrics(tiles: &HashSet<TilePos>) -> ShapeMetrics {
    if tiles.is_empty() {
        return ShapeMetrics {
            aspect_ratio: 1.0,
            compactness: 0.0,
            is_fragmented: true,
            is_thin_corridor: false,
        };
    }

    // Calculate bounding box
    let min_x = tiles.iter().map(|p| p.x).min().unwrap();
    let max_x = tiles.iter().map(|p| p.x).max().unwrap();
    let min_y = tiles.iter().map(|p| p.y).min().unwrap();
    let max_y = tiles.iter().map(|p| p.y).max().unwrap();

    let width = (max_x - min_x + 1) as f32;
    let height = (max_y - min_y + 1) as f32;

    // Calculate aspect ratio (always >= 1.0)
    let aspect_ratio = if width > height {
        width / height
    } else {
        height / width
    };

    // Calculate compactness (ratio of actual tiles to bounding box)
    let bounding_box_area = width * height;
    let actual_area = tiles.len() as f32;
    let compactness = actual_area / bounding_box_area;

    // Check for thin corridor (aspect ratio > 4:1)
    let is_thin_corridor = aspect_ratio > 4.0;

    // Check for fragmentation
    let is_fragmented = !is_contiguous(tiles);

    ShapeMetrics {
        aspect_ratio,
        compactness,
        is_fragmented,
        is_thin_corridor,
    }
}

/// Calculate the quality of a room based on size and shape
pub fn calculate_room_quality(room: &Room, room_data: &RoomData) -> f32 {
    let size = room.tiles.len() as u32;
    let mut quality = 100.0; // Start at 100% base quality

    // Apply size threshold multipliers
    let mut size_multiplier = 1.0;
    for threshold in &room_data.scaling.size_thresholds {
        if size >= threshold.tiles {
            size_multiplier = threshold.multiplier;
        } else {
            break; // Thresholds should be sorted, stop at first unmet
        }
    }
    quality *= size_multiplier;

    // Calculate shape metrics
    let metrics = calculate_shape_metrics(&room.tiles);

    // Apply shape penalties
    if metrics.is_thin_corridor {
        if let Some(&penalty) = room_data.scaling.shape_penalties.get("thin_corridor") {
            quality *= penalty;
        }
    }

    if metrics.is_fragmented {
        if let Some(&penalty) = room_data.scaling.shape_penalties.get("fragmented") {
            quality *= penalty;
        }
    }

    // Apply compactness penalty for very sparse rooms
    if metrics.compactness < 0.5 {
        if let Some(&penalty) = room_data.scaling.shape_penalties.get("sparse") {
            quality *= penalty;
        } else {
            // Default penalty for sparse rooms
            quality *= 0.9;
        }
    }

    // Apply poor aspect ratio penalty
    if metrics.aspect_ratio > 3.0 && !metrics.is_thin_corridor {
        if let Some(&penalty) = room_data.scaling.shape_penalties.get("poor_aspect_ratio") {
            quality *= penalty;
        } else {
            // Default penalty
            quality *= 0.95;
        }
    }

    quality
}

/// Find the best room of a given type near a position
/// Returns (room_index, distance) or None if no suitable room found
pub fn find_nearest_room(
    rooms: &[Room],
    room_type: &str,
    pos: TilePos,
    min_quality: f32,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;

    for room in rooms.iter() {
        // Check if room matches type and quality requirements
        if room.room_type != room_type || room.quality < min_quality || !room.active {
            continue;
        }

        // Calculate distance to room center
        let center = room.get_center();
        let dx = (center.x - pos.x) as f32;
        let dy = (center.y - pos.y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();

        // Update best if this is closer
        // Update best if this is closer
        match best {
            None => best = Some((room.id, distance)),
            Some((_, best_dist)) if distance < best_dist => {
                best = Some((room.id, distance));
            }
            _ => {}
        }
    }

    best
}

/// Why the dungeon as a whole will not accept another room of this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementRefusal {
    /// `requirements.global_rooms_required` names a room that has not been built.
    MissingPrerequisite(String),
    /// `requirements.max_instances` is already satisfied.
    AlreadyAtLimit(u32),
    /// `requirements.forbidden_if` names a room that has been built.
    ForbiddenBy(String),
}

impl PlacementRefusal {
    pub fn message(&self, game_data: &GameData) -> String {
        let name = |id: &str| {
            game_data
                .rooms
                .get(id)
                .map(|room| room.name.clone())
                .unwrap_or_else(|| id.to_string())
        };
        match self {
            Self::MissingPrerequisite(id) => format!("Requires a {} first.", name(id)),
            Self::AlreadyAtLimit(max) => {
                format!("Already have the maximum of {max}.")
            }
            Self::ForbiddenBy(id) => format!("Cannot coexist with a {}.", name(id)),
        }
    }
}

/// Whether the dungeon permits another room of this type at all.
///
/// Covers the requirements that are about the *dungeon* rather than a
/// particular tile. None of the three was enforced:
/// `global_rooms_required` was read only to draw a tooltip, so the build
/// button promised "Requires: Library" and the engine let you build without
/// one; `max_instances` and `forbidden_if` were read nowhere.
pub fn dungeon_permits_room(room_data: &RoomData, rooms: &[Room]) -> Result<(), PlacementRefusal> {
    let built: HashSet<&str> = rooms
        .iter()
        .filter(|room| room.active)
        .map(|room| crate::data::rooms::room_data_id(&room.room_type))
        .collect();

    for required in &room_data.requirements.global_rooms_required {
        if !built.contains(required.as_str()) {
            return Err(PlacementRefusal::MissingPrerequisite(required.clone()));
        }
    }

    for forbidden in &room_data.requirements.forbidden_if {
        if built.contains(forbidden.as_str()) {
            return Err(PlacementRefusal::ForbiddenBy(forbidden.clone()));
        }
    }

    // 0 means unlimited, which is how every shipped room is authored.
    let max = room_data.requirements.max_instances;
    if max > 0 {
        let this_id = room_data.id.as_str();
        let existing = rooms
            .iter()
            .filter(|room| room.active)
            .filter(|room| crate::data::rooms::room_data_id(&room.room_type) == this_id)
            .count() as u32;
        if existing >= max {
            return Err(PlacementRefusal::AlreadyAtLimit(max));
        }
    }

    Ok(())
}

/// Whether one tile can carry this room.
///
/// Replaces three hardcoded conditions that happened to match every shipped
/// room's data exactly — `ownership == Player` for `requires_claimed`,
/// `room_id.is_none()` for `!can_overlap`, and `supports_rooms` standing in
/// for `allowed_terrain`. All 22 rooms author the same values, so this changes
/// nothing today; it means a room that wants to sit on lava, or overlap
/// another, becomes a data edit rather than a code one.
pub fn tile_permits_room(
    room_data: &RoomData,
    tile: &crate::state::tile_state::TileState,
    game_data: &GameData,
) -> bool {
    if room_data.build.requires_claimed && tile.ownership != Ownership::Player {
        return false;
    }

    if !room_data.build.can_overlap && tile.room_id.is_some() {
        return false;
    }

    // An empty `allowed_terrain` means "anywhere a room can go at all".
    if !room_data.build.allowed_terrain.is_empty()
        && !room_data.build.allowed_terrain.contains(&tile.tile_type)
    {
        return false;
    }

    // `dig_required` is about the tile having been excavated; a tile that
    // still blocks movement plainly has not been.
    if room_data.build.dig_required && !tile_types::is_walkable(&tile.tile_type, game_data) {
        return false;
    }

    tile_types::can_build_room(&tile.tile_type, game_data)
}

/// The `rooms.json` entry backing a room instance, resolving the
/// `training_room`/`training_hall` alias on the way.
pub fn room_data_for<'a>(
    room: &Room,
    game_data: &'a crate::data::GameData,
) -> Option<&'a crate::data::RoomData> {
    game_data
        .rooms
        .get(crate::data::rooms::room_data_id(&room.room_type))
}

/// The `creature_defense_modifier` of the active room covering `pos`, or 0
/// outside one.
///
/// Generic on the effect, like [`room_data_for`]'s other callers: any room
/// declaring a modifier fortifies the creatures standing in it. The barracks
/// has authored a value of 2 since it shipped with nothing reading it.
pub fn room_defense_at(
    pos: TilePos,
    room_manager: &crate::state::room_manager::RoomManager,
    game_data: &crate::data::GameData,
) -> f32 {
    room_manager
        .get_room_at(pos)
        .filter(|room| room.active)
        .and_then(|room| room_data_for(room, game_data))
        .map(|data| data.effects.creature_defense_modifier)
        .unwrap_or(0.0)
}

/// Nearest active room whose data declares `ai.task_type == task_type`.
///
/// The sibling of [`find_nearest_room`], keyed on what a room *does* rather
/// than which room it is. Research used to mean `room_type == "library"` in
/// two separate places, so a second research room could be built, staffed and
/// still produce nothing. Matching the task family instead makes "another room
/// that researches" an authoring decision rather than a code change.
pub fn find_nearest_room_for_task(
    rooms: &[Room],
    task_type: &str,
    pos: TilePos,
    game_data: &crate::data::GameData,
) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;

    for room in rooms.iter() {
        if !room.active {
            continue;
        }
        let matches = room_data_for(room, game_data)
            .map(|data| data.ai.task_type == task_type)
            .unwrap_or(false);
        if !matches {
            continue;
        }

        let center = room.get_center();
        let dx = (center.x - pos.x) as f32;
        let dy = (center.y - pos.y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();

        match best {
            None => best = Some((room.id, distance)),
            Some((_, best_dist)) if distance < best_dist => {
                best = Some((room.id, distance));
            }
            _ => {}
        }
    }

    best
}

#[cfg(test)]
mod tests;
