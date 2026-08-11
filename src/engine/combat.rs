//! Combat resolution system
//!
//! Handles damage calculation, status effects, and combat outcomes
//! between creatures and heroes.

mod targeting;

pub use targeting::{
    find_combat_targets, get_detection_range, get_entity_attack_type, in_combat_range,
    manhattan_distance,
};

use crate::data::GameData;
use crate::state::entities::{Entity, EntityId, StatusEffect};
use crate::state::tile_state::TilePos;
use std::collections::HashMap;

/// Result of a combat tick
#[derive(Debug, Clone)]
pub struct CombatResult {
    pub damage_dealt: f32,
    pub status_applied: Vec<StatusEffect>,
    pub defender_died: bool,
    pub projectile_spawned: Option<(String, f32)>, // (type, damage)
}

/// Combat statistics for an entity
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub health: f32,
    pub attack: f32,
    pub defense: f32,
    pub attack_type: String,
    pub damage_range: [f32; 2],
    pub attack_speed: f32,
    pub resistances: HashMap<String, f32>,
    pub level: u32,
    pub abilities: Vec<String>,
}

/// Resolve one tick of combat between two entities
///
/// `defender_room_defense` is the `creature_defense_modifier` of the room the
/// defender is standing in (see `room_validator::room_defense_at`). It is
/// passed in rather than looked up here so combat stays a pure function of its
/// arguments, the same way `calculate_mood` takes its room modifier.
pub fn resolve_combat_tick(
    attacker: &Entity,
    defender: &Entity,
    dt: f32,
    game_data: &GameData,
    defender_room_defense: f32,
    hero_supply_penalty: (f32, f32),
) -> CombatResult {
    if is_stunned(attacker) {
        return CombatResult {
            damage_dealt: 0.0,
            status_applied: Vec::new(),
            defender_died: false,
            projectile_spawned: None,
        };
    }

    let (attack_penalty, defense_penalty) = hero_supply_penalty;

    let mut attacker_stats = extract_combat_stats(attacker, game_data);
    apply_hero_supply_penalty(
        attacker,
        &mut attacker_stats,
        attack_penalty,
        defense_penalty,
    );

    let mut defender_stats = extract_combat_stats(defender, game_data);
    apply_hero_supply_penalty(
        defender,
        &mut defender_stats,
        attack_penalty,
        defense_penalty,
    );
    defender_stats.defense =
        fortified_defense(defender, defender_stats.defense, defender_room_defense);

    // Calculate if attack hits this tick
    let attacks_per_second = attacker_stats.attack_speed;
    let attack_chance = attacks_per_second * dt;

    if macroquad_toolkit::rng::gen_range(0.0f32, 1.0) > attack_chance {
        // No attack this tick
        return CombatResult {
            damage_dealt: 0.0,
            status_applied: Vec::new(),
            defender_died: false,
            projectile_spawned: None,
        };
    }

    // Calculate damage
    let base_damage = calculate_damage(&attacker_stats, &defender_stats, game_data);
    let actual_damage = base_damage.max(0.0);

    // Determine if we should spawn a projectile or apply instant damage
    let is_melee = attacker_stats.attack_type == "melee";

    if !is_melee && actual_damage > 0.0 {
        // Ranged/Magic attack - spawn projectile. Status effects are rolled now (attacker
        // abilities are known now) but only actually applied once the projectile lands, in
        // apply_projectile_impact.
        return CombatResult {
            damage_dealt: 0.0, // Defer damage
            status_applied: generate_status_effects(&attacker_stats.abilities, game_data),
            defender_died: false,
            projectile_spawned: Some((attacker_stats.attack_type.clone(), actual_damage)),
        };
    }

    // Apply damage instantly (Melee)
    let defender_would_die = defender.is_alive() && (defender_stats.health - actual_damage) <= 0.0;

    let status_effects = generate_status_effects(&attacker_stats.abilities, game_data);

    // Log combat using all stats for debug
    if actual_damage > 0.0 {
        trace_log!(
            "combat",
            "Combat: {} (Lvl {}) hit {} (Lvl {}) for {:.1} damage (Type: {})",
            get_type_name(&attacker.entity_type),
            attacker_stats.level,
            get_type_name(&defender.entity_type),
            defender_stats.level,
            actual_damage,
            attacker_stats.attack_type
        );
    }

    CombatResult {
        damage_dealt: actual_damage,
        status_applied: status_effects,
        defender_died: defender_would_die,
        projectile_spawned: None,
    }
}

