//! Trap construction and triggering system
//! Handles funding, building, and triggering of traps/doors

use crate::data::GameData;
use crate::state::dungeon::Dungeon;
use crate::state::entities::{EntityId, EntityManager, EntityType};
use crate::state::player_state::PlayerState;
use crate::state::tile_state::TilePos;
use crate::state::OwnerId;
use std::collections::HashSet;

/// Get the material cost for a trap type from game data
pub fn get_trap_cost(trap_type: &str, game_data: &GameData) -> i32 {
    game_data
        .traps
        .get(trap_type)
        .map(|data| data.cost)
        .unwrap_or(50) // Default fallback
}

/// Get the build time for a trap type in seconds from game data
pub fn get_trap_build_time(trap_type: &str, game_data: &GameData) -> f32 {
    game_data
        .traps
        .get(trap_type)
        .map(|data| data.build_time)
        .unwrap_or(5.0) // Default fallback
}

/// Toggle a constructed lockable door at `pos` and return its new state.
///
/// Door locking is deliberately a tile action: the same function serves a
/// mouse tap and the right-click shortcut, and an unlocked door immediately
/// becomes traversable because `tile_types::is_tile_walkable` reads `locked`.
pub fn toggle_door_lock_at(
    dungeon: &mut Dungeon,
    game_data: &GameData,
    pos: TilePos,
) -> Option<bool> {
    let tile = dungeon.get_tile_mut(pos)?;
    let trap = tile.trap.as_mut()?;
    let trap_data = game_data.traps.get(&trap.trap_type)?;
    if !trap.constructed || !trap_data.effects.lockable {
        return None;
    }
    trap.locked = !trap.locked;
    Some(trap.locked)
}

/// Process trap construction progress
/// Returns positions of completed traps
pub fn process_trap_construction(
    dungeon: &mut Dungeon,
    player: &mut PlayerState,
    pending_trap_builds: &mut HashSet<TilePos>,
    game_data: &GameData,
    dt: f32,
) -> Vec<TilePos> {
    let pending: Vec<TilePos> = pending_trap_builds.iter().cloned().collect();

    for pos in &pending {
        try_fund_trap(dungeon, player, *pos);
    }

    let completed_traps: Vec<TilePos> = pending
        .into_iter()
        .filter(|pos| progress_trap_construction(dungeon, *pos, game_data, dt))
        .collect();

    for pos in &completed_traps {
        pending_trap_builds.remove(pos);
    }

    completed_traps
}

/// Try to fund a trap at the given position
fn try_fund_trap(dungeon: &mut Dungeon, player: &mut PlayerState, pos: TilePos) {
    let tile = match dungeon.get_tile_mut(pos) {
        Some(t) => t,
        None => return,
    };
    let trap = match tile.trap.as_mut() {
        Some(t) => t,
        None => return,
    };

    if trap.funded || trap.constructed {
        return;
    }

    let trap_type = trap.trap_type.clone();
    if player.consume_trap_inventory(&trap_type, 1) {
        trap.funded = true;
        trace_log!("traps", "Funded trap at {:?}", pos);
    }
}

/// Progress trap construction, returns true if completed
fn progress_trap_construction(
    dungeon: &mut Dungeon,
    pos: TilePos,
    game_data: &GameData,
    dt: f32,
) -> bool {
    let tile = match dungeon.get_tile_mut(pos) {
        Some(t) => t,
        None => return false,
    };
    let trap = match tile.trap.as_mut() {
        Some(t) => t,
        None => return false,
    };

    if !trap.funded || trap.constructed {
        return false;
    }

    let build_time = get_trap_build_time(&trap.trap_type, game_data);
    trap.construction_progress += dt;

    if trap.construction_progress >= build_time {
        trap.constructed = true;
        trap.active = true;
        trap.locked = game_data
            .traps
            .get(&trap.trap_type)
            .map(|data| data.effects.lockable)
            .unwrap_or(false);
        trace_log!("traps", "Trap construction complete at {:?}", pos);
        return true;
    }

    false
}

/// Result of a trap trigger
#[derive(Debug)]
pub struct TrapTriggerResult {
    pub trap_type: String,
    pub damage_dealt: f32,
}

