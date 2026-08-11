use super::*;
use crate::data::campaign::{
    CampaignDefinition, CampaignMission, CampaignProgress, CampaignUnlocks,
};
use crate::data::scenario::ScenarioObjective;

#[test]
fn timed_scenario_objective_drives_victory() {
    let mut game_data = GameData::load().expect("game data should load");
    add_test_scenario(
        &mut game_data,
        "timed_win",
        vec![ScenarioObjective::SurviveTime { seconds: 10.0 }],
    );
    let mut state = GameState::new_for_scenario(&game_data, "timed_win");

    state.time_elapsed = 10.1;
    update_victory_and_defeat(&mut state, &game_data);

    assert!(state.game_over);
    assert!(state.victory);
    assert!(state
        .scenario_runtime
        .as_ref()
        .unwrap()
        .completed_objectives
        .contains("survive_time"));
}

#[test]
fn scenario_victory_completes_campaign_mission() {
    let mut game_data = GameData::load().expect("game data should load");
    add_test_scenario(
        &mut game_data,
        "campaign_win",
        vec![ScenarioObjective::SurviveTime { seconds: 1.0 }],
    );
    let campaign = CampaignDefinition {
        id: "test_campaign".to_string(),
        name: "Test Campaign".to_string(),
        description: String::new(),
        starting_mission: "m1".to_string(),
        persistent_unlocks: CampaignUnlocks::default(),
        missions: vec![
            CampaignMission {
                id: "m1".to_string(),
                scenario_id: "campaign_win".to_string(),
                name: "One".to_string(),
                briefing: "Briefing".to_string(),
                unlocks_after: vec!["m2".to_string()],
                required_completed: Vec::new(),
            },
            CampaignMission {
                id: "m2".to_string(),
                scenario_id: "campaign_win".to_string(),
                name: "Two".to_string(),
                briefing: String::new(),
                unlocks_after: Vec::new(),
                required_completed: vec!["m1".to_string()],
            },
        ],
    };
    game_data
        .campaigns
        .insert(campaign.id.clone(), campaign.clone());

    let mut state = GameState::new_for_scenario(&game_data, "campaign_win");
    state.campaign_progress = Some(CampaignProgress::new(&campaign));
    state.time_elapsed = 1.1;

    update_victory_and_defeat(&mut state, &game_data);

    let progress = state.campaign_progress.as_ref().unwrap();
    assert!(progress.completed_missions.contains("m1"));
    assert_eq!(progress.active_mission, "m2");
}

#[test]
fn campaign_start_applies_persistent_content_unlocks() {
    let mut game_data = GameData::load().expect("game data should load");
    add_test_scenario(
        &mut game_data,
        "unlock_campaign",
        vec![ScenarioObjective::SurviveTime { seconds: 1.0 }],
    );
    let campaign = CampaignDefinition {
        id: "unlock_campaign".to_string(),
        name: "Unlock Campaign".to_string(),
        description: String::new(),
        starting_mission: "m1".to_string(),
        persistent_unlocks: CampaignUnlocks {
            rooms: vec!["workshop".to_string()],
            spells: vec!["lightning_strike".to_string()],
            traps: vec!["spike_trap".to_string()],
            creatures: vec!["orc".to_string()],
        },
        missions: vec![CampaignMission {
            id: "m1".to_string(),
            scenario_id: "unlock_campaign".to_string(),
            name: "One".to_string(),
            briefing: String::new(),
            unlocks_after: Vec::new(),
            required_completed: Vec::new(),
        }],
    };
    game_data
        .campaigns
        .insert(campaign.id.clone(), campaign.clone());

    let state = GameState::new_campaign_start(&game_data, "unlock_campaign");

    assert!(state.player.unlocked_rooms.contains("workshop"));
    assert!(state.player.unlocked_spells.contains("lightning_strike"));
    assert!(state.player.unlocked_traps.contains("spike_trap"));
    assert!(state.player.unlocked_creatures.contains("orc"));
}

#[test]
fn dungeon_heart_state_health_drives_defeat() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state.dungeon_heart_health = 0.0;

    update_victory_and_defeat(&mut state, &game_data);

    assert!(state.game_over);
    assert!(!state.victory);
}

fn add_test_scenario(
    game_data: &mut GameData,
    scenario_id: &str,
    objectives: Vec<ScenarioObjective>,
) {
    let mut scenario = game_data
        .scenarios
        .get("dark_beginnings")
        .expect("base scenario should exist")
        .clone();
    scenario.meta.id = scenario_id.to_string();
    scenario.objectives = objectives;
    game_data
        .scenarios
        .insert(scenario_id.to_string(), scenario);
}
