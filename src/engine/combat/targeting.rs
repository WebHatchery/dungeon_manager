use super::*;

/// Get the attack range for a given attack type
pub fn get_attack_range(attack_type: &str, game_data: &GameData) -> i32 {
    match attack_type {
        "melee" => game_data.config.combat_ranges.melee,
        "ranged" => game_data.config.combat_ranges.ranged,
        "magic" => game_data.config.combat_ranges.magic,
        _ => game_data.config.combat_ranges.melee,
    }
}

/// Calculate Manhattan distance between two positions (public for use elsewhere)
pub fn manhattan_distance(a: TilePos, b: TilePos) -> i32 {
    calculate_manhattan_distance(a, b)
}

/// Check if two entities are in combat range and have line of sight
pub fn in_combat_range(
    attacker_pos: TilePos,
    defender_pos: TilePos,
    attack_type: &str,
    dungeon_grid: &[Vec<crate::state::tile_state::TileState>],
    game_data: &GameData,
) -> bool {
    let distance = calculate_manhattan_distance(attacker_pos, defender_pos);
    let max_range = get_attack_range(attack_type, game_data);

    if distance > max_range {
        return false;
    }

    // Line of Sight Check
    // Melee always hits if adjacent (distance <= 1)
    if distance <= 1 {
        return true;
    }

    check_line_of_sight(attacker_pos, defender_pos, dungeon_grid, game_data)
}