/// True if the entity has an active "stun" status effect (can't attack this tick).
fn is_stunned(entity: &Entity) -> bool {
    let status_effects = match &entity.entity_type {
        crate::state::entities::EntityType::Creature(state) => &state.status_effects,
        crate::state::entities::EntityType::Hero(state) => &state.status_effects,
        crate::state::entities::EntityType::Structure(_)
        | crate::state::entities::EntityType::ResourcePile(_) => return false,
    };
    status_effects.iter().any(|e| e.effect_type == "stun")
}

fn get_type_name(entity_type: &crate::state::entities::EntityType) -> String {
    match entity_type {
        crate::state::entities::EntityType::Creature(c) => c.creature_id.clone(),
        crate::state::entities::EntityType::Hero(h) => h.hero_id.clone(),
        crate::state::entities::EntityType::Structure(s) => s.building_id.clone(),
        crate::state::entities::EntityType::ResourcePile(_) => "gold_pile".to_string(),
    }
}

/// Extract combat stats from an entity
pub fn extract_combat_stats(entity: &Entity, game_data: &GameData) -> CombatStats {
    match &entity.entity_type {
        crate::state::entities::EntityType::Creature(creature_state) => {
            let creature_data = game_data
                .monsters
                .get(&creature_state.creature_id)
                .expect("Creature data not found");

            // Calculate level bonuses
            let level_multiplier = 1.0
                + (creature_state.level - 1) as f32
                    * game_data.config.combat.creature_level_multiplier;

            // Trait-driven attack/defense multipliers (data-driven; see traits.json).
            // Goes through the shared resolver so traits grafted onto this
            // individual count here too — this used to be a second copy that
            // only looked at the creature's *kind*.
            let trait_data = crate::engine::creature_ai::needs::creature_traits(
                creature_state,
                creature_data,
                game_data,
            );
            let attack_multiplier: f32 = trait_data.iter().map(|t| t.attack_multiplier).product();

            // Darkness bonus, scaled by how dark this creature's tile actually
            // is (cached by `lighting::cache_creature_darkness`). A Shadow
            // Stalker in a lit treasury is an ordinary skirmisher; the same
            // creature in an unlit corridor is not.
            let darkness_multiplier: f32 = 1.0
                + trait_data
                    .iter()
                    .map(|t| t.darkness_attack_bonus)
                    .sum::<f32>()
                    * creature_state.darkness;
            let defense_multiplier: f32 = trait_data.iter().map(|t| t.defense_multiplier).product();

            CombatStats {
                health: creature_state.health,
                attack: creature_data.stats.attack
                    * level_multiplier
                    * attack_multiplier
                    * darkness_multiplier,
                defense: creature_data.stats.defense * level_multiplier * defense_multiplier,
                attack_type: creature_data.combat.attack_type.clone(),
                damage_range: creature_data.combat.damage_range,
                attack_speed: creature_data.combat.attack_speed,
                resistances: creature_data.combat.resistances.clone(),
                level: creature_state.level,
                abilities: creature_data.combat.abilities.clone(),
            }
        }
        crate::state::entities::EntityType::Hero(hero_state) => {
            let hero_data = game_data
                .heroes
                .get(&hero_state.hero_id)
                .expect("Hero data not found");

            // Calculate level bonuses
            let level_multiplier =
                1.0 + (hero_state.level - 1) as f32 * game_data.config.combat.hero_level_multiplier;

            CombatStats {
                health: hero_state.health,
                attack: hero_data.stats.attack * level_multiplier,
                defense: hero_data.stats.defense * level_multiplier,
                attack_type: hero_data.combat.attack_type.clone(),
                damage_range: hero_data.combat.damage_range,
                attack_speed: hero_data.combat.attack_speed,
                resistances: hero_data.combat.resistances.clone(),
                level: hero_state.level,
                // Hero abilities (HeroAbilityData) have a richer trigger/effect shape than
                // simple on-hit procs and aren't wired into combat yet; see TODO.md.
                abilities: Vec::new(),
            }
        }
        crate::state::entities::EntityType::Structure(structure_state) => CombatStats {
            health: structure_state.health,
            attack: 0.0,
            defense: game_data.config.combat.building_base_defense,
            attack_type: "none".to_string(),
            damage_range: [0.0, 0.0],
            attack_speed: game_data.config.combat.building_attack_speed,
            resistances: HashMap::new(),
            level: 1,
            abilities: Vec::new(),
        },
        crate::state::entities::EntityType::ResourcePile(_) => CombatStats {
            health: 1.0,
            attack: 0.0,
            defense: 0.0,
            attack_type: "none".to_string(),
            damage_range: [0.0, 0.0],
            attack_speed: 1.0,
            resistances: HashMap::new(),
            level: 1,
            abilities: Vec::new(),
        },
    }
}

