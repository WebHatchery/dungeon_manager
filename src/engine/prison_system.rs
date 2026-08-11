use crate::data::GameData;
use crate::engine::special_rooms::room_object_capacity;
use crate::state::entities::{EntityId, EntityManager, HeroGoal, Task};
use crate::state::notifications::NotificationManager;
use crate::state::room_manager::RoomManager;
use crate::state::tile_state::TilePos;
use crate::state::OwnerId;
use std::collections::{HashMap, HashSet};

/// Handle capturing dead heroes - teleport to available prison cells.
pub fn handle_prison_captures(
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    notifications: &mut NotificationManager,
    game_data: &GameData,
) {
    let mut heroes_to_capture: Vec<EntityId> = entities
        .heroes()
        .filter(|(_, hero)| hero.health <= 0.0 && !hero.is_captured && !hero.is_converted)
        .map(|(hero_id, _)| hero_id)
        .collect();

    if heroes_to_capture.is_empty() {
        return;
    }
    heroes_to_capture.sort_unstable();

    let mut available_prison_tiles = available_prison_tiles(entities, room_manager, game_data);
    if available_prison_tiles.is_empty() {
        return;
    }

    for hero_id in heroes_to_capture {
        let Some(target_pos) = available_prison_tiles.pop() else {
            break;
        };

        if let Some(entity) = entities.get_mut(hero_id) {
            entity.pos = target_pos;
            entity.visual_pos = (target_pos.x as f32, target_pos.y as f32);

            if let Some(hero) = entity.as_hero_mut() {
                hero.is_captured = true;
                hero.conversion_progress = 0.0;
                hero.health = 10.0;

                eprintln!(
                    "Hero {} captured and teleported to prison at {:?}!",
                    hero.hero_id, target_pos
                );
                notifications.success("Hero captured!");
            }
        }
    }
}

