//! Data-driven hero ability execution.
//!
//! A hero ability (`data::heroes::HeroAbilityData`) pairs a `trigger` — a small, fixed
//! vocabulary evaluated generically below, by trigger *type*, never by ability id — with a list
//! of `effects` using the exact same schema and dispatcher spells use
//! (`data::spells::SpellEffect` / `spell_effects::apply_spell_effect`). Adding a new ability, or
//! changing what an existing one does, is a `heroes.json` edit. Only a genuinely new trigger
//! *type* needs an engine change here, the same way a genuinely new effect *type* needs one in
//! `spell_effects`.
//!
//! Recognized triggers, grouped by what they resolve to:
//! - self: "passive", "on_low_health" / "on_self_low_health" / "defensive",
//!   "on_hit" / "on_damaged"
//! - nearest ally below the low-health threshold: "on_ally_low_health" / "on_party_damaged"
//! - nearest hostile entity: "on_target" / "on_armored_target", "on_undead_nearby" (further
//!   filtered to a creature with the "undead" trait), "on_target_isolated" (exactly one hostile
//!   in range)
//! - nearest hostile's position, once enough are in range for an AoE to make sense:
//!   "on_multiple_targets" / "on_group" / "on_creature_nearby" / "on_open_area" (>=2),
//!   "on_large_group" (>=3)
//! - the hero's own tile/room: "in_room" (optionally "in_room:<room_type>" to require a
//!   specific room type)
//! - live dungeon conditions: "on_ritual_detected", "on_corruption",
//!   "on_corruption_detected", "on_sneak_attack", and "on_trapped"

use crate::data::GameData;
use crate::engine::spell_effects::apply_spell_effect;
use crate::state::entities::{Entity, EntityId, EntityType};
use crate::state::game_state::GameState;
use crate::state::tile_state::TilePos;

/// Passive abilities authored with cooldown 0 still get a floor so they don't reapply (and
/// stack) their effects every single simulation tick.
const MIN_PASSIVE_INTERVAL: f32 = 1.0;

