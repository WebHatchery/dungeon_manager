use super::*;

#[test]
fn embedded_scenario_parses() {
    let scenarios = load_scenarios().expect("scenario json should parse");
    let scenario = scenarios
        .get("dark_beginnings")
        .expect("default campaign scenario missing");
    assert_eq!(scenario.players[0].role, ScenarioPlayerRole::Human);
    assert!(!scenario.objectives.is_empty());
    assert!(!scenario.hero_parties.is_empty());
}

#[test]
fn authored_campaign_scenarios_reference_only_real_content() {
    // Every authored mission scenario must reference only ids that exist in
    // game data — the manifest loader makes adding these a pure-content act,
    // so this guards content authors against typos across the whole campaign.
    let game_data = GameData::load().expect("game data should load");
    for scenario in game_data.scenarios.values() {
        let errors = scenario.validate_references(&game_data);
        assert!(
            errors.is_empty(),
            "scenario '{}' has dangling references: {:?}",
            scenario.meta.id,
            errors
        );
    }
}

#[test]
fn blood_and_iron_is_wired_and_valid() {
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data
        .scenarios
        .get("blood_and_iron")
        .expect("blood_and_iron scenario should be loaded via the manifest");

    assert!(scenario.validate_references(&game_data).is_empty());
    // Objectives: survive the assault, then raze the outpost.
    assert!(scenario
        .objectives
        .iter()
        .any(|o| matches!(o, ScenarioObjective::DestroyAllHeroBuildings)));
    assert!(scenario
        .objectives
        .iter()
        .any(|o| matches!(o, ScenarioObjective::SurviveTime { .. })));
    // Two scripted hero waves.
    assert_eq!(scenario.hero_parties.len(), 2);
}

#[test]
fn scenario_validation_catches_missing_ids() {
    let mut scenario = load_scenarios()
        .expect("scenario json should parse")
        .remove("dark_beginnings")
        .expect("scenario missing");
    let game_data = GameData::default();
    scenario
        .creature_pool
        .insert("not_a_creature".to_string(), 1);

    let errors = scenario.validate_references(&game_data);

    assert!(errors.iter().any(|e| e.contains("not_a_creature")));
}

#[test]
fn scenario_supports_above_ground_maps() {
    let json = r#"{
      "meta": { "id": "surface_keep", "name": "Surface Keep" },
      "map": { "path": "assets/maps/surface_keep.json", "above_ground": true },
      "players": [
        { "id": "town", "owner": "above_ground", "role": "above_ground" }
      ]
    }"#;
    let scenario: ScenarioDefinition = serde_json::from_str(json).unwrap();
    assert!(scenario.map.above_ground);
    assert_eq!(scenario.players[0].owner, OwnerId::AboveGround);
}

#[test]
fn scenario_supports_action_points() {
    let json = r#"{
      "meta": { "id": "ambush_path", "name": "Ambush Path" },
      "map": { "path": "assets/maps/level_1.json" },
      "action_points": [
        { "id": "outer_gate", "x": 10, "y": 12, "radius": 2, "owner": "player" }
      ],
      "events": [
        {
          "id": "gate_ambush",
          "trigger": { "type": "action_point_reached", "id": "outer_gate", "owner": "player" },
          "actions": []
        }
      ]
    }"#;
    let scenario: ScenarioDefinition = serde_json::from_str(json).unwrap();
    assert_eq!(scenario.action_points[0].id, "outer_gate");
    assert_eq!(scenario.action_points[0].owner, Some(OwnerId::Player));
}
