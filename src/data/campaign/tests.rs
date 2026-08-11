use super::*;

#[test]
fn deep_dominion_campaign_is_complete_thirteen_entries() {
    // The full 12-slot arc is authored — 13 mission *entries* because the M7
    // slot is a two-way branch (restless_dead + pacts_and_sacrifice). The
    // finale is terminal, and playing the chain (graveyard branch) reaches it.
    let campaign = load_campaigns()
        .expect("campaign json should parse")
        .remove("deep_dominion")
        .expect("campaign missing");
    assert_eq!(
        campaign.missions.len(),
        13,
        "12 arc slots + the extra branch mission"
    );

    let finale = campaign
        .missions
        .iter()
        .find(|m| m.id == "deep_dominion_finale")
        .expect("finale present");
    assert!(finale.unlocks_after.is_empty(), "finale is terminal");

    let mut progress = CampaignProgress::new(&campaign);
    for m in [
        "dark_beginnings",
        "blood_and_iron",
        "the_long_dark",
        "no_prisoners",
        "whispers_in_the_circle",
        "the_kennels",
        "restless_dead",
        "the_iron_siege",
        "corruption_rising",
        "two_kings",
        "heavens_reach",
    ] {
        progress.complete_mission(&campaign, m);
    }
    assert!(
        progress.unlocked_missions.contains("deep_dominion_finale"),
        "the finale must be reachable by playing the chain"
    );
}

#[test]
fn mission_menu_tags_status_and_exposes_the_branch_choice() {
    let campaign = load_campaigns()
        .expect("campaign json should parse")
        .remove("deep_dominion")
        .expect("campaign missing");
    let mut progress = CampaignProgress::new(&campaign);

    // At the start only the opener is Available; everything else is Locked.
    let menu = progress.mission_menu(&campaign);
    assert_eq!(menu.len(), 13);
    let opener = menu.iter().find(|e| e.id == "dark_beginnings").unwrap();
    assert_eq!(opener.status, MissionStatus::Available);
    assert!(!opener.name.is_empty() && !opener.briefing.is_empty());
    assert_eq!(
        menu.iter()
            .find(|e| e.id == "blood_and_iron")
            .unwrap()
            .status,
        MissionStatus::Locked
    );

    // A locked mission cannot be selected...
    assert!(!progress.select_mission(&campaign, "the_iron_siege"));

    // Play up to the branch: both M7 paths become Available together — this
    // is exactly what the mission-select screen exposes (the player picks).
    for m in [
        "dark_beginnings",
        "blood_and_iron",
        "the_long_dark",
        "no_prisoners",
        "whispers_in_the_circle",
        "the_kennels",
    ] {
        progress.complete_mission(&campaign, m);
    }
    let menu = progress.mission_menu(&campaign);
    let status = |id: &str| menu.iter().find(|e| e.id == id).unwrap().status;
    assert_eq!(status("restless_dead"), MissionStatus::Available);
    assert_eq!(status("pacts_and_sacrifice"), MissionStatus::Available);
    assert_eq!(status("the_kennels"), MissionStatus::Completed);
    assert_eq!(status("the_iron_siege"), MissionStatus::Locked);

    // The player selects the temple path explicitly.
    assert!(progress.select_mission(&campaign, "pacts_and_sacrifice"));
    assert_eq!(progress.active_mission, "pacts_and_sacrifice");
    // A completed mission may be replayed.
    assert!(progress.select_mission(&campaign, "the_kennels"));
}

#[test]
fn embedded_campaign_parses() {
    let campaigns = load_campaigns().expect("campaign json should parse");
    let campaign = campaigns
        .get("deep_dominion")
        .expect("default campaign missing");
    assert_eq!(campaign.starting_mission, "dark_beginnings");
    assert!(!campaign.missions.is_empty());
}

