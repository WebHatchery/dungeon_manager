//! Tile grid operations and coordinate conversion
//! Stateless functions for coordinate conversion and neighbor detection.

use crate::data::GameData;
use crate::engine::tile_types;
use crate::state::tile_state::{FogState, TilePos, TileState};
#[allow(unused_imports)]
pub use macroquad_toolkit::grid::{iso_to_world, world_to_iso};
use std::collections::HashSet;

pub type Grid = Vec<Vec<TileState>>;

/// Create a rectangular tile grid filled with earth tiles.
pub fn create_grid(width: usize, height: usize, _game_data: &GameData) -> Grid {
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| TileState::new("earth".to_string(), TilePos::new(x as i32, y as i32)))
                .collect()
        })
        .collect()
}

/// Convert screen position to tile position using 3D raycasting with optional grid collision
pub fn screen_to_tile(
    mouse_x: f32,
    mouse_y: f32,
    camera: &macroquad::camera::Camera3D,
    _tile_width: f32,
    _tile_height: f32,
    grid: Option<&Grid>,
    game_data: &GameData,
) -> TilePos {
    // 1. Calculate Normalized Device Coordinates (NDC)
    let ndc_x = (mouse_x / macroquad::window::screen_width()) * 2.0 - 1.0;
    let ndc_y = 1.0 - (mouse_y / macroquad::window::screen_height()) * 2.0;

    // 2. Calculate Ray in View Space
    // Access fovy from camera (assuming 45 degrees as set in CameraState)
    let fov_rad = 45.0f32.to_radians();
    let aspect = macroquad::window::screen_width() / macroquad::window::screen_height();

    let scale_y = (fov_rad * 0.5).tan();
    let scale_x = scale_y * aspect;

    // (x, y, -1) points forward into the screen in OpenGL/Macroquad coordinates
    let ray_view_dir = macroquad::math::vec3(ndc_x * scale_x, ndc_y * scale_y, -1.0).normalize();

    // 3. Convert View Space Ray to World Space
    // Construct camera basis vectors (Standard Right-Handed System)
    let cam_fwd = (camera.target - camera.position).normalize();
    let world_up = macroquad::math::vec3(0.0, 1.0, 0.0);
    let cam_right = cam_fwd.cross(world_up).normalize();
    let cam_up = cam_right.cross(cam_fwd).normalize();

    // Transform direction: WorldDir = Right * View.x + Up * View.y + (-Forward) * View.z
    let ray_world_dir =
        (cam_right * ray_view_dir.x + cam_up * ray_view_dir.y + (-cam_fwd) * ray_view_dir.z)
            .normalize();
    let ray_start = camera.position;

    // Function to intersect ray with a horizontal plane at y = height
    let intersect_plane = |height: f32| -> Option<TilePos> {
        if ray_world_dir.y.abs() < 0.0001 {
            return None;
        }

        let t = (height - ray_start.y) / ray_world_dir.y;

        if t < 0.0 {
            return None;
        } // Hit is behind camera

        let p = ray_start + ray_world_dir * t;
        Some(TilePos::new(p.x.round() as i32, p.z.round() as i32))
    };

    // 4. Check intersection with walls first (y = 0.5)
    if let Some(grid) = grid {
        if let Some(wall_hit) = intersect_plane(0.5) {
            // Check if there is actually a wall at this position
            if let Some(tile) = get_tile(grid, wall_hit) {
                if tile_types::is_wall(&tile.tile_type, game_data) {
                    return wall_hit;
                }
            }
        }
    }

    // 5. Fallback to floor intersection (y = 0.0)
    intersect_plane(0.0).unwrap_or(TilePos::new(0, 0))
}

/// Get a tile from the grid at the specified position
pub fn get_tile(grid: &Grid, pos: TilePos) -> Option<&TileState> {
    if pos.y >= 0 && (pos.y as usize) < grid.len() {
        let row = &grid[pos.y as usize];
        if pos.x >= 0 && (pos.x as usize) < row.len() {
            return Some(&row[pos.x as usize]);
        }
    }
    None
}

/// Get a mutable reference to a tile from the grid
pub fn get_tile_mut(grid: &mut Grid, pos: TilePos) -> Option<&mut TileState> {
    if pos.y >= 0 && (pos.y as usize) < grid.len() {
        let row = &mut grid[pos.y as usize];
        if pos.x >= 0 && (pos.x as usize) < row.len() {
            return Some(&mut row[pos.x as usize]);
        }
    }
    None
}