/// Process trap triggers when entities step on them
/// Call this every tick to check for trap activations
pub fn process_trap_triggers(
    dungeon: &mut Dungeon,
    entities: &mut EntityManager,
    game_data: &GameData,
    dt: f32,
) -> Vec<TrapTriggerResult> {
    update_trap_cooldowns(dungeon, dt);
    rearm_triggered_traps(dungeon, entities, game_data);

    let hero_positions: Vec<(EntityId, TilePos)> = entities
        .heroes()
        .filter_map(|(id, _)| entities.get(id).map(|e| (id, e.pos)))
        .collect();

    let traps_to_trigger: Vec<(TilePos, String, EntityId)> = hero_positions
        .into_iter()
        .filter_map(|(hero_id, hero_pos)| get_triggerable_trap(dungeon, hero_pos, hero_id))
        .collect();

    traps_to_trigger
        .into_iter()
        .filter_map(|(pos, trap_type, hero_id)| {
            let trap_data = game_data.traps.get(&trap_type)?;
            if hero_spots_trap(hero_id, entities, game_data) {
                // Spotted, so not sprung. The trap stays armed for the next
                // hero through — a scout leading the way does not clear the
                // corridor for the militia behind them.
                return None;
            }
            let tending = nearby_trap_tending_bonus(pos, entities, game_data);
            trigger_trap(
                pos, &trap_type, trap_data, hero_id, entities, dungeon, game_data, tending,
            )
        })
        .collect()
}

/// Damage multiplier from creatures tending this trap.
///
/// The Kobold's "buffs nearby traps": any creature whose traits carry a
/// `trap_damage_multiplier` boosts traps within its `trap_tending_radius`. The
/// best tender wins rather than the bonuses stacking, so a crowd of kobolds
/// around one corridor is not a damage multiplier you can farm.
///
/// This is an entity scan, but it runs only when a trap actually fires — an
/// event, not a frame — so it does not touch the per-frame O(n) scan problem.
fn nearby_trap_tending_bonus(pos: TilePos, entities: &EntityManager, game_data: &GameData) -> f32 {
    entities
        .creatures()
        .filter_map(|(id, creature)| {
            let entity_pos = entities.get(id)?.pos;
            let data = game_data.monsters.get(&creature.creature_id)?;
            let best = data
                .traits
                .iter()
                .filter_map(|tag| game_data.traits.get(tag))
                .filter(|t| t.trap_damage_multiplier > 1.0)
                .filter(|t| pos.distance_to(&entity_pos) <= t.trap_tending_radius)
                .map(|t| t.trap_damage_multiplier)
                .fold(1.0f32, f32::max);
            Some(best)
        })
        .fold(1.0f32, f32::max)
}

/// Whether a hero notices the trap under their feet in time.
///
/// `behavior.trap_awareness` is authored per hero — 0.9 for a scout, 0.3 for a
/// militiaman — and had never been read, so every hero walked into everything
/// with identical carelessness.
fn hero_spots_trap(hero_id: EntityId, entities: &EntityManager, game_data: &GameData) -> bool {
    let awareness = entities
        .get(hero_id)
        .and_then(|entity| match &entity.entity_type {
            EntityType::Hero(hero) => game_data.heroes.get(&hero.hero_id),
            _ => None,
        })
        .map(|data| data.behavior.trap_awareness.clamp(0.0, 1.0))
        .unwrap_or(0.0);

    awareness > 0.0 && macroquad_toolkit::rng::gen_range(0.0f32, 1.0) < awareness
}

/// Update cooldowns for all traps
fn update_trap_cooldowns(dungeon: &mut Dungeon, dt: f32) {
    for y in 0..dungeon.height {
        for x in 0..dungeon.width {
            let pos = TilePos::new(x as i32, y as i32);
            if let Some(tile) = dungeon.get_tile_mut(pos) {
                if let Some(trap) = tile.trap.as_mut() {
                    trap.cooldown = (trap.cooldown - dt).max(0.0);
                }
            }
        }
    }
}

/// Check if a trap at the given position is triggerable and return its info
fn get_triggerable_trap(
    dungeon: &Dungeon,
    pos: TilePos,
    hero_id: EntityId,
) -> Option<(TilePos, String, EntityId)> {
    let tile = dungeon.get_tile(pos)?;
    let trap = tile.trap.as_ref()?;

    if trap.active && trap.constructed && trap.cooldown <= 0.0 && !trap.triggered {
        Some((pos, trap.trap_type.clone(), hero_id))
    } else {
        None
    }
}

