//! Tutorial step progression
//!
//! Each step is a predicate over existing game state, checked once per tick.
//! No events need to be wired: the tutorial simply observes what the player
//! has already accomplished.

use crate::data::tutorial::{TutorialCompletion, TutorialStepData};
use crate::data::GameData;
use crate::state::game_state::GameState;
use crate::state::tile_state::Ownership;
use crate::state::OwnerId;

/// The step the player is currently on, if the tutorial is running.
pub fn current_step<'a>(
    state: &GameState,
    game_data: &'a GameData,
) -> Option<&'a TutorialStepData> {
    if !state.tutorial.enabled || state.tutorial.complete {
        return None;
    }
    game_data.tutorial.steps.get(state.tutorial.step_index)
}

/// Progress counter (current, target) for count-based steps.
pub fn step_progress(state: &GameState, game_data: &GameData) -> Option<(usize, usize)> {
    let step = game_data.tutorial.steps.get(state.tutorial.step_index)?;
    match step.completion {
        TutorialCompletion::Dig { target } => Some((dig_progress(state).min(target), target)),
        TutorialCompletion::Claim { target } => {
            Some((claimed_tiles(state).min(target), target))
        }
        _ => None,
    }
}

/// Scenario intro lines to show before the game starts, if any remain unseen.
pub fn pending_intro<'a>(state: &GameState, game_data: &'a GameData) -> Option<&'a [String]> {
    if !state.tutorial.enabled || state.tutorial.intro_dismissed {
        return None;
    }
    let scenario_id = state.active_scenario_id.as_deref()?;
    let intro = &game_data.scenarios.get(scenario_id)?.meta.intro;
    if intro.is_empty() {
        None
    } else {
        Some(intro)
    }
}

/// Advance the tutorial when the current step's condition is met.
pub fn update_tutorial(state: &mut GameState, game_data: &GameData) {
    if !state.tutorial.enabled || state.tutorial.complete {
        return;
    }

    let Some(step) = game_data.tutorial.steps.get(state.tutorial.step_index) else {
        state.tutorial.complete = true;
        return;
    };
    let done = completion_met(&step.completion, state, game_data);

    if !done {
        return;
    }

    state
        .notifications
        .success(format!("Objective complete: {}", step.title));

    state.tutorial.step_index += 1;
    if state.tutorial.step_index >= game_data.tutorial.steps.len() {
        state.tutorial.complete = true;
        state
            .notifications
            .success("Tutorial complete. Your dungeon awaits, Keeper!");
    }
}

fn completion_met(
    completion: &TutorialCompletion,
    state: &GameState,
    game_data: &GameData,
) -> bool {
    match completion {
        TutorialCompletion::Dig { target } => dig_progress(state) >= *target,
        TutorialCompletion::Claim { target } => claimed_tiles(state) >= *target,
        TutorialCompletion::Room { room } => has_room(state, room),
        TutorialCompletion::Recruit => has_recruited_creature(state),
        TutorialCompletion::Combat => state.player.kills.values().sum::<u32>() > 0,
        TutorialCompletion::Trap => state
            .dungeon
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .any(|tile| tile.trap.as_ref().is_some_and(|trap| trap.triggered)),
        TutorialCompletion::Payday => {
            state.time_elapsed >= game_data.config.timing.pay_day_interval
        }
        TutorialCompletion::Research => !state.player.completed_technologies.is_empty(),
        TutorialCompletion::Wave => state.hero_base.current_wave_number > 0,
        TutorialCompletion::Spell => !state.player.spells_cast.is_empty(),
        TutorialCompletion::Conversion => state.conversion_count > 0,
        TutorialCompletion::Temple => has_room(state, "temple"),
    }
}

/// Tiles marked for digging plus tiles already excavated count toward the
/// dig objective, so fast imps can't undercut the counter.
fn dig_progress(state: &GameState) -> usize {
    let marked_or_dug = state
        .dungeon
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|tile| {
            tile.marked_for_dig
                || (tile.tile_type == crate::engine::tile_types::types::FLOOR
                    && tile.ownership == Ownership::Unclaimed)
        })
        .count();
    marked_or_dug + state.player.claimed_tile_count
}

fn claimed_tiles(state: &GameState) -> usize {
    state
        .dungeon
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|tile| tile.ownership == Ownership::Player)
        .count()
}

fn has_room(state: &GameState, room_type: &str) -> bool {
    state
        .room_manager
        .rooms
        .iter()
        .any(|room| room.room_type == room_type)
}

fn has_recruited_creature(state: &GameState) -> bool {
    state
        .entities
        .all()
        .filter(|entity| entity.owner == OwnerId::Player)
        .filter_map(|entity| entity.as_creature())
        .any(|creature| creature.creature_id != "imp")
}

#[cfg(test)]
mod tests;