/// Get the 4 cardinal neighbors (no diagonals)
pub fn get_cardinal_neighbors(grid: &Grid, pos: TilePos) -> Vec<TilePos> {
    let offsets = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut neighbors = Vec::new();

    for (dx, dy) in offsets.iter() {
        let neighbor_pos = TilePos::new(pos.x + dx, pos.y + dy);
        if get_tile(grid, neighbor_pos).is_some() {
            neighbors.push(neighbor_pos);
        }
    }

    neighbors
}

/// Get all 8 neighboring positions, including diagonals.
pub fn get_neighbors(grid: &Grid, pos: TilePos) -> Vec<TilePos> {
    let offsets = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];

    offsets
        .iter()
        .filter_map(|(dx, dy)| {
            let neighbor_pos = TilePos::new(pos.x + dx, pos.y + dy);
            get_tile(grid, neighbor_pos).map(|_| neighbor_pos)
        })
        .collect()
}

/// Calculate fog state for a tile based on player vision
pub fn calculate_fog_state(tile: &TileState, player_vision: &HashSet<TilePos>) -> FogState {
    if player_vision.contains(&tile.pos) {
        FogState::Visible
    } else if tile.fog_state == FogState::Visible {
        // Tile was visible before, now it's not -> transition to Revealed (explored but not seen)
        FogState::Revealed
    } else if tile.fog_state == FogState::Revealed {
        // Keep Revealed tiles as Revealed (previously explored, still explored)
        FogState::Revealed
    } else {
        // Never explored -> stays Hidden
        FogState::Hidden
    }
}

/// Update fog of war based on claimed tiles and creature positions
pub fn update_fog_of_war(
    grid: &mut Grid,
    claimed_tiles: &HashSet<TilePos>,
    creature_positions: &[TilePos],
    sight_radius: i32,
    game_data: &GameData,
) {
    let mut visible_tiles = HashSet::new();

    // Add all claimed tiles to visible
    // Create a read-only snapshot of tile types for line-of-sight checks
    let tile_types: Vec<Vec<String>> = grid
        .iter()
        .map(|row| row.iter().map(|t| t.tile_type.clone()).collect())
        .collect();

    // Reveal adjacent walls for claimed tiles (radius 1.5)
    for claimed_pos in claimed_tiles {
        visible_tiles.insert(*claimed_pos);

        let radius = 1.5f32;
        let r_ceil = radius.ceil() as i32;

        for dy in -r_ceil..=r_ceil {
            for dx in -r_ceil..=r_ceil {
                let distance = (dx * dx + dy * dy) as f32;
                if distance <= (radius * radius) {
                    let tile_pos = TilePos::new(claimed_pos.x + dx, claimed_pos.y + dy);
                    visible_tiles.insert(tile_pos);
                }
            }
        }
    }

    // Add tiles around creatures (with line-of-sight check)
    for creature_pos in creature_positions {
        for dy in -sight_radius..=sight_radius {
            for dx in -sight_radius..=sight_radius {
                let distance = (dx * dx + dy * dy) as f32;
                if distance <= (sight_radius * sight_radius) as f32 {
                    let tile_pos = TilePos::new(creature_pos.x + dx, creature_pos.y + dy);

                    // Check line of sight using the snapshot
                    if has_line_of_sight_snapshot(&tile_types, *creature_pos, tile_pos, game_data) {
                        visible_tiles.insert(tile_pos);
                    }
                }
            }
        }
    }

    // Update fog states
    for row in grid.iter_mut() {
        for tile in row.iter_mut() {
            tile.fog_state = calculate_fog_state(tile, &visible_tiles);
        }
    }
}

/// Check line of sight using a snapshot of tile types (to avoid borrow issues)
fn has_line_of_sight_snapshot(
    tile_types: &[Vec<String>],
    from: TilePos,
    to: TilePos,
    game_data: &GameData,
) -> bool {
    let mut x0 = from.x;
    let mut y0 = from.y;
    let x1 = to.x;
    let y1 = to.y;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        // Check if current tile blocks vision (skip the start position)
        if (x0, y0) != (from.x, from.y) && y0 >= 0 && (y0 as usize) < tile_types.len() {
            let row = &tile_types[y0 as usize];
            if x0 >= 0 && (x0 as usize) < row.len() {
                let tile_type = &row[x0 as usize];
                // Solid tiles block vision
                if tile_types::blocks_vision(tile_type, game_data) {
                    return false;
                }
            }
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    true
}

/// Get the dimensions of the grid
pub fn get_grid_dimensions(grid: &Grid) -> (usize, usize) {
    if grid.is_empty() {
        (0, 0)
    } else {
        (grid[0].len(), grid.len())
    }
}

#[cfg(test)]
mod tests;