/// Trigger a specific trap at a position
fn trigger_trap(
    pos: TilePos,
    trap_type: &str,
    trap_data: &crate::data::traps::TrapData,
    triggering_entity: EntityId,
    entities: &mut EntityManager,
    dungeon: &mut Dungeon,
    game_data: &GameData,
    tending: f32,
) -> Option<TrapTriggerResult> {
    // Dispatched on what the trap *does*, not which trap it is. Adding a trap
    // to `traps.json` is content work: three finished trap sprites
    // (fire/gas/lightning) sat unreachable because reaching them meant editing
    // this match. Rooms went the same way — see `room_validator`'s task
    // families.
    let effects = &trap_data.effects;

    if effects.blocks_movement
        && dungeon
            .get_tile(pos)
            .is_some_and(|tile| tile.trap.as_ref().is_some_and(|trap| trap.locked))
    {
        // Doors bar the way; they have nothing to fire.
        return None;
    }

    if effects.alert_radius > 0.0 {
        trigger_alarm_trap(pos, trap_data, entities, dungeon, game_data);
        return None;
    }

    if effects.damage <= 0.0 {
        eprintln!(
            "Trap '{}' has no damage, alert radius or blocking effect — nothing to trigger",
            trap_type
        );
        return None;
    }

    if effects.area {
        trigger_area_trap(pos, trap_data, entities, dungeon, tending)
    } else {
        trigger_damage_trap(
            pos,
            trap_data,
            triggering_entity,
            entities,
            dungeon,
            tending,
        )
    }
}

fn trigger_damage_trap(
    pos: TilePos,
    trap_data: &crate::data::traps::TrapData,
    triggering_entity: EntityId,
    entities: &mut EntityManager,
    dungeon: &mut Dungeon,
    tending: f32,
) -> Option<TrapTriggerResult> {
    let damage = trap_data.effects.damage * tending;
    let entity = entities.get_mut(triggering_entity)?;
    let cooldown = trap_data.effects.cooldown.unwrap_or(5.0);

    apply_trap_damage(entity, damage);
    trace_log!(
        "traps",
        "{} triggered at {:?}! Dealt {} damage.",
        trap_data.name,
        pos,
        damage
    );
    set_trap_cooldown(dungeon, pos, cooldown);

    Some(TrapTriggerResult {
        trap_type: trap_data.id.clone(),
        damage_dealt: damage,
    })
}

/// Area-damage trap: hits everything within its radius, not just whoever
/// stepped on it.
fn trigger_area_trap(
    pos: TilePos,
    trap_data: &crate::data::traps::TrapData,
    entities: &mut EntityManager,
    dungeon: &mut Dungeon,
    tending: f32,
) -> Option<TrapTriggerResult> {
    let damage = trap_data.effects.damage * tending;
    let radius = if trap_data.effects.area {
        trap_data.effects.area_radius.unwrap_or(1.5)
    } else {
        trap_data.effects.single_radius.unwrap_or(0.5)
    };

    let affected_entities: Vec<EntityId> = entities
        .heroes()
        .filter_map(|(id, _)| entities.get(id).map(|e| (id, e.pos)))
        .filter(|(_, e_pos)| pos.distance_to(e_pos) <= radius)
        .map(|(id, _)| id)
        .collect();

    if affected_entities.is_empty() {
        return None;
    }

    let mut total_damage = 0.0;
    for entity_id in &affected_entities {
        if let Some(entity) = entities.get_mut(*entity_id) {
            apply_trap_damage(entity, damage);
            total_damage += damage;
        }
    }

    trace_log!(
        "traps",
        "Boulder trap triggered at {:?}! Dealt {} damage to {} entities.",
        pos,
        damage,
        affected_entities.len()
    );
    set_trap_disabled(dungeon, pos, trap_data.effects.cooldown.unwrap_or(5.0));

    Some(TrapTriggerResult {
        trap_type: "boulder_trap".to_string(),
        damage_dealt: total_damage,
    })
}

fn trigger_alarm_trap(
    pos: TilePos,
    trap_data: &crate::data::traps::TrapData,
    entities: &mut EntityManager,
    dungeon: &mut Dungeon,
    game_data: &GameData,
) {
    let alert_radius = trap_data.effects.alert_radius;

    let alerted_ids: Vec<EntityId> = entities
        .creatures()
        .filter_map(|(id, creature)| {
            let entity = entities.get(id)?;
            (entity.owner == OwnerId::Player
                && creature.creature_id != "imp"
                && pos.distance_to(&entity.pos) <= alert_radius)
                .then_some(id)
        })
        .collect();

    for entity_id in &alerted_ids {
        if let Some(entity) = entities.get_mut(*entity_id) {
            if let Some(creature) = entity.as_creature_mut() {
                creature.current_task = Some(crate::state::entities::Task::MoveTo(pos));
                creature.current_path = None;
            }
        }
    }

    trace_log!(
        "traps",
        "Alarm trap triggered at {:?}! Alerted {} creatures.",
        pos,
        alerted_ids.len()
    );
    let cooldown = game_data.config.traps.default_cooldown;
    let cooldown_multiplier = trap_data.effects.cooldown_multiplier.unwrap_or(2.0);
    set_trap_cooldown(dungeon, pos, cooldown * cooldown_multiplier);
}

