use super::*;
use crate::data::campaign::{CampaignDefinition, CampaignMission, CampaignUnlocks};
use crate::data::scenario::ScenarioObjective;
use crate::engine::objectives::update_victory_and_defeat;

#[test]
fn victory_can_start_next_campaign_mission_with_progress() {
    let mut game_data = GameData::load().expect("game data should load");
    add_test_scenario(
        &mut game_data,
        "campaign_m1",
        vec![ScenarioObjective::SurviveTime { seconds: 1.0 }],
    );
    add_test_scenario(
        &mut game_data,
        "campaign_m2",
        vec![ScenarioObjective::SurviveTime { seconds: 99.0 }],
    );
    let campaign = CampaignDefinition {
        id: "two_step".to_string(),
        name: "Two Step".to_string(),
        description: String::new(),
        starting_mission: "m1".to_string(),
        persistent_unlocks: CampaignUnlocks {
            rooms: vec!["workshop".to_string()],
            ..CampaignUnlocks::default()
        },
        missions: vec![
            CampaignMission {
                id: "m1".to_string(),
                scenario_id: "campaign_m1".to_string(),
                name: "One".to_string(),
                briefing: "First briefing".to_string(),
                unlocks_after: vec!["m2".to_string()],
                required_completed: Vec::new(),
            },
            CampaignMission {
                id: "m2".to_string(),
                scenario_id: "campaign_m2".to_string(),
                name: "Two".to_string(),
                briefing: "Second briefing".to_string(),
                unlocks_after: Vec::new(),
                required_completed: vec!["m1".to_string()],
            },
        ],
    };
    game_data.campaigns.insert(campaign.id.clone(), campaign);

    let mut state = GameState::new_campaign_start(&game_data, "two_step");
    state.time_elapsed = 1.1;
    update_victory_and_defeat(&mut state, &game_data);

    assert!(state.game_over);
    assert!(state.has_pending_campaign_mission(&game_data));

    let next_state = state
        .start_pending_campaign_mission(&game_data)
        .expect("next mission should start");
    let progress = next_state.campaign_progress.as_ref().unwrap();
    assert_eq!(
        next_state.active_scenario_id.as_deref(),
        Some("campaign_m2")
    );
    assert_eq!(progress.active_mission, "m2");
    assert!(progress.completed_missions.contains("m1"));
    assert!(next_state.player.unlocked_rooms.contains("workshop"));
    assert_eq!(
        next_state.active_campaign_briefing(&game_data),
        Some("Second briefing")
    );
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
