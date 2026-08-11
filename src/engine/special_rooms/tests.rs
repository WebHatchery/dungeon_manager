use super::*;
use crate::engine::room_validator::Room;
use crate::state::entities::{CreatureState, HeroState};

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

#[test]
fn temple_sacrifices_dropped_player_creature_for_mana() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let pos = TilePos::new(5, 5);
    state
        .room_manager
        .rooms
        .push(active_room(998, "temple", &[pos]));
    let monster_data = game_data.monsters.get("goblin").unwrap();
    let creature = CreatureState::new(
        "goblin".to_string(),
        2,
        monster_data.stats.health,
        monster_data.stats.mana,
        1,
    );
    let entity_id = state.entities.spawn_creature(pos, creature);
    state.player.mana = 0;
    let before_mana = state.player.mana;

    assert!(sacrifice_creature_at(
        &mut state, &game_data, entity_id, pos
    ));

    assert!(state.entities.get(entity_id).is_none());
    assert!(state.player.mana > before_mana);
}

#[test]
fn graveyard_stores_corpses_and_spawns_vampire() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let graveyard_tiles = [
        TilePos::new(5, 5),
        TilePos::new(6, 5),
        TilePos::new(7, 5),
        TilePos::new(5, 6),
        TilePos::new(6, 6),
    ];
    state
        .room_manager
        .rooms
        .push(active_room(999, "graveyard", &graveyard_tiles));

    for index in 0..VAMPIRE_CORPSE_COST {
        let mut hero = HeroState::new(
            "knight".to_string(),
            1,
            100.0,
            10.0,
            TilePos::new(index as i32, 1),
            1.0,
            index as u64,
        );
        hero.health = 0.0;
        state
            .entities
            .spawn_hero(TilePos::new(index as i32, 1), hero);
    }

    let report = process_special_rooms(&mut state, &game_data, 1.0);

    assert_eq!(report.corpses_collected, VAMPIRE_CORPSE_COST as usize);
    assert_eq!(report.vampires_spawned, 1);
    assert_eq!(state.player.graveyard_corpses, 0);
    assert!(state
        .entities
        .creatures()
        .any(|(_, creature)| creature.creature_id == "vampire"));
}

/// `count` dead knights lying on the floor.
fn spawn_dead_heroes(state: &mut GameState, count: u32) {
    for index in 0..count {
        let mut hero = HeroState::new(
            "knight".to_string(),
            1,
            100.0,
            10.0,
            TilePos::new(index as i32, 1),
            1.0,
            index as u64,
        );
        hero.health = 0.0;
        state
            .entities
            .spawn_hero(TilePos::new(index as i32, 1), hero);
    }
}

#[test]
fn soul_furnace_renders_bodies_into_mana() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state
        .room_manager
        .rooms
        .push(active_room(997, "soul_furnace", &[TilePos::new(9, 9)]));
    state.player.mana = 0;
    spawn_dead_heroes(&mut state, 3);

    let report = process_special_rooms(&mut state, &game_data, 1.0);

    assert_eq!(report.corpses_burned, 3);
    let per_corpse = game_data.rooms["soul_furnace"].effects.mana_per_corpse;
    assert!(report.mana_generated >= per_corpse * 3.0);
    assert!(state.player.mana > 0);
    // The bodies are consumed, not left lying.
    assert_eq!(state.entities.heroes().count(), 0);
}

#[test]
fn the_graveyard_takes_bodies_before_the_furnace_burns_them() {
    // Both rooms want the same corpses. The graveyard stores what it can
    // and the furnace only gets the surplus — if that order flipped, a
    // keeper with both rooms could never raise a vampire.
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state
        .room_manager
        .rooms
        .push(active_room(999, "graveyard", &[TilePos::new(5, 5)]));
    state
        .room_manager
        .rooms
        .push(active_room(997, "soul_furnace", &[TilePos::new(9, 9)]));

    let capacity = graveyard_corpse_capacity(&state, &game_data);
    assert!(capacity > 0, "the graveyard fixture should store something");
    spawn_dead_heroes(&mut state, capacity + 2);

    let report = process_special_rooms(&mut state, &game_data, 1.0);

    assert_eq!(report.corpses_collected, capacity as usize);
    assert_eq!(report.corpses_burned, 2);
}