/// Progress prison (skeleton) and torture (conversion) logic.
pub fn progress_prison_conversions(
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    notifications: &mut NotificationManager,
    game_data: &GameData,
    dt: f32,
) {
    let active_torture_rooms = active_torture_rooms(entities);
    assign_prisoners_to_torture(entities, room_manager, game_data, &active_torture_rooms);

    let skeleton_rate = game_data.config.conversion.skeleton_rate;
    let torture_base_rate = game_data.config.conversion.torture_rate;

    let mut conversions_to_process: Vec<(EntityId, ConversionKind)> = Vec::new();
    for (hero_id, hero) in entities.heroes() {
        if !hero.is_captured || hero.is_converted {
            continue;
        }
        let Some(entity) = entities.get(hero_id) else {
            continue;
        };
        let Some(room) = room_manager.get_room_at(entity.pos) else {
            continue;
        };

        match room.room_type.as_str() {
            "prison" => conversions_to_process.push((hero_id, ConversionKind::Prison)),
            "torture_chamber" if active_torture_rooms.contains_key(&room.id) => {
                conversions_to_process.push((
                    hero_id,
                    ConversionKind::Torture {
                        room_id: room.id,
                        torturers: active_torture_rooms[&room.id],
                    },
                ));
            }
            _ => {}
        }
    }

    for (hero_id, kind) in conversions_to_process {
        let mut completed = false;
        let mut hero_name = String::new();

        if let Some(entity) = entities.get_mut(hero_id) {
            if let Some(hero) = entity.as_hero_mut() {
                hero_name = hero.hero_id.clone();
                let rate = match kind {
                    ConversionKind::Prison => skeleton_rate,
                    ConversionKind::Torture { room_id, torturers } => {
                        torture_base_rate
                            * torture_power(room_manager, game_data, room_id)
                            * torturers as f32
                    }
                };
                hero.conversion_progress += rate * dt;
                completed = hero.conversion_progress >= 1.0;
            }
        }

        if completed {
            complete_conversion(
                hero_id,
                kind,
                entities,
                notifications,
                game_data,
                &hero_name,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ConversionKind {
    Prison,
    Torture { room_id: usize, torturers: usize },
}

fn available_prison_tiles(
    entities: &EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
) -> Vec<TilePos> {
    let occupied: HashSet<TilePos> = entities
        .all()
        .filter(|entity| {
            entity
                .as_hero()
                .map(|hero| hero.is_captured && !hero.is_converted)
                .unwrap_or(false)
        })
        .map(|entity| entity.pos)
        .collect();

    let mut available = Vec::new();
    for room in room_manager
        .rooms
        .iter()
        .filter(|room| room.active && room.room_type == "prison")
    {
        let capacity = room_object_capacity(room, game_data, "cell").max(1);
        let mut tiles: Vec<TilePos> = room.tiles.iter().copied().collect();
        tiles.sort_by_key(|pos| (pos.y, pos.x));
        available.extend(
            tiles
                .into_iter()
                .filter(|pos| !occupied.contains(pos))
                .take(capacity),
        );
    }
    available.reverse();
    available
}

fn active_torture_rooms(entities: &EntityManager) -> HashMap<usize, usize> {
    let mut rooms = HashMap::new();
    for entity in entities.all() {
        if entity.owner != OwnerId::Player {
            continue;
        }
        let Some(creature) = entity.as_creature() else {
            continue;
        };
        if creature.creature_id != "succubus" {
            continue;
        }
        if let Some(Task::Work(room_id, _)) = creature.current_task {
            *rooms.entry(room_id).or_insert(0) += 1;
        }
    }
    rooms
}

fn assign_prisoners_to_torture(
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
    active_torture_rooms: &HashMap<usize, usize>,
) {
    if active_torture_rooms.is_empty() {
        return;
    }

    let mut prisoners: Vec<EntityId> = entities
        .all()
        .filter(|entity| {
            entity
                .as_hero()
                .map(|hero| hero.is_captured && !hero.is_converted)
                .unwrap_or(false)
        })
        .filter(|entity| {
            room_manager
                .get_room_at(entity.pos)
                .map(|room| room.room_type == "prison")
                .unwrap_or(false)
        })
        .map(|entity| entity.id)
        .collect();
    prisoners.sort_unstable();

    for room_id in active_torture_rooms.keys() {
        let Some(room) = room_manager.rooms.iter().find(|room| room.id == *room_id) else {
            continue;
        };
        if room.room_type != "torture_chamber" {
            continue;
        }

        let capacity = torture_capacity(room, game_data);
        let occupied = entities
            .all()
            .filter(|entity| {
                entity
                    .as_hero()
                    .map(|hero| hero.is_captured && !hero.is_converted)
                    .unwrap_or(false)
            })
            .filter(|entity| room.tiles.contains(&entity.pos))
            .count();
        let open_slots = capacity.saturating_sub(occupied);
        if open_slots == 0 {
            continue;
        }

        let mut tiles: Vec<TilePos> = room.tiles.iter().copied().collect();
        tiles.sort_by_key(|pos| (pos.y, pos.x));
        let target_pos = tiles.first().copied().unwrap_or_else(|| room.get_center());

        for _ in 0..open_slots {
            let Some(hero_id) = prisoners.pop() else {
                return;
            };
            if let Some(entity) = entities.get_mut(hero_id) {
                entity.pos = target_pos;
                entity.visual_pos = (target_pos.x as f32, target_pos.y as f32);
                if let Some(hero) = entity.as_hero_mut() {
                    hero.conversion_progress = 0.0;
                }
            }
        }
    }
}

fn torture_capacity(room: &crate::engine::room_validator::Room, game_data: &GameData) -> usize {
    let rack_capacity = room_object_capacity(room, game_data, "rack");
    let maiden_capacity = room_object_capacity(room, game_data, "iron_maiden");
    (rack_capacity + maiden_capacity).max(1)
}

fn torture_power(room_manager: &RoomManager, game_data: &GameData, room_id: usize) -> f32 {
    room_manager
        .rooms
        .iter()
        .find(|room| room.id == room_id)
        .and_then(|room| {
            game_data
                .rooms
                .get(&room.room_type)
                .map(|data| data.effects.torture_power)
        })
        .unwrap_or(1.0)
        .max(1.0)
}

fn complete_conversion(
    hero_id: EntityId,
    kind: ConversionKind,
    entities: &mut EntityManager,
    notifications: &mut NotificationManager,
    game_data: &GameData,
    hero_name: &str,
) {
    match kind {
        ConversionKind::Torture { .. } => {
            if let Some(entity) = entities.get_mut(hero_id) {
                let pos = entity.pos;
                entity.owner = OwnerId::Player;
                if let Some(hero) = entity.as_hero_mut() {
                    hero.is_converted = true;
                    hero.is_captured = false;
                    hero.health = hero.max_health;
                    hero.current_goal = HeroGoal::RestAtSpawn(pos);
                    hero.current_path = None;
                    notifications.success(format!("{hero_name} converted to your side!"));
                }
            }
        }
        ConversionKind::Prison => {
            let pos = entities
                .get(hero_id)
                .map(|entity| entity.pos)
                .unwrap_or(TilePos::new(0, 0));
            entities.remove(hero_id);

            if let Some(monster_data) = game_data.monsters.get("skeleton") {
                let visual_seed = macroquad_toolkit::rng::random_u64();
                let creature_state = crate::state::entities::CreatureState::new(
                    "skeleton".to_string(),
                    1,
                    monster_data.stats.health,
                    monster_data.stats.mana,
                    visual_seed,
                );
                entities.spawn_creature(pos, creature_state);
                notifications.success("Captured hero rotted into a Skeleton!");
            }
        }
    }
}

#[cfg(test)]
mod tests;
