use crate::data::GameData;
use crate::state::game_state::GameState;

#[test]
fn scenario_start_gold_survives_capacity_clamp() {
    let game_data = GameData::load().expect("game data should load");
    let state = GameState::new_for_scenario(&game_data, "dark_beginnings");

    // Base capacity (2000) + dungeon heart storage (500) must hold the
    // scenario's 2500 start gold without clamping it away.
    assert!(
        state.player.max_gold >= 2500,
        "start capacity too low: {}",
        state.player.max_gold
    );
    assert_eq!(state.player.gold, 2500);
}
