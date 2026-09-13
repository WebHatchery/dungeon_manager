//! Authored mana upkeep for creatures that are bound to the dungeon's power.

use crate::data::GameData;
use crate::state::entities::EntityManager;
use crate::state::player_state::PlayerState;

/// Drain the fractional mana cost of all player creatures for this tick.
///
/// The accumulator preserves sub-point costs between ticks. A depleted pool
/// never becomes negative; it simply stops paying until the dungeon produces
/// more mana. The warning is keyed so an expensive roster cannot flood the
/// notification queue every simulation tick.
pub fn apply_mana_upkeep(
    entities: &EntityManager,
    player: &mut PlayerState,
    game_data: &GameData,
    dt: f32,
) -> f32 {
    let drain_per_minute: f32 = entities
        .all()
        .filter(|entity| entity.owner.is_player_controlled())
        .filter_map(|entity| {
            let creature = entity.as_creature()?;
            (creature.health > 0.0).then_some(creature)
        })
        .filter_map(|creature| {
            let data = game_data.monsters.get(&creature.creature_id)?;
            (data.economy.mana_upkeep_per_minute > 0.0)
                .then_some(data.economy.mana_upkeep_per_minute)
        })
        .sum();
    let drain = drain_per_minute * dt / 60.0;
    if drain <= 0.0 {
        return 0.0;
    }

    player.accumulators.mana -= drain;
    let whole_drain = player.accumulators.mana.trunc() as i32;
    player.accumulators.mana -= whole_drain as f32;
    if whole_drain < 0 {
        player.mana = (player.mana + whole_drain).max(0);
    }

    if player.mana == 0 {
        player.accumulators.mana = 0.0;
        player.warn_once(
            "mana_upkeep_shortfall",
            "The dungeon lacks mana to sustain its bound creatures.",
        );
    } else {
        player.clear_warning("mana_upkeep_shortfall");
    }

    drain
}
