use super::*;
use crate::data::scenario::load_scenarios;

#[test]
fn runtime_state_tracks_availability_and_unlocks() {
    let scenarios = load_scenarios().expect("scenario should parse");
    let scenario = scenarios.get("dark_beginnings").unwrap();
    let mut state = ScenarioRuntimeState::from_definition(scenario);

    assert!(state.available_rooms.contains("workshop"));
    assert!(!state.unlocked_rooms.contains("workshop"));
    state.unlock_room("workshop");
    assert!(state.unlocked_rooms.contains("workshop"));
}

#[test]
fn runtime_state_tracks_action_points_and_rules() {
    let scenarios = load_scenarios().expect("scenario should parse");
    let scenario = scenarios.get("dark_beginnings").unwrap();
    let mut state = ScenarioRuntimeState::from_definition(scenario);

    state.mark_action_point_reached("outer_gate", &OwnerId::Player);
    assert!(state.action_point_reached("outer_gate", &OwnerId::Player));
    assert!(!state.action_point_reached("outer_gate", &OwnerId::Heroes));

    assert!(state.set_rule("threat_multiplier", &serde_json::json!(1.5)));
    assert_eq!(state.active_rules.threat_multiplier, 1.5);
    assert!(state.set_rule("disabled_systems", &serde_json::json!(["hero_spawner"])));
    assert_eq!(
        state.active_rules.disabled_systems,
        vec!["hero_spawner".to_string()]
    );
    assert!(!state.set_rule("unknown_rule", &serde_json::json!(true)));
}
