//! Spell effects system
//! Handles casting and applying spell effects to the game state

use crate::data::spells::{SpellData, SpellEffect};
use crate::data::GameData;
use crate::state::entities::{CreatureState, EntityId};
use crate::state::game_state::GameState;
use crate::state::tile_state::{Ownership, TilePos};

/// Result of a spell cast attempt
#[derive(Debug, Clone)]
pub enum CastResult {
    Success,
    InsufficientMana,
    InsufficientGold,
    InvalidTarget,
    OnCooldown,
    OutOfRange,
    MaxCapReached,
    /// The spell declares `requires_visibility` and the target is fogged.
    NotVisible,
    /// The target's allegiance is not in the spell's `valid_targets`.
    WrongAllegiance,
}

/// Whether `pos` is currently visible to the player.
///
/// Mirrors the renderer's condition so a spell cannot be refused for fog the
/// player cannot see. With fog disabled everything counts as visible.
fn target_is_visible(game_state: &GameState, game_data: &GameData, pos: TilePos) -> bool {
    let fog_enabled = game_data.config.fog_of_war.enabled && game_state.cheat_fog_enabled;
    if !fog_enabled {
        return true;
    }

    game_state
        .get_tile(pos)
        .map(|tile| tile.fog_state != crate::state::tile_state::FogState::Hidden)
        .unwrap_or(false)
}

/// Whether an entity's allegiance satisfies a spell's `valid_targets`.
///
/// Authored on 13 of 17 spells and read by nothing, so `heal` — declared
/// `["friendly"]` — could be cast on an invading knight, and `chickenify`
/// (`["enemy"]`) on one of your own goblins. Enforced for `creature`-targeted
/// spells, where there is exactly one definite target; area spells still let
/// their effects do the faction filtering, because refusing to place a
/// fireball on an empty tile would stop you catching someone standing beside
/// it.
fn allegiance_matches(valid_targets: &[String], owner: &crate::state::OwnerId) -> bool {
    if valid_targets.is_empty() {
        return true;
    }

    let is_players = *owner == crate::state::OwnerId::Player;
    valid_targets
        .iter()
        .any(|category| match category.as_str() {
            "friendly" | "ally" => is_players,
            "enemy" => !is_players,
            // "empty" describes a tile with no occupant, so it can never be
            // satisfied by an entity.
            _ => false,
        })
}

/// Check if a spell can be cast
pub fn can_cast_spell(
    spell: &SpellData,
    game_state: &GameState,
    game_data: &GameData,
    target_pos: Option<TilePos>,
    target_entity: Option<EntityId>,
) -> CastResult {
    // Check cooldown
    if game_state.player.spell_cooldowns.contains_key(&spell.id) {
        return CastResult::OnCooldown;
    }

    // Check mana cost
    if game_state.player.mana < spell.cost.mana {
        return CastResult::InsufficientMana;
    }

    // Check gold cost
    if game_state.player.gold < spell.cost.gold {
        return CastResult::InsufficientGold;
    }

    // Special handling for summon_imps - check max cap and calculate dynamic cost
    if spell.id == "summon_imps" {
        let current_imps = game_state.count_imps();
        let max_imps = GameState::max_imps(game_data);
        if current_imps >= max_imps {
            return CastResult::MaxCapReached;
        }
        // Dynamic cost: base + per-imp cost (read from monster data)
        let (base_cost, per_imp_cost) = game_data
            .monsters
            .get("imp")
            .map(|m| {
                (
                    m.spawn.summon_base_cost.unwrap_or(10),
                    m.spawn.summon_cost_per_existing.unwrap_or(5),
                )
            })
            .unwrap_or((10, 5));
        let dynamic_cost = base_cost + (current_imps as i32 * per_imp_cost);
        if game_state.player.mana < dynamic_cost {
            return CastResult::InsufficientMana;
        }
    }

    // Check targeting validity and range
    match spell.targeting.target_type.as_str() {
        "tile" | "area" => {
            if target_pos.is_none() {
                return CastResult::InvalidTarget;
            }
            // Check range if specified
            // Start check
            if let Some(pos) = target_pos {
                // Must be claimed by player
                let tile_owned = game_state
                    .get_tile(pos)
                    .map(|t| t.ownership == Ownership::Player)
                    .unwrap_or(false);
                if !tile_owned {
                    return CastResult::InvalidTarget;
                }

                if spell.targeting.requires_visibility
                    && !target_is_visible(game_state, game_data, pos)
                {
                    return CastResult::NotVisible;
                }

                if let Some(heart_pos) = game_state.find_dungeon_heart_position() {
                    let distance =
                        ((pos.x - heart_pos.x).abs() + (pos.y - heart_pos.y).abs()) as u32;
                    if spell.targeting.range > 0 && distance > spell.targeting.range {
                        return CastResult::OutOfRange;
                    }
                }
            }
        }
        "creature" => {
            let Some(entity_id) = target_entity else {
                return CastResult::InvalidTarget;
            };
            let Some(entity) = game_state.entities.get(entity_id) else {
                return CastResult::InvalidTarget;
            };
            if !allegiance_matches(&spell.targeting.valid_targets, &entity.owner) {
                return CastResult::WrongAllegiance;
            }
            if spell.targeting.requires_visibility
                && !target_is_visible(game_state, game_data, entity.pos)
            {
                return CastResult::NotVisible;
            }
        }
        "room" => {
            if target_pos.is_none() {
                return CastResult::InvalidTarget;
            }
        }
        "global" => {
            // No target needed
        }
        _ => {}
    }

    CastResult::Success
}

