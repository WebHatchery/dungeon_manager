use crate::data::scenario::{EventAction, EventTrigger, ScenarioDefinition, ScenarioEvent};
use crate::data::GameData;
use crate::state::entities::CreatureState;
use crate::state::game_state::GameState;
use crate::state::tile_state::FogState;
use crate::state::tile_state::TilePos;

pub fn update_scenario_events(state: &mut GameState, game_data: &GameData) {
    let Some(scenario_id) = state.active_scenario_id.clone() else {
        return;
    };
    let Some(scenario) = game_data.scenarios.get(&scenario_id) else {
        return;
    };

    update_action_points(state, scenario);
    record_seen_heroes(state);

    let events_to_fire: Vec<ScenarioEvent> = scenario
        .events
        .iter()
        .filter(|event| {
            let already_fired = state
                .scenario_runtime
                .as_ref()
                .map(|runtime| runtime.fired_events.contains(&event.id))
                .unwrap_or(false);
            (!event.once || !already_fired) && trigger_matches(event, state)
        })
        .cloned()
        .collect();

    for event in events_to_fire {
        for action in &event.actions {
            apply_action(state, game_data, action);
        }
        if let Some(runtime) = &mut state.scenario_runtime {
            runtime.mark_event_fired(&event.id);
        }
    }
}

/// Living hero ids (heroes faction). Shared by the seen-tracker and the
/// `HeroDefeated` trigger so both agree on what "alive" means.
fn living_hero_ids(state: &GameState) -> impl Iterator<Item = &str> {
    use crate::state::entities::EntityType;
    use crate::state::faction::OwnerId;
    state.entities.all().filter_map(|entity| {
        if entity.owner == OwnerId::Heroes && entity.is_alive() {
            if let EntityType::Hero(hero) = &entity.entity_type {
                return Some(hero.hero_id.as_str());
            }
        }
        None
    })
}

/// Remember every hero id currently alive so `HeroDefeated` can tell "never
/// appeared" from "appeared and was slain".
fn record_seen_heroes(state: &mut GameState) {
    let seen: Vec<String> = living_hero_ids(state).map(|id| id.to_string()).collect();
    if seen.is_empty() {
        return;
    }
    if let Some(runtime) = &mut state.scenario_runtime {
        runtime.seen_hero_ids.extend(seen);
    }
}

fn trigger_matches(event: &ScenarioEvent, state: &GameState) -> bool {
    match &event.trigger {
        EventTrigger::TimeElapsed { seconds } => state.time_elapsed >= *seconds,
        EventTrigger::ObjectiveComplete { objective } => state
            .scenario_runtime
            .as_ref()
            .map(|runtime| runtime.completed_objectives.contains(objective))
            .unwrap_or(false),
        EventTrigger::RoomClaimed { room, owner } => {
            owner.is_player_controlled()
                && state
                    .room_manager
                    .rooms
                    .iter()
                    .any(|active_room| active_room.room_type == *room)
        }
        EventTrigger::ActionPointReached { id, owner } => state
            .scenario_runtime
            .as_ref()
            .map(|runtime| runtime.action_point_reached(id, owner))
            .unwrap_or(false),
        EventTrigger::DungeonBreached { owner } => state.entities.heroes().any(|(id, _)| {
            let Some(entity) = state.entities.get(id) else {
                return false;
            };
            let Some(tile) = state.dungeon.get_tile(entity.pos) else {
                return false;
            };
            tile.owner == *owner
        }),
        EventTrigger::HeroDefeated { hero } => {
            let seen = state
                .scenario_runtime
                .as_ref()
                .map(|runtime| runtime.seen_hero_ids.contains(hero))
                .unwrap_or(false);
            seen && !living_hero_ids(state).any(|id| id == hero)
        }
    }
}