/// Attack and defence a hero has lost to razed hero buildings.
///
/// Applied to heroes only, and only after their level multiplier, so a burnt
/// armoury takes the same flat bite out of a veteran as a recruit. Creatures
/// and structures are unaffected — this is the hero faction's own supply
/// chain failing.
pub fn apply_hero_supply_penalty(
    entity: &Entity,
    stats: &mut CombatStats,
    attack_penalty: f32,
    defense_penalty: f32,
) {
    if !matches!(
        entity.entity_type,
        crate::state::entities::EntityType::Hero(_)
    ) {
        return;
    }
    stats.attack = (stats.attack - attack_penalty).max(0.0);
    stats.defense = (stats.defense - defense_penalty).max(0.0);
}

/// A defender's effective defence once the room they stand in is accounted for.
///
/// A fortified room protects the keeper's own creatures, and only those: a hero
/// who fights their way into a gatehouse gets no benefit from the keeper's
/// stonework, and structures have their own flat defence.
pub fn fortified_defense(defender: &Entity, defense: f32, room_defense: f32) -> f32 {
    match defender.entity_type {
        crate::state::entities::EntityType::Creature(_) => defense + room_defense,
        _ => defense,
    }
}

/// Calculate damage from attacker to defender
pub fn calculate_damage(
    attacker: &CombatStats,
    defender: &CombatStats,
    game_data: &GameData,
) -> f32 {
    // Base damage from attacker's range
    let base_damage = if attacker.damage_range[1] > attacker.damage_range[0] {
        let range = attacker.damage_range[1] - attacker.damage_range[0];
        attacker.damage_range[0] + macroquad_toolkit::rng::gen_range(0.0f32, 1.0) * range
    } else {
        attacker.damage_range[0]
    };

    // Add attack stat bonus
    let attack_damage = base_damage + attacker.attack * game_data.config.combat.attack_stat_bonus;

    // Apply defense reduction
    let defense_reduction = defender.defense * game_data.config.combat.defense_reduction;
    let pre_resist_damage = (attack_damage - defense_reduction).max(0.0);

    // Apply elemental resistances
    let resistance_multiplier = calculate_resistance_multiplier(attacker, defender);
    let final_damage = pre_resist_damage * resistance_multiplier;

    // Ensure minimum damage of 1.0 so battles don't stalemate
    final_damage.max(1.0)
}

/// Calculate resistance multiplier based on attack type and defender resistances
fn calculate_resistance_multiplier(attacker: &CombatStats, defender: &CombatStats) -> f32 {
    let base_resistance = defender
        .resistances
        .get(&attacker.attack_type)
        .copied()
        .unwrap_or(0.0);

    // Convert resistance percentage to multiplier
    // Positive resistance reduces damage, negative increases
    1.0 - (base_resistance / 100.0)
}

