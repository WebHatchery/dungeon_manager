use crate::data::GameData;
use crate::engine::room_validator::Room;
use crate::state::entities::{CreatureState, EntityId, Task};
use crate::state::game_state::GameState;
use crate::state::tile_state::TilePos;
use crate::state::OwnerId;
use std::collections::HashSet;

const VAMPIRE_CORPSE_COST: u32 = 5;
const SCAVENGER_CONVERSION_RATE: f32 = 0.2;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpecialRoomReport {
    pub mana_generated: f32,
    pub sacrificed_creatures: usize,
    pub corpses_collected: usize,
    /// Bodies rendered down by a Soul Furnace, over and above what the
    /// graveyard could store.
    pub corpses_burned: usize,
    pub vampires_spawned: usize,
    pub scavenged_creatures: usize,
    pub scavenger_progressed: usize,
    pub barracks_marker_set: Option<TilePos>,
    pub barracks_grouped: usize,
}

pub fn process_special_rooms(
    state: &mut GameState,
    game_data: &GameData,
    dt: f32,
) -> SpecialRoomReport {
    let mut report = SpecialRoomReport {
        mana_generated: generate_room_mana(state, game_data, dt),
        ..Default::default()
    };

    report.corpses_collected = collect_graveyard_corpses(state, game_data);

    // After the graveyard, so the furnace only gets the surplus.
    let (burned, soul_mana) = burn_corpses_in_furnaces(state, game_data);
    report.corpses_burned = burned;
    report.mana_generated += soul_mana;
    if burned > 0 {
        state.notifications.info(format!(
            "The Soul Furnace rendered {} {} into {:.0} mana.",
            burned,
            if burned == 1 { "body" } else { "bodies" },
            soul_mana
        ));
    }

    if report.mana_generated > 0.0 {
        state
            .player
            .add_resources_precise(0.0, report.mana_generated, 0.0, 0.0);
    }

    report.vampires_spawned = spawn_vampires_from_corpses(state, game_data);
    if report.vampires_spawned > 0 {
        state.notifications.info(format!(
            "{} vampire rose from the graveyard.",
            report.vampires_spawned
        ));
    }

    let scavenger = progress_scavenger_rooms(state, game_data, dt);
    report.scavenged_creatures = scavenger.converted;
    report.scavenger_progressed = scavenger.progressed;
    report.barracks_marker_set = maintain_barracks_defense_marker(state);
    report.barracks_grouped = group_barracks_creatures(state, game_data);
    report
}

pub fn sacrifice_creature_at(
    state: &mut GameState,
    game_data: &GameData,
    entity_id: EntityId,
    pos: TilePos,
) -> bool {
    if !is_room_at(state, pos, "temple") && !is_room_at(state, pos, "ritual_circle") {
        return false;
    }

    let mana = {
        let Some(entity) = state.entities.get(entity_id) else {
            return false;
        };
        if entity.owner != OwnerId::Player {
            return false;
        }
        let Some(creature) = entity.as_creature() else {
            return false;
        };
        temple_sacrifice_mana(creature, game_data)
    };

    state.entities.remove(entity_id);
    state.player.add_resources_precise(0.0, mana, 0.0, 0.0);
    state
        .notifications
        .info(format!("Temple sacrifice yielded {:.0} mana.", mana));
    true
}

pub fn room_object_capacity(room: &Room, game_data: &GameData, object_id: &str) -> usize {
    let Some(room_data) = game_data.rooms.get(&room.room_type) else {
        return room.tiles.len().max(1);
    };

    let object_capacity = room_data
        .visual
        .object_spawn
        .iter()
        .find(|spawn| spawn.object == object_id)
        .map(|spawn| ((room.tiles.len() as f32) * spawn.density).ceil() as usize)
        .unwrap_or(0);

    if object_capacity > 0 {
        return object_capacity;
    }

    if room_data.ai.max_creatures > 0 {
        return room_data.ai.max_creatures as usize;
    }

    room.tiles.len().max(1)
}

/// Passive mana from every room that declares it.
///
/// Used to test `room_type == "temple" || == "ritual_circle"`, so a third
/// mana-generating room could carry a rate in `rooms.json` and produce
/// nothing. The effect is the qualifier now, not the room name.
fn generate_room_mana(state: &GameState, game_data: &GameData, dt: f32) -> f32 {
    state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active)
        .filter_map(|room| {
            crate::engine::room_validator::room_data_for(room, game_data).map(|room_data| {
                room.tiles.len() as f32
                    * room.efficiency
                    * room_data.effects.mana_generation_per_second
                    * dt
            })
        })
        .sum()
}