fn apply_action(state: &mut GameState, game_data: &GameData, action: &EventAction) {
    match action {
        EventAction::UnlockRoom { room, owner } => {
            if owner.is_player_controlled() {
                state.player.unlock_room(room.clone());
            }
            if let Some(runtime) = &mut state.scenario_runtime {
                runtime.unlock_room(room);
            }
        }
        EventAction::UnlockSpell { spell, owner } => {
            if owner.is_player_controlled() {
                state.player.unlock_spell(spell.clone());
            }
            if let Some(runtime) = &mut state.scenario_runtime {
                runtime.unlock_spell(spell);
            }
        }
        EventAction::UnlockTrap { trap, owner } => {
            if owner.is_player_controlled() {
                state.player.unlock_trap(trap.clone());
            }
            if let Some(runtime) = &mut state.scenario_runtime {
                runtime.unlock_trap(trap);
            }
        }
        EventAction::SpawnCreature {
            creature,
            owner,
            x,
            y,
            level,
        } => {
            let Some(monster_data) = game_data.monsters.get(creature) else {
                return;
            };
            let visual_seed = macroquad_toolkit::rng::random_u64();
            let creature_state = CreatureState::new(
                creature.clone(),
                *level,
                monster_data.stats.health,
                monster_data.stats.mana,
                visual_seed,
            );
            state.entities.spawn_creature_for_owner(
                TilePos::new(*x, *y),
                creature_state,
                owner.clone(),
            );
        }
        EventAction::SpawnHeroParty { party, x, y } => {
            let Some(scenario_id) = &state.active_scenario_id else {
                return;
            };
            let Some(scenario) = game_data.scenarios.get(scenario_id) else {
                return;
            };
            let Some(party) = scenario.hero_parties.iter().find(|p| p.id == *party) else {
                return;
            };
            crate::engine::hero_parties::spawn_hero_party(
                state,
                party,
                TilePos::new(*x, *y),
                game_data,
            );
        }
        EventAction::SetRule { key, value } => apply_rule(state, game_data, key, value),
        EventAction::CompleteObjective { objective } => {
            if let Some(runtime) = &mut state.scenario_runtime {
                runtime.mark_objective_complete(objective);
            }
        }
    }
}

fn update_action_points(state: &mut GameState, scenario: &ScenarioDefinition) {
    if scenario.action_points.is_empty() {
        return;
    }

    let mut reached = Vec::new();
    for point in &scenario.action_points {
        let center = TilePos::new(point.x, point.y);
        for entity in state.entities.all() {
            if let Some(owner) = &point.owner {
                if &entity.owner != owner {
                    continue;
                }
            }
            if entity.pos.manhattan_distance(&center) <= point.radius.max(0) {
                reached.push((point.id.clone(), entity.owner.clone()));
            }
        }
    }

    if let Some(runtime) = &mut state.scenario_runtime {
        for (point_id, owner) in reached {
            runtime.mark_action_point_reached(&point_id, &owner);
        }
    }
}

fn apply_rule(state: &mut GameState, game_data: &GameData, key: &str, value: &serde_json::Value) {
    let Some(runtime) = &mut state.scenario_runtime else {
        return;
    };
    if !runtime.set_rule(key, value) {
        return;
    }

    match key {
        "fog_of_war" => {
            if value.as_bool() == Some(false) {
                state.cheat_fog_enabled = false;
                reveal_all_tiles(state);
            } else if value.as_bool() == Some(true) {
                state.cheat_fog_enabled = true;
            }
        }
        "starting_research" => {
            if let Some(research_ids) = value.as_array() {
                for tech_id in research_ids.iter().filter_map(|id| id.as_str()) {
                    if let Some(tech) = game_data.technologies.get(tech_id) {
                        state.player.complete_research(tech);
                    }
                }
            }
        }
        _ => {}
    }
}

fn reveal_all_tiles(state: &mut GameState) {
    for row in &mut state.dungeon.grid {
        for tile in row {
            tile.fog_state = FogState::Visible;
        }
    }
}

#[cfg(test)]
mod tests;