/// Roll each of the attacker's combat abilities against the data-driven
/// `game_data.config.status_effects.ability_effects` table, returning the status effects that
/// proc'd on this landed hit. Abilities with no entry in that table (e.g. ones that aren't a
/// poison/burn/freeze/stun proc, like a flat damage bonus) are silently skipped here.
fn generate_status_effects(abilities: &[String], game_data: &GameData) -> Vec<StatusEffect> {
    let mut effects = Vec::new();
    for ability in abilities {
        if let Some(ability_effect) = game_data.config.status_effects.ability_effects.get(ability) {
            if macroquad_toolkit::rng::gen_range(0.0f32, 1.0) < ability_effect.proc_chance {
                effects.push(StatusEffect {
                    effect_type: ability_effect.status_type.clone(),
                    duration: ability_effect.duration,
                    strength: ability_effect.strength,
                });
            }
        }
    }
    effects
}

/// Apply combat result to entities
pub fn apply_combat_result(
    result: &CombatResult,
    attacker_id: EntityId,
    defender_id: EntityId,
    entities: &mut HashMap<EntityId, Entity>,
    game_data: &GameData,
    current_time: f32,
) {
    // Apply damage to defender
    if let Some(defender) = entities.get_mut(&defender_id) {
        if result.damage_dealt > 0.0 {
            defender.last_damage_time = current_time;
        }
        match &mut defender.entity_type {
            crate::state::entities::EntityType::Creature(state) => {
                state.take_damage(result.damage_dealt);
            }
            crate::state::entities::EntityType::Hero(state) => {
                state.take_damage(result.damage_dealt);
            }
            crate::state::entities::EntityType::Structure(state) => {
                state.take_damage(result.damage_dealt);
            }
            crate::state::entities::EntityType::ResourcePile(_) => {}
        }
    }

    // Apply status effects to defender. "freeze" slows movement immediately on application;
    // combat::update_status_effects reverts the slow when the effect's duration runs out.
    if let Some(defender) = entities.get_mut(&defender_id) {
        for effect in &result.status_applied {
            match &mut defender.entity_type {
                crate::state::entities::EntityType::Creature(state) => {
                    if effect.effect_type == "freeze" && effect.strength != 0.0 {
                        state.movement_speed *= effect.strength;
                    }
                    state.status_effects.push(effect.clone());
                }
                crate::state::entities::EntityType::Hero(state) => {
                    if effect.effect_type == "freeze" && effect.strength != 0.0 {
                        state.movement_speed *= effect.strength;
                    }
                    state.status_effects.push(effect.clone());
                }
                crate::state::entities::EntityType::Structure(_) => {
                    // Structures don't take status effects yet
                }
                crate::state::entities::EntityType::ResourcePile(_) => {}
            }
        }
    }

    // Handle death and experience separately to avoid borrow issues
    let defender_died = result.defender_died;
    let victim_level = if defender_died {
        if let Some(defender) = entities.get(&defender_id) {
            match &defender.entity_type {
                crate::state::entities::EntityType::Creature(state) => state.level,
                crate::state::entities::EntityType::Hero(_) => 0, // Heroes don't give XP
                crate::state::entities::EntityType::Structure(_) => {
                    game_data.config.combat.building_xp_reward as u32
                }
                crate::state::entities::EntityType::ResourcePile(_) => 0,
            }
        } else {
            0
        }
    } else {
        0
    };

    // Award experience to attacker if defender died
    if defender_died && victim_level > 0 {
        if let Some(attacker) = entities.get_mut(&attacker_id) {
            award_experience(attacker, victim_level, game_data);
        }
    }
}

/// Award experience to attacker for killing a creature
fn award_experience(attacker: &mut Entity, victim_level: u32, game_data: &GameData) {
    let exp_gain = victim_level * game_data.config.combat.xp_per_victim_level as u32;

    match &mut attacker.entity_type {
        crate::state::entities::EntityType::Creature(state) => {
            state.experience += exp_gain as f32;

            // Check for level up
            // Use same max_experience field from CreatureState
            if state.experience >= state.max_experience {
                level_up_creature(state, game_data);
            }
        }
        crate::state::entities::EntityType::Hero(_state) => {
            // Heroes don't level up in combat in this simplified system
            // Could be extended to award hero XP
        }
        crate::state::entities::EntityType::Structure(_) => {}
        crate::state::entities::EntityType::ResourcePile(_) => {}
    }
}