fn set_trap_cooldown(dungeon: &mut Dungeon, pos: TilePos, cooldown: f32) {
    if let Some(tile) = dungeon.get_tile_mut(pos) {
        if let Some(trap) = tile.trap.as_mut() {
            trap.cooldown = cooldown;
        }
    }
}

fn set_trap_disabled(dungeon: &mut Dungeon, pos: TilePos, cooldown: f32) {
    if let Some(tile) = dungeon.get_tile_mut(pos) {
        if let Some(trap) = tile.trap.as_mut() {
            trap.triggered = true;
            trap.active = false;
            trap.cooldown = cooldown;
        }
    }
}

/// Imps can restore a spent area trap once its cooldown has elapsed.
fn rearm_triggered_traps(dungeon: &mut Dungeon, entities: &EntityManager, game_data: &GameData) {
    let imp_positions: Vec<TilePos> = entities
        .creatures()
        .filter_map(|(id, creature)| {
            let entity = entities.get(id)?;
            (entity.owner == OwnerId::Player && creature.creature_id == "imp").then_some(entity.pos)
        })
        .collect();
    if imp_positions.is_empty() {
        return;
    }

    for y in 0..dungeon.height {
        for x in 0..dungeon.width {
            let pos = TilePos::new(x as i32, y as i32);
            let Some(tile) = dungeon.get_tile_mut(pos) else {
                continue;
            };
            let Some(trap) = tile.trap.as_mut() else {
                continue;
            };
            let Some(data) = game_data.traps.get(&trap.trap_type) else {
                continue;
            };
            if !trap.triggered
                || trap.cooldown > 0.0
                || !data.effects.area
                || !imp_positions
                    .iter()
                    .any(|imp_pos| pos.distance_to(imp_pos) <= 1.5)
            {
                continue;
            }
            trap.triggered = false;
            trap.active = true;
            trace_log!("traps", "Imp rearmed {} at {:?}.", trap.trap_type, pos);
        }
    }
}

/// Apply trap damage to an entity
fn apply_trap_damage(entity: &mut crate::state::entities::Entity, damage: f32) {
    match &mut entity.entity_type {
        EntityType::Hero(hero) => {
            hero.health = (hero.health - damage).max(0.0);
            trace_log!(
                "traps",
                "Hero {} took {} trap damage (HP: {}/{})",
                hero.hero_id,
                damage,
                hero.health,
                hero.max_health
            );
        }
        EntityType::Creature(creature) => {
            creature.health = (creature.health - damage).max(0.0);
            trace_log!(
                "traps",
                "Creature {} took {} trap damage (HP: {}/{})",
                creature.creature_id,
                damage,
                creature.health,
                creature.max_health
            );
        }
        EntityType::Structure(_) => {
            // Structures don't take trap damage
        }
        EntityType::ResourcePile(_) => {} // Piles don't take trap damage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::entities::CreatureState;
    use crate::state::game_state::GameState;
    use crate::state::OwnerId;

    #[test]
    fn an_imp_rearms_a_spent_area_trap() {
        let game_data = GameData::load().expect("game data should load");
        let mut state = GameState::new(20, 20, &game_data);
        let pos = TilePos::new(4, 4);
        state.dungeon.get_tile_mut(pos).unwrap().trap = Some(crate::state::tile_state::TrapState {
            trap_type: "boulder_trap".to_string(),
            constructed: true,
            construction_progress: 1.0,
            active: false,
            locked: false,
            funded: true,
            cooldown: 0.0,
            triggered: true,
        });
        state.entities.spawn_creature_for_owner(
            pos,
            CreatureState::new("imp".to_string(), 1, 10.0, 0.0, 1),
            OwnerId::Player,
        );

        rearm_triggered_traps(&mut state.dungeon, &state.entities, &game_data);
        let trap = state.dungeon.get_tile(pos).unwrap().trap.as_ref().unwrap();
        assert!(trap.active);
        assert!(!trap.triggered);
    }

    #[test]
    fn an_alarm_sends_nearby_creatures_to_the_alarm_tile() {
        let game_data = GameData::load().expect("game data should load");
        let mut state = GameState::new(20, 20, &game_data);
        let pos = TilePos::new(4, 4);
        let creature_id = state.entities.spawn_creature(
            TilePos::new(5, 4),
            CreatureState::new("goblin".to_string(), 1, 20.0, 0.0, 1),
        );

        trigger_alarm_trap(
            pos,
            &game_data.traps["alarm_trap"],
            &mut state.entities,
            &mut state.dungeon,
            &game_data,
        );

        assert_eq!(
            state
                .entities
                .get(creature_id)
                .unwrap()
                .as_creature()
                .unwrap()
                .current_task,
            Some(crate::state::entities::Task::MoveTo(pos))
        );
    }
}