#[test]
fn deep_dominion_unlocks_blood_and_iron_after_dark_beginnings() {
    let campaigns = load_campaigns().expect("campaign json should parse");
    let campaign = campaigns.get("deep_dominion").expect("campaign missing");

    // Both authored missions are present and correctly gated.
    assert!(campaign.missions.iter().any(|m| m.id == "blood_and_iron"));
    let mut progress = CampaignProgress::new(campaign);
    assert_eq!(progress.active_mission, "dark_beginnings");

    progress.complete_mission(campaign, "dark_beginnings");
    assert_eq!(
        progress.active_mission, "blood_and_iron",
        "completing the opener should advance to the second authored mission"
    );

    // ...and the demo slice's third mission gates behind the second.
    assert!(campaign.missions.iter().any(|m| m.id == "the_long_dark"));
    progress.complete_mission(campaign, "blood_and_iron");
    assert_eq!(
        progress.active_mission, "the_long_dark",
        "completing mission 2 should advance to mission 3"
    );

    // ...and the Act I finale gates behind the economy mission.
    assert!(campaign.missions.iter().any(|m| m.id == "no_prisoners"));
    progress.complete_mission(campaign, "the_long_dark");
    assert_eq!(
        progress.active_mission, "no_prisoners",
        "completing mission 3 should advance to mission 4"
    );

    // ...and the Act II opener gates behind the Act I finale.
    assert!(campaign
        .missions
        .iter()
        .any(|m| m.id == "whispers_in_the_circle"));
    progress.complete_mission(campaign, "no_prisoners");
    assert_eq!(
        progress.active_mission, "whispers_in_the_circle",
        "completing mission 4 should advance to mission 5"
    );

    // ...and the army-scaling mission gates behind the Act II opener.
    assert!(campaign.missions.iter().any(|m| m.id == "the_kennels"));
    progress.complete_mission(campaign, "whispers_in_the_circle");
    assert_eq!(
        progress.active_mission, "the_kennels",
        "completing mission 5 should advance to mission 6"
    );

    // The branch: completing M6 unlocks BOTH M7 paths (the arc's design fork),
    // each gated on the_kennels via required_completed.
    progress.complete_mission(campaign, "the_kennels");
    assert!(
        progress.unlocked_missions.contains("restless_dead"),
        "M6 completion should unlock the graveyard path"
    );
    assert!(
        progress.unlocked_missions.contains("pacts_and_sacrifice"),
        "M6 completion should unlock the temple path"
    );
    for branch in ["restless_dead", "pacts_and_sacrifice"] {
        let mission = campaign
            .missions
            .iter()
            .find(|m| m.id == branch)
            .expect("branch mission present");
        assert_eq!(mission.required_completed, vec!["the_kennels".to_string()]);
    }

    // The re-merge: completing EITHER branch unlocks M8 (the_iron_siege).
    // Both branches `unlocks_after` M8, so the OR-join is expressed through
    // the additive unlock edge rather than an (AND-only) required_completed.
    // (active_mission may point at the still-available other branch until the
    // player selects — the graph edge is what matters for reachability.)
    progress.complete_mission(campaign, "restless_dead");
    assert!(
        progress.unlocked_missions.contains("the_iron_siege"),
        "finishing either M7 path should unlock the M8 re-merge"
    );
}

#[test]
fn either_m7_branch_re_merges_into_the_iron_siege() {
    // Prove the OR-join from the *other* branch too: a fresh playthrough that
    // takes the temple path still reaches M8.
    let campaign = load_campaigns()
        .expect("campaign json should parse")
        .remove("deep_dominion")
        .expect("campaign missing");
    let mut progress = CampaignProgress::new(&campaign);
    for m in [
        "dark_beginnings",
        "blood_and_iron",
        "the_long_dark",
        "no_prisoners",
    ] {
        progress.complete_mission(&campaign, m);
    }
    progress.complete_mission(&campaign, "whispers_in_the_circle");
    progress.complete_mission(&campaign, "the_kennels");
    progress.complete_mission(&campaign, "pacts_and_sacrifice");
    assert!(
        progress.unlocked_missions.contains("the_iron_siege"),
        "the temple path must also unlock the M8 re-merge"
    );

    // ...and past the re-merge M8 unlocks the Act III offense mission.
    // (active_mission may still point at the unplayed M7 branch, which
    // remains available; reachability is the unlock edge.)
    progress.complete_mission(&campaign, "the_iron_siege");
    assert!(
        progress.unlocked_missions.contains("corruption_rising"),
        "completing M8 should unlock the Act III offense mission"
    );

    // ...and M9 unlocks the two-rival duel (M10).
    progress.complete_mission(&campaign, "corruption_rising");
    assert!(
        progress.unlocked_missions.contains("two_kings"),
        "completing M9 should unlock the rival-keeper duel"
    );
}

#[test]
fn progress_unlocks_next_mission() {
    let campaign: CampaignDefinition = serde_json::from_str(
        r#"{
          "id": "test",
          "name": "Test",
          "starting_mission": "m1",
          "missions": [
            { "id": "m1", "scenario_id": "s1", "name": "One", "unlocks_after": ["m2"] },
            { "id": "m2", "scenario_id": "s2", "name": "Two", "required_completed": ["m1"] }
          ]
        }"#,
    )
    .unwrap();

    let mut progress = CampaignProgress::new(&campaign);
    progress.complete_mission(&campaign, "m1");

    assert!(progress.completed_missions.contains("m1"));
    assert_eq!(progress.active_mission, "m2");
}

#[test]
fn progress_exposes_active_briefing_and_unlocked_missions() {
    let campaign: CampaignDefinition = serde_json::from_str(
        r#"{
          "id": "test",
          "name": "Test",
          "starting_mission": "m1",
          "missions": [
            { "id": "m1", "scenario_id": "s1", "name": "One", "briefing": "First briefing" },
            { "id": "m2", "scenario_id": "s2", "name": "Two" }
          ]
        }"#,
    )
    .unwrap();

    let progress = CampaignProgress::new(&campaign);

    assert_eq!(
        progress.active_mission(&campaign).unwrap().briefing,
        "First briefing"
    );
    assert_eq!(progress.unlocked_missions(&campaign).len(), 1);
}