/// Cast a spell and apply its effects
pub fn cast_spell(
    spell_id: &str,
    game_state: &mut GameState,
    game_data: &GameData,
    target_pos: Option<TilePos>,
    target_entity: Option<EntityId>,
) -> CastResult {
    let spell = match game_data.spells.get(spell_id) {
        Some(s) => s,
        None => return CastResult::InvalidTarget,
    };

    // Check if cast is valid
    let can_cast = can_cast_spell(spell, game_state, game_data, target_pos, target_entity);
    if !matches!(can_cast, CastResult::Success) {
        return can_cast;
    }

    // Deduct costs from spell data
    game_state.player.mana -= spell.cost.mana;
    game_state.player.gold -= spell.cost.gold;
    trace_log!(
        "spells",
        "Cast spell: {} (mana: {}, gold: {})",
        spell.name,
        spell.cost.mana,
        spell.cost.gold
    );

    // Apply effects
    for effect in &spell.effects {
        apply_spell_effect(effect, game_state, game_data, target_pos, target_entity);
    }

    // Set cooldown
    if spell.cooldown > 0.0 {
        game_state
            .player
            .spell_cooldowns
            .insert(spell_id.to_string(), spell.cooldown);
    }

    CastResult::Success
}

/// Apply a single spell effect. `pub(crate)` so `engine::hero_abilities` can reuse the exact
/// same generic effect vocabulary for data-driven hero abilities instead of duplicating it.
pub(crate) fn apply_spell_effect(
    effect: &SpellEffect,
    game_state: &mut GameState,
    game_data: &GameData,
    target_pos: Option<TilePos>,
    target_entity: Option<EntityId>,
) {
    match effect.effect_type.as_str() {
        "damage" => {
            if let Some(entity_id) = target_entity {
                apply_damage_effect(entity_id, effect, game_state);
            } else if let Some(pos) = target_pos {
                // Area damage - find all entities in range (radius 1 default)
                let area_radius = 1;
                let entities_in_area: Vec<EntityId> = game_state
                    .entities
                    .all()
                    .filter(|e| {
                        let dist = ((e.pos.x - pos.x).pow(2) + (e.pos.y - pos.y).pow(2)) as f32;
                        dist.sqrt() <= area_radius as f32
                    })
                    .map(|e| e.id)
                    .collect();

                for entity_id in entities_in_area {
                    apply_damage_effect(entity_id, effect, game_state);
                }
            }
        }
        "heal" => {
            if let Some(entity_id) = target_entity {
                apply_heal_effect(entity_id, effect, game_state);
            }
        }
        "stat_modifier" => {
            if let Some(entity_id) = target_entity {
                apply_stat_modifier(entity_id, effect, game_state);
            }
        }
        "status_apply" => {
            if let Some(entity_id) = target_entity {
                apply_status_effect(entity_id, effect, game_state);
            }
        }
        "tile_transform" => {
            if let Some(pos) = target_pos {
                apply_tile_transform(pos, effect, game_state, game_data);
            }
        }
        "spawn_entity" => {
            if let Some(pos) = target_pos {
                spawn_entity_effect(pos, effect, game_state, game_data);
            }
        }
        "reveal_map" => {
            if let Some(pos) = target_pos {
                reveal_map_effect(pos, effect, game_state);
            }
        }
        _ => {
            eprintln!("Unknown spell effect type: {}", effect.effect_type);
        }
    }
}

