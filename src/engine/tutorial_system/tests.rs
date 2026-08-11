use super::*;
use crate::data::GameData;

#[test]
fn tutorial_starts_with_intro_and_advances_on_dig_marks() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");

    assert!(state.tutorial.enabled);
    assert!(!state.tutorial.complete);
    assert_eq!(state.tutorial.step_index, 0);
    assert!(
        pending_intro(&state, &game_data).is_some(),
        "dark_beginnings should have intro story lines"
    );

    // Nothing dug yet: the first objective must not auto-complete
    update_tutorial(&mut state, &game_data);
    assert_eq!(state.tutorial.step_index, 0);

    // Mark six earth tiles for digging
    let mut marked = 0;
    for row in &mut state.dungeon.grid {
        for tile in row.iter_mut() {
            if marked >= DIG_TARGET {
                break;
            }
            if tile.tile_type == "earth" {
                tile.marked_for_dig = true;
                marked += 1;
            }
        }
    }
    assert_eq!(marked, DIG_TARGET);

    update_tutorial(&mut state, &game_data);
    assert_eq!(state.tutorial.step_index, 1);
}