/// Bodies left on the floor rendered down into mana by a Soul Furnace.
///
/// Runs *after* the graveyard has taken what it can store, so a keeper with
/// both rooms fills the graveyard first — bodies there become vampires — and
/// burns only the surplus. The two rooms competing for the same corpses is the
/// point; it makes the choice between them a real one.
///
/// Any room declaring `mana_per_corpse` does this. Where several exist the
/// best-run one does the work, since a corpse can only burn once.
fn burn_corpses_in_furnaces(state: &mut GameState, game_data: &GameData) -> (usize, f32) {
    let mana_per_corpse = state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active)
        .filter_map(|room| {
            crate::engine::room_validator::room_data_for(room, game_data)
                .map(|data| data.effects.mana_per_corpse * room.efficiency)
        })
        .fold(0.0f32, f32::max);

    if mana_per_corpse <= 0.0 {
        return (0, 0.0);
    }

    let dead_heroes: Vec<EntityId> = state
        .entities
        .heroes()
        .filter(|(_, hero)| hero.health <= 0.0 && !hero.is_captured && !hero.is_converted)
        .map(|(id, _)| id)
        .collect();

    if dead_heroes.is_empty() {
        return (0, 0.0);
    }

    for hero_id in &dead_heroes {
        state.entities.remove(*hero_id);
    }

    (
        dead_heroes.len(),
        dead_heroes.len() as f32 * mana_per_corpse,
    )
}

fn temple_sacrifice_mana(creature: &CreatureState, game_data: &GameData) -> f32 {
    let base = game_data
        .monsters
        .get(&creature.creature_id)
        .map(|monster| (monster.stats.health + monster.stats.mana) * 0.25)
        .unwrap_or(25.0);
    base + creature.level as f32 * 10.0
}

fn collect_graveyard_corpses(state: &mut GameState, game_data: &GameData) -> usize {
    let capacity = graveyard_corpse_capacity(state, game_data);
    if capacity == 0 || state.player.graveyard_corpses >= capacity {
        return 0;
    }

    let open_slots = capacity - state.player.graveyard_corpses;
    let dead_heroes: Vec<EntityId> = state
        .entities
        .heroes()
        .filter(|(_, hero)| hero.health <= 0.0 && !hero.is_captured && !hero.is_converted)
        .map(|(id, _)| id)
        .take(open_slots as usize)
        .collect();

    for hero_id in &dead_heroes {
        state.entities.remove(*hero_id);
    }

    state.player.graveyard_corpses += dead_heroes.len() as u32;
    dead_heroes.len()
}

fn graveyard_corpse_capacity(state: &GameState, game_data: &GameData) -> u32 {
    state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active && room.room_type == "graveyard")
        .map(|room| {
            game_data
                .rooms
                .get(&room.room_type)
                .map(|room_data| room_data.effects.corpse_storage.max(0) as u32)
                .unwrap_or(room.tiles.len() as u32)
                .max(1)
        })
        .sum()
}

fn spawn_vampires_from_corpses(state: &mut GameState, game_data: &GameData) -> usize {
    if state.player.graveyard_corpses < VAMPIRE_CORPSE_COST {
        return 0;
    }
    if !graveyards_spawn_vampires(state, game_data) {
        return 0;
    }
    let Some(spawn_pos) = first_room_center(state, "graveyard") else {
        return 0;
    };
    let Some(monster_data) = game_data.monsters.get("vampire") else {
        return 0;
    };

    let mut spawned = 0;
    while state.player.graveyard_corpses >= VAMPIRE_CORPSE_COST {
        state.player.graveyard_corpses -= VAMPIRE_CORPSE_COST;
        let visual_seed = macroquad_toolkit::rng::random_u64();
        let creature_state = CreatureState::new(
            "vampire".to_string(),
            1,
            monster_data.stats.health,
            monster_data.stats.mana,
            visual_seed,
        );
        state
            .entities
            .spawn_creature_for_owner(spawn_pos, creature_state, OwnerId::Player);
        spawned += 1;
    }

    spawned
}

fn graveyards_spawn_vampires(state: &GameState, game_data: &GameData) -> bool {
    state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active && room.room_type == "graveyard")
        .any(|room| {
            game_data
                .rooms
                .get(&room.room_type)
                .map(|room_data| room_data.effects.spawns_vampires)
                .unwrap_or(false)
        })
}

#[derive(Default)]
struct ScavengerResult {
    converted: usize,
    progressed: usize,
}