/// Apply damage to an entity
fn apply_damage_effect(entity_id: EntityId, effect: &SpellEffect, game_state: &mut GameState) {
    if let Some(entity) = game_state.entities.get_mut(entity_id) {
        let damage = effect.amount;

        match &mut entity.entity_type {
            crate::state::entities::EntityType::Creature(creature) => {
                creature.health = (creature.health - damage).max(0.0);
                trace_log!(
                    "spells",
                    "Spell damage: {} took {} damage (HP: {}/{})",
                    creature.creature_id,
                    damage,
                    creature.health,
                    creature.max_health
                );
            }
            crate::state::entities::EntityType::Hero(hero) => {
                hero.health = (hero.health - damage).max(0.0);
                trace_log!(
                    "spells",
                    "Spell damage: {} took {} damage (HP: {}/{})",
                    hero.hero_id,
                    damage,
                    hero.health,
                    hero.max_health
                );
            }
            crate::state::entities::EntityType::Structure(structure) => {
                structure.take_damage(damage);
                trace_log!(
                    "spells",
                    "Spell damage: {} took {} damage (HP: {}/{})",
                    structure.building_id,
                    damage,
                    structure.health,
                    structure.max_health
                );
            }
            crate::state::entities::EntityType::ResourcePile(_) => {}
        }
    }
}

/// Apply healing to an entity
fn apply_heal_effect(entity_id: EntityId, effect: &SpellEffect, game_state: &mut GameState) {
    if let Some(entity) = game_state.entities.get_mut(entity_id) {
        let heal_amount = effect.amount;

        match &mut entity.entity_type {
            crate::state::entities::EntityType::Creature(creature) => {
                creature.health = (creature.health + heal_amount).min(creature.max_health);
                trace_log!(
                    "spells",
                    "Spell heal: {} healed {} HP (HP: {}/{})",
                    creature.creature_id,
                    heal_amount,
                    creature.health,
                    creature.max_health
                );
            }
            crate::state::entities::EntityType::Hero(hero) => {
                hero.health = (hero.health + heal_amount).min(hero.max_health);
                trace_log!(
                    "spells",
                    "Spell heal: {} healed {} HP (HP: {}/{})",
                    hero.hero_id,
                    heal_amount,
                    hero.health,
                    hero.max_health
                );
            }
            crate::state::entities::EntityType::Structure(structure) => {
                structure.health = (structure.health + heal_amount).min(structure.max_health);
                trace_log!(
                    "spells",
                    "Spell heal: {} healed {} HP (HP: {}/{})",
                    structure.building_id,
                    heal_amount,
                    structure.health,
                    structure.max_health
                );
            }
            crate::state::entities::EntityType::ResourcePile(_) => {} // Cannot heal a pile
        }
    }
}

/// Apply stat modifier (e.g., speed boost). CreatureState only tracks `movement_speed` as a
/// mutable runtime stat today, so "speed" is the only supported `effect.stat` value; other stats
/// (attack/defense/etc.) are computed from base data + level at combat time and have no mutable
/// field to modify without a broader data-model change.
fn apply_stat_modifier(entity_id: EntityId, effect: &SpellEffect, game_state: &mut GameState) {
    if effect.stat.as_deref() != Some("speed") {
        return;
    }
    let multiplier = effect.multiplier.unwrap_or(1.0);

    let Some(entity) = game_state.entities.get_mut(entity_id) else {
        return;
    };

    // Temporary buffs revert when their status effect expires (see
    // combat::update_status_effects); permanent buffs omit `duration`.
    match &mut entity.entity_type {
        crate::state::entities::EntityType::Creature(creature) => {
            creature.movement_speed *= multiplier;
            if let Some(duration) = effect.duration {
                creature
                    .status_effects
                    .push(crate::state::entities::StatusEffect {
                        effect_type: "speed_modifier".to_string(),
                        duration,
                        strength: multiplier,
                    });
            }
        }
        crate::state::entities::EntityType::Hero(hero) => {
            hero.movement_speed *= multiplier;
            if let Some(duration) = effect.duration {
                hero.status_effects
                    .push(crate::state::entities::StatusEffect {
                        effect_type: "speed_modifier".to_string(),
                        duration,
                        strength: multiplier,
                    });
            }
        }
        _ => {}
    }
}

