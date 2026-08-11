use super::*;
use crate::data::scenario::{
    ActionPointDefinition, ScenarioEvent, ScenarioObjective, ScenarioRules,
};
use crate::state::OwnerId;

#[test]
fn timed_event_unlocks_trap_and_spawns_party_once() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    // first_raid now triggers at 300s to give new players a setup grace period
    state.time_elapsed = 301.0;

    update_scenario_events(&mut state, &game_data);
    let hero_count = state.entities.heroes().count();
    update_scenario_events(&mut state, &game_data);

    assert_eq!(hero_count, state.entities.heroes().count());
    let runtime = state.scenario_runtime.as_ref().unwrap();
    assert!(runtime.fired_events.contains("first_raid"));
    assert!(runtime.unlocked_traps.contains("spike_trap"));
    assert!(state.player.unlocked_traps.contains("spike_trap"));
}

#[test]
fn hero_defeated_trigger_completes_the_climax_boss_objective() {
    use crate::state::entities::EntityType;

    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "heavens_reach");

    // The knight-commander boss stands in the capital from the start.
    let boss_ids: Vec<_> = state
        .entities
        .all()
        .filter(|e| e.owner == OwnerId::Heroes)
        .filter(|e| {
            matches!(&e.entity_type,
                EntityType::Hero(h) if h.hero_id == "knight_commander")
        })
        .map(|e| e.id)
        .collect();
    assert_eq!(
        boss_ids.len(),
        1,
        "the boss should be the only knight_commander at start"
    );

    // While the boss lives, it is seen but the defeat objective is not met.
    update_scenario_events(&mut state, &game_data);
    let runtime = state.scenario_runtime.as_ref().unwrap();
    assert!(runtime.seen_hero_ids.contains("knight_commander"));
    assert!(!runtime
        .completed_objectives
        .contains("defeat_the_commander"));

    // Slay the boss → the HeroDefeated trigger completes the objective.
    for id in boss_ids {
        if let Some(hero) = state.entities.get_mut(id).and_then(|e| e.as_hero_mut()) {
            hero.health = 0.0;
        }
    }
    update_scenario_events(&mut state, &game_data);
    assert!(
        state
            .scenario_runtime
            .as_ref()
            .unwrap()
            .completed_objectives
            .contains("defeat_the_commander"),
        "defeating the boss must complete the defeat objective"
    );
}

#[test]
fn hero_defeated_does_not_fire_before_the_boss_appears() {
    // A boss that only arrives in a later wave (M12's champion) must not
    // count as "defeated" before it has ever been seen.
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "deep_dominion_finale");
    update_scenario_events(&mut state, &game_data);
    let runtime = state.scenario_runtime.as_ref().unwrap();
    assert!(
        !runtime.seen_hero_ids.contains("champion_of_light"),
        "the champion is wave-only and hasn't spawned yet"
    );
    assert!(
        !runtime.completed_objectives.contains("defeat_the_champion"),
        "the champion objective must not complete before the champion appears"
    );
}

#[test]
fn action_point_event_fires_for_matching_owner() {
    let mut game_data = GameData::load().expect("game data should load");
    let mut scenario = game_data
        .scenarios
        .get("dark_beginnings")
        .expect("scenario should exist")
        .clone();
    scenario.meta.id = "action_point_test".to_string();
    scenario.action_points = vec![ActionPointDefinition {
        id: "frontier".to_string(),
        x: 12,
        y: 10,
        radius: 0,
        owner: Some(OwnerId::Player),
    }];
    scenario.events = vec![ScenarioEvent {
        id: "frontier_reached".to_string(),
        trigger: EventTrigger::ActionPointReached {
            id: "frontier".to_string(),
            owner: OwnerId::Player,
        },
        actions: vec![EventAction::CompleteObjective {
            objective: "frontier_done".to_string(),
        }],
        once: true,
    }];
    scenario.objectives = vec![ScenarioObjective::Custom {
        id: "frontier_done".to_string(),
        description: "Reach the frontier.".to_string(),
    }];
    game_data
        .scenarios
        .insert(scenario.meta.id.clone(), scenario);

    let mut state = GameState::new_for_scenario(&game_data, "action_point_test");
    update_scenario_events(&mut state, &game_data);

    let runtime = state.scenario_runtime.as_ref().unwrap();
    assert!(runtime.action_point_reached("frontier", &OwnerId::Player));
    assert!(runtime.fired_events.contains("frontier_reached"));
    assert!(runtime.completed_objectives.contains("frontier_done"));
}

#[test]
fn set_rule_event_updates_runtime_and_applies_side_effects() {
    let mut game_data = GameData::load().expect("game data should load");
    let mut scenario = game_data
        .scenarios
        .get("dark_beginnings")
        .expect("scenario should exist")
        .clone();
    scenario.meta.id = "rule_event_test".to_string();
    scenario.rules = ScenarioRules::default();
    scenario.events = vec![ScenarioEvent {
        id: "disable_fog".to_string(),
        trigger: EventTrigger::TimeElapsed { seconds: 1.0 },
        actions: vec![EventAction::SetRule {
            key: "fog_of_war".to_string(),
            value: serde_json::json!(false),
        }],
        once: true,
    }];
    game_data
        .scenarios
        .insert(scenario.meta.id.clone(), scenario);

    let mut state = GameState::new_for_scenario(&game_data, "rule_event_test");
    state.time_elapsed = 1.0;
    update_scenario_events(&mut state, &game_data);

    let runtime = state.scenario_runtime.as_ref().unwrap();
    assert!(!runtime.active_rules.fog_of_war);
    assert!(!state.cheat_fog_enabled);
    assert!(state
        .dungeon
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .all(|tile| tile.fog_state == FogState::Visible));
}