/// Calculate Manhattan distance between positions
fn calculate_manhattan_distance(a: TilePos, b: TilePos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Simple line of sight check using Bresenham's algorithm
fn check_line_of_sight(
    start: TilePos,
    end: TilePos,
    grid: &[Vec<crate::state::tile_state::TileState>],
    game_data: &GameData,
) -> bool {
    let (x0, y0, x1, y1) = (start.x, start.y, end.x, end.y);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);

    loop {
        if x == x1 && y == y1 {
            return true;
        }

        if !(x == x0 && y == y0) && tile_blocks_vision(x, y, grid, game_data) {
            return false;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Check if a tile at the given coordinates blocks vision
fn tile_blocks_vision(
    x: i32,
    y: i32,
    grid: &[Vec<crate::state::tile_state::TileState>],
    game_data: &GameData,
) -> bool {
    let row = match grid.get(y as usize) {
        Some(r) => r,
        None => return false,
    };
    let tile = match row.get(x as usize) {
        Some(t) => t,
        None => return false,
    };
    game_data
        .tiles
        .get(&tile.tile_type)
        .map(|td| td.blocks_vision)
        .unwrap_or(false)
}

/// Get detection range for an entity (how far they can see enemies to engage)
pub fn get_detection_range(entity: &Entity, game_data: &GameData) -> i32 {
    match &entity.entity_type {
        crate::state::entities::EntityType::Creature(state) => game_data
            .monsters
            .get(&state.creature_id)
            .map(|data| data.stats.sight_radius as i32)
            .unwrap_or(8),
        crate::state::entities::EntityType::Hero(state) => game_data
            .heroes
            .get(&state.hero_id)
            .map(|data| data.stats.sight_radius as i32)
            .unwrap_or(8),
        _ => 8,
    }
}

/// Find potential combat targets for an entity within detection range, sorted by priority (heroes > buildings) then distance
/// This uses DETECTION range (sight), not attack range - creatures will chase enemies they can see
pub fn find_combat_targets(
    entity: &Entity,
    entities: &HashMap<EntityId, Entity>,
    dungeon: &crate::state::dungeon::Dungeon,
    game_data: &GameData,
) -> Vec<EntityId> {
    // (EntityId, Distance, Priority)
    // Priority: 0 = Hero/Creature (High), 1 = Structure (Low)
    let mut targets: Vec<(EntityId, i32, u8)> = Vec::new();

    let detection_range = get_detection_range(entity, game_data);

    for (other_id, other_entity) in entities {
        if *other_id == entity.id {
            continue; // Don't target self
        }

        if !other_entity.is_alive() {
            continue; // Skip dead entities
        }

        // Check if entities are hostile
        if are_hostile(entity, other_entity, game_data) {
            let distance = calculate_manhattan_distance(entity.pos, other_entity.pos);

            // Check if within detection range (sight range)
            if distance <= detection_range {
                // Also check line of sight for ranged detection
                if distance <= 1
                    || check_line_of_sight(entity.pos, other_entity.pos, &dungeon.grid, game_data)
                {
                    let priority = match other_entity.entity_type {
                        crate::state::entities::EntityType::Structure(_) => 1,
                        _ => 0,
                    };
                    targets.push((*other_id, distance, priority));
                }
            }
        }
    }

    // Sort by priority (asc) then distance (asc)
    // This ensures Heroes/Creatures (0) are targeted before Structures (1)
    // Within same priority, closest target is preferred
    targets.sort_by_key(|(_, dist, priority)| (*priority, *dist));

    targets.into_iter().map(|(id, _, _)| id).collect()
}

/// Get the attack type for an entity
pub fn get_entity_attack_type(entity: &Entity, game_data: &GameData) -> String {
    match &entity.entity_type {
        crate::state::entities::EntityType::Creature(state) => game_data
            .monsters
            .get(&state.creature_id)
            .map(|data| data.combat.attack_type.clone())
            .unwrap_or_else(|| "melee".to_string()),
        crate::state::entities::EntityType::Hero(state) => game_data
            .heroes
            .get(&state.hero_id)
            .map(|data| data.combat.attack_type.clone())
            .unwrap_or_else(|| "melee".to_string()),
        crate::state::entities::EntityType::Structure(_) => "none".to_string(),
        crate::state::entities::EntityType::ResourcePile(_) => "none".to_string(),
    }
}

/// Check if two entities are hostile to each other
fn are_hostile(entity_a: &Entity, entity_b: &Entity, game_data: &GameData) -> bool {
    if matches!(
        entity_a.entity_type,
        crate::state::entities::EntityType::ResourcePile(_)
    ) || matches!(
        entity_b.entity_type,
        crate::state::entities::EntityType::ResourcePile(_)
    ) {
        return false;
    }

    if entity_a.owner.is_hostile_to(&entity_b.owner) {
        return true;
    }

    if entity_a.owner == entity_b.owner {
        return false;
    }

    let faction_a = get_faction(entity_a, game_data);
    let faction_b = get_faction(entity_b, game_data);

    match (faction_a.as_str(), faction_b.as_str()) {
        ("resource", _) | (_, "resource") => false, // Resources are neutral
        ("dungeon", "hero") => true,
        ("hero", "dungeon") => true,
        ("wild", "wild") => false, // Wild creatures don't attack each other
        ("wild", "dungeon") => true, // Wild attacks dungeon creatures
        ("wild", "hero") => true,  // Wild attacks heroes
        ("dungeon", "wild") => true, // Dungeon creatures attack wild
        ("hero", "wild") => true,  // Heroes attack wild
        ("hero", "hero") => false,
        ("dungeon", "dungeon") => false, // Friendly fire off
        _ => false,
    }
}

fn get_faction(entity: &Entity, game_data: &GameData) -> String {
    if entity.owner != crate::state::OwnerId::Neutral {
        return entity.owner.label();
    }

    match &entity.entity_type {
        crate::state::entities::EntityType::Creature(c) => game_data
            .monsters
            .get(&c.creature_id)
            .map(|m| m.faction.clone())
            .unwrap_or("dungeon".to_string()),

        crate::state::entities::EntityType::Hero(h) => {
            if h.is_converted {
                "dungeon".to_string()
            } else {
                "hero".to_string()
            }
        }
        crate::state::entities::EntityType::Structure(_) => "hero".to_string(), // Structures belong to hero faction
        crate::state::entities::EntityType::ResourcePile(_) => "resource".to_string(),
    }
}