fn manhattan_distance(a: TilePos, b: TilePos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

enum AbilityTarget {
    SelfTarget,
    Entity(EntityId),
    Position(TilePos),
}

/// Tick every hero's ability cooldowns and fire any ability whose trigger currently holds.
pub fn update_hero_abilities(game_state: &mut GameState, game_data: &GameData, dt: f32) {
    let hero_ids: Vec<EntityId> = game_state
        .entities
        .all()
        .filter(|e| matches!(e.entity_type, EntityType::Hero(_)))
        .map(|e| e.id)
        .collect();

    for hero_id in hero_ids {
        update_single_hero_abilities(hero_id, game_state, game_data, dt);
    }
}

fn update_single_hero_abilities(
    hero_id: EntityId,
    game_state: &mut GameState,
    game_data: &GameData,
    dt: f32,
) {
    let hero_data_id = match game_state.entities.get(hero_id).and_then(|e| e.as_hero()) {
        Some(hero) => hero.hero_id.clone(),
        None => return,
    };

    let hero_data = match game_data.heroes.get(&hero_data_id) {
        Some(data) if !data.abilities.is_empty() => data,
        _ => return,
    };

    if let Some(hero) = game_state
        .entities
        .get_mut(hero_id)
        .and_then(|e| e.as_hero_mut())
    {
        for remaining in hero.ability_cooldowns.values_mut() {
            *remaining -= dt;
        }
        hero.ability_cooldowns
            .retain(|_, remaining| *remaining > 0.0);
    }

    for ability in &hero_data.abilities {
        let ready = game_state
            .entities
            .get(hero_id)
            .and_then(|e| e.as_hero())
            .map(|hero| !hero.ability_cooldowns.contains_key(&ability.id))
            .unwrap_or(false);
        if !ready {
            continue;
        }

        let (target, hero_entity_pos) = match game_state.entities.get(hero_id) {
            Some(entity) => (
                evaluate_trigger(&ability.trigger, entity, game_state, game_data),
                entity.pos,
            ),
            None => return,
        };
        let Some(target) = target else {
            continue;
        };

        // Supply both target_pos and target_entity where we have them: apply_spell_effect
        // dispatches per effect_type (some need a position — reveal_map/tile_transform/
        // spawn_entity — others an entity — heal/stat_modifier/status_apply — and "damage"
        // accepts either), so a single resolved target can drive whichever effect an ability
        // author reaches for without this code needing to know which.
        let (target_pos, target_entity) = match target {
            AbilityTarget::SelfTarget => (Some(hero_entity_pos), Some(hero_id)),
            AbilityTarget::Entity(id) => {
                let pos = game_state.entities.get(id).map(|e| e.pos);
                (pos, Some(id))
            }
            AbilityTarget::Position(pos) => (Some(pos), None),
        };

        for effect in &ability.effects {
            apply_spell_effect(effect, game_state, game_data, target_pos, target_entity);
        }

        if let Some(hero) = game_state
            .entities
            .get_mut(hero_id)
            .and_then(|e| e.as_hero_mut())
        {
            hero.ability_cooldowns.insert(
                ability.id.clone(),
                ability.cooldown.max(MIN_PASSIVE_INTERVAL),
            );
        }
    }
}

fn evaluate_trigger(
    trigger: &str,
    hero_entity: &Entity,
    game_state: &GameState,
    game_data: &GameData,
) -> Option<AbilityTarget> {
    match trigger {
        "passive" => Some(AbilityTarget::SelfTarget),

        // "defensive" reads as "react when in danger" — same self-preservation condition as a
        // low-health rally, just a different flavor of ability.
        "on_low_health" | "on_self_low_health" | "defensive" => {
            let hero = hero_entity.as_hero()?;
            (hero.health / hero.max_health < game_data.config.hero_abilities.low_health_threshold)
                .then_some(AbilityTarget::SelfTarget)
        }

        "on_hit" | "on_damaged" => {
            let since_hit = game_state.time_elapsed - hero_entity.last_damage_time;
            (0.0..game_data.config.hero_abilities.recent_hit_window)
                .contains(&since_hit)
                .then_some(AbilityTarget::SelfTarget)
        }

        "on_ally_low_health" | "on_party_damaged" => {
            nearby_low_health_ally(hero_entity, game_state, game_data).map(AbilityTarget::Entity)
        }

        // A precise, single-target proc: nearest hostile entity in range.
        "on_target" | "on_armored_target" => nearby_enemies(hero_entity, game_state, game_data)
            .into_iter()
            .next()
            .map(AbilityTarget::Entity),

        // "isolated" = exactly one hostile entity in range (no others to have picked instead).
        "on_target_isolated" => {
            let enemies = nearby_enemies(hero_entity, game_state, game_data);
            (enemies.len() == 1).then(|| AbilityTarget::Entity(enemies[0]))
        }

        // Area-flavored triggers: fire at the nearest enemy's position once enough hostiles are
        // in range for an AoE effect to make sense (thresholds vary by how "grouped" the
        // trigger name implies).
        "on_multiple_targets" | "on_group" | "on_creature_nearby" | "on_open_area" => {
            area_target(hero_entity, game_state, game_data, 2)
        }
        "on_large_group" => area_target(hero_entity, game_state, game_data, 3),

        "on_undead_nearby" => nearby_enemies(hero_entity, game_state, game_data)
            .into_iter()
            .find(|&id| is_undead_creature(id, game_state, game_data))
            .map(AbilityTarget::Entity),

        // A ritual is detectable from nearby tiles, so the ability can fire
        // before the hero is standing inside the circle itself.
        "on_ritual_detected" => {
            ritual_nearby(hero_entity, game_state, game_data).map(|_| AbilityTarget::SelfTarget)
        }

        // Corruption is a tile condition. Purification is self-targeted while
        // mass cleansing is also gated by the same nearby scan.
        "on_corruption" => {
            corrupted_at(hero_entity.pos, game_state).then_some(AbilityTarget::SelfTarget)
        }
        "on_corruption_detected" => corrupted_nearby(hero_entity, game_state, game_data)
            .then_some(AbilityTarget::SelfTarget),

        // A rogue gets one opening strike against a nearby hostile while it has
        // not been hit recently. The cooldown prevents repeated procs.
        "on_sneak_attack" => {
            let recently_hit = (0.0..game_data.config.hero_abilities.recent_hit_window)
                .contains(&(game_state.time_elapsed - hero_entity.last_damage_time));
            (!recently_hit)
                .then(|| {
                    nearby_enemies(hero_entity, game_state, game_data)
                        .into_iter()
                        .next()
                })
                .flatten()
                .map(AbilityTarget::Entity)
        }

        // Trap awareness is evaluated before trap triggering in the fixed
        // timestep, allowing the wizard's authored speed escape to matter.
        "on_trapped" => {
            let trapped = game_state
                .dungeon
                .get_tile(hero_entity.pos)
                .and_then(|tile| tile.trap.as_ref())
                .is_some_and(|trap| trap.constructed && trap.active && trap.cooldown <= 0.0);
            trapped.then_some(AbilityTarget::SelfTarget)
        }

        // Room-scoped abilities act on the hero's current tile/room, not the hero itself.
        trigger if trigger == "in_room" || trigger.starts_with("in_room:") => {
            let room_type_filter = trigger.strip_prefix("in_room:");
            let in_matching_room = game_state.room_manager.rooms.iter().any(|room| {
                room.active
                    && room.tiles.contains(&hero_entity.pos)
                    && room_type_filter.is_none_or(|t| room.room_type == t)
            });
            in_matching_room.then_some(AbilityTarget::Position(hero_entity.pos))
        }

        _ => None,
    }
}

fn ritual_nearby(
    hero_entity: &Entity,
    game_state: &GameState,
    game_data: &GameData,
) -> Option<TilePos> {
    game_state
        .room_manager
        .rooms
        .iter()
        .filter(|room| room.active && room.room_type == "ritual_circle")
        .filter(|room| {
            manhattan_distance(hero_entity.pos, room.get_center())
                <= game_data.config.hero_abilities.scan_radius
        })
        .min_by_key(|room| manhattan_distance(hero_entity.pos, room.get_center()))
        .map(|room| room.get_center())
}

fn corrupted_at(pos: TilePos, game_state: &GameState) -> bool {
    game_state
        .dungeon
        .get_tile(pos)
        .is_some_and(|tile| tile.tile_type == "corrupted_floor")
}

fn corrupted_nearby(hero_entity: &Entity, game_state: &GameState, game_data: &GameData) -> bool {
    game_state
        .dungeon
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .any(|tile| {
            tile.tile_type == "corrupted_floor"
                && manhattan_distance(hero_entity.pos, tile.pos)
                    <= game_data.config.hero_abilities.scan_radius
        })
}

/// Nearest enemy's position, if at least `min_count` hostiles are within scan range.
fn area_target(
    hero_entity: &Entity,
    game_state: &GameState,
    game_data: &GameData,
    min_count: usize,
) -> Option<AbilityTarget> {
    let enemies = nearby_enemies(hero_entity, game_state, game_data);
    if enemies.len() < min_count {
        return None;
    }
    game_state
        .entities
        .get(enemies[0])
        .map(|e| AbilityTarget::Position(e.pos))
}

/// Nearest same-owner hero below the low-health threshold within scan range, if any.
fn nearby_low_health_ally(
    hero_entity: &Entity,
    game_state: &GameState,
    game_data: &GameData,
) -> Option<EntityId> {
    game_state
        .entities
        .all()
        .filter(|e| e.id != hero_entity.id)
        .filter(|e| e.owner == hero_entity.owner)
        .filter(|e| {
            manhattan_distance(hero_entity.pos, e.pos)
                <= game_data.config.hero_abilities.scan_radius
        })
        .filter_map(|e| e.as_hero().map(|h| (e.id, h.health / h.max_health)))
        .filter(|(_, health_pct)| {
            *health_pct < game_data.config.hero_abilities.low_health_threshold
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(id, _)| id)
}

/// Hostile, living entities within scan range, nearest first.
fn nearby_enemies(
    hero_entity: &Entity,
    game_state: &GameState,
    game_data: &GameData,
) -> Vec<EntityId> {
    let mut enemies: Vec<(EntityId, i32)> = game_state
        .entities
        .all()
        .filter(|e| e.id != hero_entity.id)
        .filter(|e| e.is_alive())
        .filter(|e| hero_entity.owner.is_hostile_to(&e.owner))
        .map(|e| (e.id, manhattan_distance(hero_entity.pos, e.pos)))
        .filter(|(_, dist)| *dist <= game_data.config.hero_abilities.scan_radius)
        .collect();
    enemies.sort_by_key(|(_, dist)| *dist);
    enemies.into_iter().map(|(id, _)| id).collect()
}

fn is_undead_creature(entity_id: EntityId, game_state: &GameState, game_data: &GameData) -> bool {
    match game_state.entities.get(entity_id).map(|e| &e.entity_type) {
        Some(EntityType::Creature(creature)) => game_data
            .monsters
            .get(&creature.creature_id)
            .map(|m| m.traits.iter().any(|t| t == "undead"))
            .unwrap_or(false),
        _ => false,
    }
}
