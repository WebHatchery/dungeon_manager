use super::*;
use crate::state::settings::Difficulty;

/// Advance only the wave clock, and report the real seconds it took to fire.
fn seconds_until_wave_one(difficulty: Difficulty, scenario: &str) -> f32 {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, scenario);
    state.difficulty = difficulty;
    state.hero_base.enabled = true;

    let dt = 0.5;
    let mut elapsed = 0.0;
    // Generous ceiling: the authored delay is 600s and threat only shortens it.
    while state.hero_base.current_wave_number == 0 && elapsed < 5000.0 {
        update_wave_system(&mut state, &game_data, dt);
        elapsed += dt;
    }
    assert!(
        state.hero_base.current_wave_number > 0,
        "wave 1 never launched within {elapsed}s"
    );
    elapsed
}

/// The first wave used to be the one wave with no dial on it: `HeroBase::new`
/// seeded the countdown from `initial_delay` and nothing scaled it, so wave 1
/// landed at exactly 600s on the tutorial map at Easy and on the hardest
/// mission at Hard alike. Verified by reverting the `* threat` in
/// `update_wave_system` and watching these two go equal.
#[test]
fn wave_one_arrives_sooner_on_harder_difficulty() {
    let easy = seconds_until_wave_one(Difficulty::Easy, "the_iron_siege");
    let normal = seconds_until_wave_one(Difficulty::Normal, "the_iron_siege");
    let hard = seconds_until_wave_one(Difficulty::Hard, "the_iron_siege");

    assert!(
        hard < normal && normal < easy,
        "wave 1 should scale with difficulty: hard={hard} normal={normal} easy={easy}"
    );
}

/// The mission's authored `threat_multiplier` has to reach wave 1 too, not
/// just the waves after it — `the_iron_siege` authors 1.35, the campaign
/// opener authors none (1.0).
#[test]
fn wave_one_respects_the_missions_threat_dial() {
    let tense = seconds_until_wave_one(Difficulty::Normal, "the_iron_siege");
    let calm = seconds_until_wave_one(Difficulty::Normal, "no_such_scenario_falls_back_to_1x");

    assert!(
        tense < calm,
        "a mission authoring higher threat should be attacked sooner: {tense} vs {calm}"
    );
}

/// Real time to the wave is the stored countdown divided by threat. Pins the
/// denomination so a future reader does not "fix" the field into real seconds
/// and silently restore the unscaled first wave.
#[test]
fn wave_one_lands_at_the_authored_delay_divided_by_threat() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "the_iron_siege");
    state.difficulty = Difficulty::Normal;
    state.hero_base.enabled = true;

    let threat = state.effective_threat_multiplier(&game_data);
    let expected = game_data.config.hero_waves.initial_delay / threat;
    let actual = seconds_until_wave_one(Difficulty::Normal, "the_iron_siege");

    assert!(
        (actual - expected).abs() < 2.0,
        "expected wave 1 at ~{expected}s (600 / threat {threat}), got {actual}s"
    );
}