/// Apply projectile impact
pub fn apply_projectile_impact(
    impact: &crate::state::projectiles::Impact,
    entities: &mut crate::state::entities::EntityManager,
    game_data: &GameData,
    current_time: f32,
) {
    // Apply damage to defender
    if let Some(defender) = entities.get_mut(impact.defender_id) {
        // Use apply_combat_result mechanics but simplified
        let damage = impact.damage;

        if damage > 0.0 {
            defender.last_damage_time = current_time;
        }

        match &mut defender.entity_type {
            crate::state::entities::EntityType::Creature(state) => {
                state.take_damage(damage);
            }
            crate::state::entities::EntityType::Hero(state) => {
                state.take_damage(damage);
            }
            crate::state::entities::EntityType::Structure(state) => {
                state.take_damage(damage);
            }
            crate::state::entities::EntityType::ResourcePile(_) => {}
        }
    }

    // XP Awarding needs to happen safely.
    // Check if defender died
    let defender_dead_and_level = if let Some(defender) = entities.get(impact.defender_id) {
        if !defender.is_alive() {
            match &defender.entity_type {
                crate::state::entities::EntityType::Creature(state) => Some(state.level),
                crate::state::entities::EntityType::Structure(_) => {
                    Some(game_data.config.combat.building_xp_reward as u32)
                }
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(level) = defender_dead_and_level {
        if let Some(attacker) = entities.get_mut(impact.attacker_id) {
            award_experience(attacker, level, game_data);
        }
    }
}

/// Level up a creature
fn level_up_creature(state: &mut crate::state::entities::CreatureState, game_data: &GameData) {
    if state.level >= game_data.config.combat.max_creature_level {
        return; // Max level cap
    }

    state.level += 1;
    state.experience = 0.0; // Reset for next level
    state.max_experience *= game_data.config.combat.xp_requirement_multiplier; // Scaling XP requirement

    // Increase stats (simplified - in full game would use progression data)
    state.max_health += game_data.config.combat.level_up_health_bonus;
    state.health = state.max_health; // Full heal on level up

    // Could also increase attack, defense, etc.
}

// Placeholder - will act validation in next step

/// Sum the poison/burn damage-per-second entries in a status effect list for this tick.
fn dot_damage(status_effects: &[StatusEffect], dt: f32) -> f32 {
    status_effects
        .iter()
        .filter(|e| e.effect_type == "poison" || e.effect_type == "burn")
        .map(|e| e.strength * dt)
        .sum()
}

/// Which movement-speed multipliers just expired and need to be divided back out.
fn expired_speed_multipliers(status_effects: &[StatusEffect]) -> Vec<f32> {
    status_effects
        .iter()
        .filter(|e| {
            e.duration <= 0.0
                && (e.effect_type == "speed_modifier" || e.effect_type == "freeze")
                && e.strength != 0.0
        })
        .map(|e| e.strength)
        .collect()
}

/// Update status effects on an entity: ticks duration down, applies poison/burn damage over
/// time, and reverts freeze/speed_modifier movement-speed changes once they expire. Stun has no
/// per-tick effect here; combat::resolve_combat_tick checks for it directly before an attack.
pub fn update_status_effects(entity: &mut Entity, dt: f32) {
    match &mut entity.entity_type {
        crate::state::entities::EntityType::Creature(state) => {
            let dot = dot_damage(&state.status_effects, dt);
            if dot > 0.0 {
                state.health = (state.health - dot).max(0.0);
            }

            for effect in &mut state.status_effects {
                effect.duration -= dt;
            }
            let expired = expired_speed_multipliers(&state.status_effects);
            state.status_effects.retain(|effect| effect.duration > 0.0);
            for multiplier in expired {
                state.movement_speed /= multiplier;
            }
        }
        crate::state::entities::EntityType::Hero(state) => {
            let dot = dot_damage(&state.status_effects, dt);
            if dot > 0.0 {
                state.health = (state.health - dot).max(0.0);
            }

            for effect in &mut state.status_effects {
                effect.duration -= dt;
            }
            let expired = expired_speed_multipliers(&state.status_effects);
            state.status_effects.retain(|effect| effect.duration > 0.0);
            for multiplier in expired {
                state.movement_speed /= multiplier;
            }
        }
        crate::state::entities::EntityType::Structure(_) => {}
        crate::state::entities::EntityType::ResourcePile(_) => {}
    }
}