/// Apply status effect
fn apply_status_effect(entity_id: EntityId, effect: &SpellEffect, game_state: &mut GameState) {
    if let Some(entity) = game_state.entities.get_mut(entity_id) {
        if let Some(status) = &effect.status {
            let duration = effect.duration.unwrap_or(10.0);
            let strength = if effect.amount > 0.0 {
                effect.amount
            } else {
                1.0
            };

            match &mut entity.entity_type {
                crate::state::entities::EntityType::Creature(creature) => {
                    // "freeze" slows movement immediately on application; combat's
                    // update_status_effects reverts it when the effect expires.
                    if status == "freeze" && strength != 0.0 {
                        creature.movement_speed *= strength;
                    }
                    creature
                        .status_effects
                        .push(crate::state::entities::StatusEffect {
                            effect_type: status.clone(),
                            duration,
                            strength,
                        });
                    trace_log!(
                        "spells",
                        "Status applied: {} gained '{}' for {} seconds",
                        creature.creature_id,
                        status,
                        duration
                    );
                }
                crate::state::entities::EntityType::Hero(hero) => {
                    if status == "freeze" && strength != 0.0 {
                        hero.movement_speed *= strength;
                    }
                    hero.status_effects
                        .push(crate::state::entities::StatusEffect {
                            effect_type: status.clone(),
                            duration,
                            strength,
                        });
                    trace_log!(
                        "spells",
                        "Status applied: {} gained '{}' for {} seconds",
                        hero.hero_id,
                        status,
                        duration
                    );
                }
                crate::state::entities::EntityType::Structure(_) => {}
                crate::state::entities::EntityType::ResourcePile(_) => {}
            }
        }
    }
}

/// Transform tile type
fn apply_tile_transform(
    pos: TilePos,
    effect: &SpellEffect,
    game_state: &mut GameState,
    game_data: &GameData,
) {
    let area_radius = effect.radius.unwrap_or(0); // Default to single tile if not specified

    for dy in -(area_radius as i32)..=(area_radius as i32) {
        for dx in -(area_radius as i32)..=(area_radius as i32) {
            let target_pos = TilePos::new(pos.x + dx, pos.y + dy);

            if let Some(tile) = game_state.get_tile_mut(target_pos) {
                // Check if tile matches 'from' type (if specified)
                if let Some(from_type) = &effect.from_tile {
                    if tile.tile_type != *from_type {
                        continue;
                    }
                }

                // Transform to 'to' type
                if let Some(to_type) = &effect.to_tile {
                    tile.tile_type = to_type.clone();

                    // If the new tile type is not claimable (e.g. earth, rock), reset ownership
                    // This ensures player-owned floor transformed to earth becomes neutral/diggable
                    if !crate::engine::tile_types::is_claimable(to_type, game_data) {
                        tile.ownership = Ownership::Unclaimed;
                        tile.room_id = None;
                        tile.marked_for_dig = false;
                    }

                    trace_log!(
                        "spells",
                        "Tile transformed at {:?} to {}",
                        target_pos,
                        to_type
                    );
                }
            }
        }
    }
}

/// Spawn entity effect
fn spawn_entity_effect(
    pos: TilePos,
    effect: &SpellEffect,
    game_state: &mut GameState,
    game_data: &GameData,
) {
    let Some(entity_type) = &effect.entity else {
        return;
    };

    // Imps are capped separately from other creatures (economy balance)
    if entity_type == "imp" {
        let max_imps = GameState::max_imps(game_data);
        if game_state.count_imps() >= max_imps {
            trace_log!(
                "spells",
                "Cannot summon imp: max cap of {} reached",
                max_imps
            );
            return;
        }
    }

    let Some(monster_data) = game_data.monsters.get(entity_type.as_str()) else {
        eprintln!("Cannot summon unknown entity type: {}", entity_type);
        return;
    };

    let visual_seed = macroquad_toolkit::rng::random_u64();
    let creature_state = CreatureState::new(
        entity_type.clone(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        visual_seed,
    );

    game_state.entities.spawn_creature(pos, creature_state);

    trace_log!("spells", "Spell summoned {} at {:?}", entity_type, pos);
}

/// Reveal map effect
fn reveal_map_effect(center: TilePos, _effect: &SpellEffect, game_state: &mut GameState) {
    let radius = 8; // Default reveal radius

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let target_pos = TilePos::new(center.x + dx, center.y + dy);

            if let Some(tile) = game_state.get_tile_mut(target_pos) {
                tile.fog_state = crate::state::tile_state::FogState::Visible;
            }
        }
    }

    trace_log!(
        "spells",
        "Revealed map around {:?} with radius {}",
        center,
        radius
    );
}

/// Update spell cooldowns (call once per second)
pub fn update_spell_cooldowns(game_state: &mut GameState, dt: f32) {
    let mut completed_cooldowns = Vec::new();

    for (spell_id, remaining_time) in game_state.player.spell_cooldowns.iter_mut() {
        *remaining_time -= dt;
        if *remaining_time <= 0.0 {
            completed_cooldowns.push(spell_id.clone());
        }
    }

    for spell_id in completed_cooldowns {
        game_state.player.spell_cooldowns.remove(&spell_id);
    }
}

#[cfg(test)]
mod tests;
