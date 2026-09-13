use super::*;
use crate::engine::room_validator::Room;

fn active_room(id: usize, room_type: &str, tiles: &[TilePos]) -> Room {
    let mut room = Room::new(
        id,
        room_type.to_string(),
        tiles.iter().copied().collect(),
        Vec::new(),
    );
    room.active = true;
    room
}

fn set_tile(state: &mut GameState, pos: TilePos, tile_type: &str, ownership: Ownership) {
    let tile = state.get_tile_mut(pos).expect("test tile should exist");
    tile.tile_type = tile_type.to_string();
    match ownership {
        Ownership::Player => tile.claim(),
        Ownership::Enemy => tile.set_owner(crate::state::OwnerId::RivalKeeper(1)),
        Ownership::Unclaimed => {
            tile.ownership = Ownership::Unclaimed;
            tile.owner = crate::state::OwnerId::Neutral;
        }
    }
}

#[test]
fn bridge_building_claims_water_or_lava_crossing() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let floor = TilePos::new(2, 2);
    let water = TilePos::new(3, 2);
    set_tile(&mut state, floor, tt::CLAIMED_FLOOR, Ownership::Player);
    set_tile(&mut state, water, "water", Ownership::Unclaimed);
    let before_gold = state.player.gold;

    handle_build_room(&mut state, &game_data, tt::BRIDGE, water);

    let tile = state.get_tile(water).expect("bridge tile should exist");
    assert_eq!(tile.tile_type, tt::BRIDGE);
    assert_eq!(tile.ownership, Ownership::Player);
    assert!(state.player.gold < before_gold);
}

#[test]
fn trap_placement_consumes_manufactured_inventory() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let trap_pos = TilePos::new(2, 2);
    set_tile(&mut state, trap_pos, tt::CLAIMED_FLOOR, Ownership::Player);
    state
        .room_manager
        .rooms
        .push(active_room(777, "workshop", &[TilePos::new(3, 3)]));
    state.player.unlock_trap("spike_trap".to_string());
    state.player.add_trap_inventory("spike_trap".to_string(), 1);

    handle_build_trap(&mut state, &game_data, "spike_trap", trap_pos);

    let tile = state.get_tile(trap_pos).expect("trap tile should exist");
    let trap = tile.trap.as_ref().expect("trap should be placed");
    assert_eq!(trap.trap_type, "spike_trap");
    assert!(trap.funded);
    assert_eq!(state.player.trap_inventory_count("spike_trap"), 0);
    assert!(state.pending_trap_builds.contains(&trap_pos));
}

#[test]
fn lockable_doors_can_be_opened_and_closed_by_tile_action() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let door_pos = TilePos::new(2, 2);
    let tile = state.get_tile_mut(door_pos).unwrap();
    tile.tile_type = tt::CLAIMED_FLOOR.to_string();
    tile.claim();
    tile.trap = Some(crate::state::tile_state::TrapState {
        trap_type: "magic_door".to_string(),
        constructed: true,
        construction_progress: 12.0,
        active: true,
        locked: true,
        funded: true,
        cooldown: 0.0,
        triggered: false,
    });

    assert_eq!(
        crate::engine::trap_system::toggle_door_lock_at(
            &mut state.dungeon,
            &game_data,
            door_pos,
        ),
        Some(false)
    );
    assert!(crate::engine::tile_types::is_tile_walkable(
        state.get_tile(door_pos).unwrap(),
        &game_data
    ));
    assert_eq!(
        crate::engine::trap_system::toggle_door_lock_at(
            &mut state.dungeon,
            &game_data,
            door_pos,
        ),
        Some(true)
    );
    assert!(!crate::engine::tile_types::is_tile_walkable(
        state.get_tile(door_pos).unwrap(),
        &game_data
    ));
}
