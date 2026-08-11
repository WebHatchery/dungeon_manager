use crate::data::GameData;
use crate::state::game_state::GameState;
use crate::state::tile_state::{FogState, TilePos};

#[test]
fn rival_lair_stays_hidden_until_scouted() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state.update_fog_of_war_system(&game_data);

    // The rival keeper's heart and garrison must not be revealed by the
    // rival's own creatures
    let rival_heart = TilePos::new(6, 23);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let pos = TilePos::new(rival_heart.x + dx, rival_heart.y + dy);
            let tile = state.get_tile(pos).expect("rival lair tile should exist");
            assert_eq!(
                tile.fog_state,
                FogState::Hidden,
                "rival lair tile {pos:?} should be hidden"
            );
        }
    }

    let player_heart = TilePos::new(14, 9);
    let tile = state
        .get_tile(player_heart)
        .expect("player heart tile should exist");
    assert_eq!(tile.fog_state, FogState::Visible);
}