#[test]
fn a_room_without_mana_per_corpse_burns_nothing() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state
        .room_manager
        .rooms
        .push(active_room(996, "treasury", &[TilePos::new(9, 9)]));
    spawn_dead_heroes(&mut state, 3);

    let report = process_special_rooms(&mut state, &game_data, 1.0);

    assert_eq!(report.corpses_burned, 0);
    assert_eq!(state.entities.heroes().count(), 3);
}

#[test]
fn passive_mana_comes_from_the_effect_not_the_room_name() {
    // `generate_room_mana` used to test for temple/ritual_circle by name,
    // so a third room could declare a rate and produce nothing.
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state
        .room_manager
        .rooms
        .push(active_room(995, "soul_furnace", &[TilePos::new(9, 9)]));

    let report = process_special_rooms(&mut state, &game_data, 1.0);

    let rate = game_data.rooms["soul_furnace"]
        .effects
        .mana_generation_per_second;
    assert!(rate > 0.0, "the furnace should declare a passive rate");
    assert!(report.mana_generated > 0.0);
}

#[test]
fn barracks_sets_marker_and_groups_idle_combat_creatures() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    state.room_manager.rooms.push(active_room(
        1000,
        "barracks",
        &[TilePos::new(2, 2), TilePos::new(3, 2), TilePos::new(2, 3)],
    ));
    let monster_data = game_data.monsters.get("goblin").unwrap();
    let creature = CreatureState::new(
        "goblin".to_string(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        42,
    );
    let entity_id = state.entities.spawn_creature(TilePos::new(1, 1), creature);

    let report = process_special_rooms(&mut state, &game_data, 1.0);

    assert!(report.barracks_marker_set.is_some());
    assert_eq!(report.barracks_grouped, 1);
    let creature = state
        .entities
        .get(entity_id)
        .and_then(|entity| entity.as_creature())
        .unwrap();
    assert_eq!(creature.current_task, state.defend_marker.map(Task::MoveTo));
}

#[test]
fn scavenger_room_converts_enemy_creature_after_progress() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let pos = TilePos::new(4, 4);
    state.room_manager.rooms.push(active_room(
        1001,
        "scavenger",
        &[pos, TilePos::new(5, 4), TilePos::new(4, 5)],
    ));

    let monster_data = game_data.monsters.get("goblin").unwrap();
    let creature = CreatureState::new(
        "goblin".to_string(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        42,
    );
    let entity_id = state
        .entities
        .spawn_creature_for_owner(pos, creature, OwnerId::RivalKeeper(1));

    let partial = process_special_rooms(&mut state, &game_data, 1.0);
    assert_eq!(partial.scavenger_progressed, 1);
    assert_eq!(partial.scavenged_creatures, 0);
    assert_eq!(
        state.entities.get(entity_id).unwrap().owner,
        OwnerId::RivalKeeper(1)
    );

    let complete = process_special_rooms(&mut state, &game_data, 10.0);
    assert_eq!(complete.scavenged_creatures, 1);
    assert_eq!(
        state.entities.get(entity_id).unwrap().owner,
        OwnerId::Player
    );
}

#[test]
fn room_object_capacity_uses_visual_density() {
    let game_data = GameData::load().expect("game data should load");
    let room = active_room(
        1002,
        "prison",
        &[
            TilePos::new(1, 1),
            TilePos::new(2, 1),
            TilePos::new(3, 1),
            TilePos::new(4, 1),
            TilePos::new(5, 1),
        ],
    );

    assert_eq!(room_object_capacity(&room, &game_data, "cell"), 1);
}
