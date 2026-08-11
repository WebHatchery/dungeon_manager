//! Tutorial step progression
//!
//! Each step is a predicate over existing game state, checked once per tick.
//! No events need to be wired: the tutorial simply observes what the player
//! has already accomplished.

use crate::data::GameData;
use crate::state::game_state::GameState;
use crate::state::tile_state::Ownership;
use crate::state::OwnerId;

pub struct TutorialStep {
    pub title: &'static str,
    pub hint: &'static str,
}

pub const STEPS: &[TutorialStep] = &[
    TutorialStep {
        title: "Dig out a room",
        hint: "Select Dig (key 1 or the Build tab) and drag across earth tiles. Your imps will excavate them.",
    },
    TutorialStep {
        title: "Expand your territory",
        hint: "Imps claim dug floor connected to your dungeon. Keep digging until your domain grows.",
    },
    TutorialStep {
        title: "Build a Lair",
        hint: "Select Lair (key 2) and drag over claimed floor. Creatures need somewhere to sleep.",
    },
    TutorialStep {
        title: "Build a Hatchery",
        hint: "Select Hatchery (key 3) and place it on claimed floor. It feeds your creatures.",
    },
    TutorialStep {
        title: "Build a Treasury",
        hint: "Select Treasury (key 4) to store more gold. Dig into gold veins to fill it.",
    },
    TutorialStep {
        title: "Recruit a creature",
        hint: "With a lair and hatchery ready, creatures will join your dungeon. Prepare defenses before the heroes arrive!",
    },
];

/// The step the player is currently on, if the tutorial is running.
pub fn current_step(state: &GameState) -> Option<&'static TutorialStep> {
    if !state.tutorial.enabled || state.tutorial.complete {
        return None;
    }
    STEPS.get(state.tutorial.step_index)
}

/// Progress counter (current, target) for count-based steps.
pub fn step_progress(state: &GameState) -> Option<(usize, usize)> {
    match state.tutorial.step_index {
        0 => Some((dig_progress(state).min(DIG_TARGET), DIG_TARGET)),
        1 => Some((claimed_tiles(state).min(CLAIM_TARGET), CLAIM_TARGET)),
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
pub fn update_tutorial(state: &mut GameState, _game_data: &GameData) {
    if !state.tutorial.enabled || state.tutorial.complete {
        return;
    }

    let done = match state.tutorial.step_index {
        0 => dig_progress(state) >= DIG_TARGET,
        1 => claimed_tiles(state) >= CLAIM_TARGET,
        2 => has_room(state, "lair"),
        3 => has_room(state, "hatchery"),
        4 => has_room(state, "treasury"),
        5 => has_recruited_creature(state),
        _ => true,
    };

    if !done {
        return;
    }

    if let Some(step) = STEPS.get(state.tutorial.step_index) {
        state
            .notifications
            .success(format!("Objective complete: {}", step.title));
    }

    state.tutorial.step_index += 1;
    if state.tutorial.step_index >= STEPS.len() {
        state.tutorial.complete = true;
        state
            .notifications
            .success("Tutorial complete. Your dungeon awaits, Keeper!");
    }
}

const DIG_TARGET: usize = 6;
const CLAIM_TARGET: usize = 15;

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
