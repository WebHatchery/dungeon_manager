use super::*;
use crate::data::GameData;
use crate::state::entities::CreatureState;
use crate::state::tile_state::TilePos;

#[test]
fn hostile_creatures_can_damage_the_player_heart() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let heart_pos = state.find_dungeon_heart_position().unwrap();
    let monster_data = game_data.monsters.get("goblin").unwrap();
    let creature = CreatureState::new(
        "goblin".to_string(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        42,
    );
    state.entities.spawn_creature_for_owner(
        TilePos::new(heart_pos.x + 1, heart_pos.y),
        creature,
        OwnerId::RivalKeeper(1),
    );
    let start_health = state.dungeon_heart_health;

    state.process_dungeon_heart_attacks(&game_data, 100.0);

    assert!(state.dungeon_heart_health < start_health);
}