fn progress_scavenger_rooms(
    state: &mut GameState,
    game_data: &GameData,
    dt: f32,
) -> ScavengerResult {
    let scavenger_rooms: Vec<&Room> = state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active && room.room_type == "scavenger")
        .collect();

    if scavenger_rooms.is_empty() {
        state.player.scavenger_conversion_progress.clear();
        return ScavengerResult::default();
    }

    let scavenger_tiles: HashSet<TilePos> = scavenger_rooms
        .iter()
        .flat_map(|room| room.tiles.iter().copied())
        .collect();
    let capacity: usize = scavenger_rooms
        .iter()
        .map(|room| room_object_capacity(room, game_data, "focus_crystal").max(1))
        .sum();

    let mut candidates: Vec<(EntityId, TilePos)> = state
        .entities
        .all()
        .filter(|entity| {
            entity.as_creature().is_some()
                && entity.owner != OwnerId::Player
                && scavenger_tiles.contains(&entity.pos)
        })
        .map(|entity| (entity.id, entity.pos))
        .collect();
    candidates.sort_by_key(|(id, _)| *id);
    candidates.truncate(capacity);

    let active_ids: HashSet<EntityId> = candidates.iter().map(|(id, _)| *id).collect();
    state
        .player
        .scavenger_conversion_progress
        .retain(|id, _| active_ids.contains(id));

    let mut result = ScavengerResult::default();
    let mut converted_ids = Vec::new();
    for (entity_id, pos) in candidates {
        let room_efficiency = state
            .room_manager
            .get_room_at(pos)
            .map(|room| room.efficiency.max(0.25))
            .unwrap_or(1.0);
        let progress = state
            .player
            .scavenger_conversion_progress
            .entry(entity_id)
            .or_insert(0.0);
        *progress += SCAVENGER_CONVERSION_RATE * room_efficiency * dt;
        result.progressed += 1;
        if *progress >= 1.0 {
            converted_ids.push(entity_id);
        }
    }

    for entity_id in converted_ids {
        state
            .player
            .scavenger_conversion_progress
            .remove(&entity_id);
        if let Some(entity) = state.entities.get_mut(entity_id) {
            entity.owner = OwnerId::Player;
            if let Some(creature) = entity.as_creature_mut() {
                creature.current_task = None;
                creature.current_path = None;
            }
            result.converted += 1;
        }
    }

    result
}

fn maintain_barracks_defense_marker(state: &mut GameState) -> Option<TilePos> {
    if state.defend_marker.is_some() {
        return None;
    }

    let pos = first_room_center(state, "barracks")?;
    state.defend_marker = Some(pos);
    Some(pos)
}

fn group_barracks_creatures(state: &mut GameState, game_data: &GameData) -> usize {
    let Some(marker) = state.defend_marker else {
        return 0;
    };

    // Any room declaring `grouping_point` rallies creatures, not the barracks
    // by name — the effect had been authored on the barracks and read nowhere.
    let capacity: usize = state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active)
        .filter_map(|room| crate::engine::room_validator::room_data_for(room, game_data))
        .filter(|room_data| room_data.effects.grouping_point)
        .map(|room_data| (room_data.ai.max_creatures as usize).max(1))
        .sum();
    if capacity == 0 {
        return 0;
    }

    let mut grouped = 0;
    let ids: Vec<EntityId> = state
        .entities
        .all()
        .filter(|entity| entity.owner == OwnerId::Player)
        .filter_map(|entity| {
            let creature = entity.as_creature()?;
            let monster = game_data.monsters.get(&creature.creature_id)?;
            if creature.creature_id == "imp" || monster.role == "worker" {
                return None;
            }
            match creature.current_task {
                None | Some(Task::Idle) => Some(entity.id),
                _ => None,
            }
        })
        .take(capacity)
        .collect();

    for entity_id in ids {
        if let Some(entity) = state.entities.get_mut(entity_id) {
            if let Some(creature) = entity.as_creature_mut() {
                creature.current_task = Some(Task::MoveTo(marker));
                creature.current_path = None;
                grouped += 1;
            }
        }
    }

    grouped
}

fn first_room_center(state: &GameState, room_type: &str) -> Option<TilePos> {
    state
        .room_manager
        .rooms
        .iter()
        .find(|room| room.active && room.room_type == room_type)
        .map(|room| room.get_center())
}

fn is_room_at(state: &GameState, pos: TilePos, room_type: &str) -> bool {
    state
        .room_manager
        .get_room_at(pos)
        .map(|room| room.active && room.room_type == room_type)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
