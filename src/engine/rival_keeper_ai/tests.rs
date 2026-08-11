use super::*;

fn clear_rival_keeper_area(state: &mut GameState, owner: &OwnerId) {
    for row in &mut state.dungeon.grid {
        for tile in row {
            if &tile.owner == owner {
                tile.set_owner(OwnerId::Neutral);
                if tile.tile_type == tt::DUNGEON_HEART {
                    tile.tile_type = "earth".to_string();
                }
            }
        }
    }
}

fn remove_rival_keeper_creatures(state: &mut GameState, owner: &OwnerId) {
    let ids: Vec<EntityId> = state
        .entities
        .all()
        .filter(|entity| &entity.owner == owner)
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        state.entities.remove(id);
    }
}

fn set_tile(state: &mut GameState, pos: TilePos, tile_type: &str, owner: OwnerId) {
    let tile = state
        .dungeon
        .get_tile_mut(pos)
        .expect("test tile should exist");
    tile.tile_type = tile_type.to_string();
    tile.resources_remaining = None;
    tile.room_id = None;
    tile.marked_for_dig = false;
    tile.set_owner(owner);
}

#[test]
fn rival_keeper_plan_requests_reinforcements() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let owner = OwnerId::RivalKeeper(1);
    remove_rival_keeper_creatures(&mut state, &owner);
    state.rival_keepers.keepers[0].next_decision_time = 0.0;
    state.rival_keepers.keepers[0].desired_rooms.clear();
    state.dungeon.grid[1][1].set_owner(owner.clone());

    let plans = plan_rival_keepers(&state);
    assert!(plans[0].wants_reinforcements);

    let before_count = state
        .entities
        .all()
        .filter(|entity| entity.owner == owner)
        .count();
    update_rival_keeper_ai(&mut state, &game_data, 1.0);
    let rival_count = state
        .entities
        .all()
        .filter(|entity| entity.owner == owner)
        .count();
    assert!(rival_count > before_count);
}

#[test]
fn rival_keeper_launches_attack_when_garrison_is_ready() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let owner = OwnerId::RivalKeeper(1);
    remove_rival_keeper_creatures(&mut state, &owner);
    state.rival_keepers.keepers[0].next_decision_time = 0.0;
    state.rival_keepers.keepers[0].next_attack_time = 0.0;
    state.rival_keepers.keepers[0].desired_rooms.clear();
    state.dungeon.grid[1][1].set_owner(owner.clone());
    let min_garrison = state.rival_keepers.keepers[0].min_garrison;
    let raid_size = state.rival_keepers.keepers[0].raid_size;
    let monster_data = game_data.monsters.get("goblin").unwrap();
    for offset in 0..min_garrison {
        let creature = CreatureState::new(
            "goblin".to_string(),
            1,
            monster_data.stats.health,
            monster_data.stats.mana,
            offset as u64,
        );
        state.entities.spawn_creature_for_owner(
            TilePos::new(1 + offset as i32, 1),
            creature,
            owner.clone(),
        );
    }
    let heart_pos = state.find_dungeon_heart_position().unwrap();

    let plans = plan_rival_keepers(&state);
    assert!(plans[0].wants_attack);
    update_rival_keeper_ai(&mut state, &game_data, 1.0);

    let attackers = state
        .entities
        .all()
        .filter(|entity| entity.owner == owner)
        .filter_map(|entity| entity.as_creature())
        .filter(|creature| creature.current_task == Some(Task::MoveTo(heart_pos)))
        .count();
    assert_eq!(attackers, raid_size);
    assert!(state.rival_keepers.keepers[0].next_attack_time > 0.0);
}

#[test]
fn rival_keeper_first_attack_waits_for_grace_period_then_gaps_grow() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let owner = OwnerId::RivalKeeper(1);

    // The opening raid is gated by the grace period, not the repeat cooldown
    let keeper = &state.rival_keepers.keepers[0];
    assert_eq!(keeper.next_attack_time, keeper.first_attack_delay);
    assert!(keeper.first_attack_delay > keeper.attack_cooldown);

    // Force a raid and check the cooldown escalates for the next one
    remove_rival_keeper_creatures(&mut state, &owner);
    state.rival_keepers.keepers[0].next_decision_time = 0.0;
    state.rival_keepers.keepers[0].next_attack_time = 0.0;
    state.rival_keepers.keepers[0].desired_rooms.clear();
    state.dungeon.grid[1][1].set_owner(owner.clone());
    let min_garrison = state.rival_keepers.keepers[0].min_garrison;
    let cooldown_before = state.rival_keepers.keepers[0].attack_cooldown;
    let growth = state.rival_keepers.keepers[0].attack_cooldown_growth;
    assert!(growth > 1.0);
    let monster_data = game_data.monsters.get("goblin").unwrap();
    for offset in 0..min_garrison {
        let creature = CreatureState::new(
            "goblin".to_string(),
            1,
            monster_data.stats.health,
            monster_data.stats.mana,
            offset as u64,
        );
        state.entities.spawn_creature_for_owner(
            TilePos::new(1 + offset as i32, 1),
            creature,
            owner.clone(),
        );
    }

    update_rival_keeper_ai(&mut state, &game_data, 1.0);

    let keeper = &state.rival_keepers.keepers[0];
    assert_eq!(keeper.attack_cooldown, cooldown_before * growth);
    assert_eq!(keeper.next_attack_time, keeper.attack_cooldown);
}

#[test]
fn rival_keeper_digs_owned_expansion() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let owner = OwnerId::RivalKeeper(1);
    clear_rival_keeper_area(&mut state, &owner);

    let owned = TilePos::new(2, 2);
    let target = TilePos::new(3, 2);
    set_tile(&mut state, owned, tt::CLAIMED_FLOOR, owner.clone());
    set_tile(&mut state, target, "earth", OwnerId::Neutral);
    set_tile(&mut state, TilePos::new(1, 2), "bedrock", OwnerId::Neutral);
    set_tile(&mut state, TilePos::new(2, 1), "bedrock", OwnerId::Neutral);
    set_tile(&mut state, TilePos::new(2, 3), "bedrock", OwnerId::Neutral);

    assert!(dig_expansion(&mut state, &game_data, &owner, 1));

    let tile = state.dungeon.get_tile(target).expect("target should exist");
    assert_eq!(tile.tile_type, tt::CLAIMED_FLOOR);
    assert_eq!(tile.owner, owner);
}

#[test]
fn rival_keeper_places_desired_room_on_owned_floor() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let owner = OwnerId::RivalKeeper(1);
    clear_rival_keeper_area(&mut state, &owner);

    let room_tiles = [
        TilePos::new(2, 2),
        TilePos::new(3, 2),
        TilePos::new(2, 3),
        TilePos::new(3, 3),
    ];
    for pos in room_tiles {
        set_tile(&mut state, pos, tt::CLAIMED_FLOOR, owner.clone());
    }

    assert!(place_room(&mut state, &game_data, &owner, "lair", 2));

    for pos in room_tiles {
        let tile = state.dungeon.get_tile(pos).expect("room tile should exist");
        assert_eq!(tile.tile_type, "lair");
        assert_eq!(tile.owner, owner);
        assert_eq!(tile.ownership, Ownership::Enemy);
    }
}

#[test]
fn rival_keeper_defends_owned_area_against_nearby_threat() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let owner = OwnerId::RivalKeeper(1);
    remove_rival_keeper_creatures(&mut state, &owner);
    clear_rival_keeper_area(&mut state, &owner);
    set_tile(
        &mut state,
        TilePos::new(2, 2),
        tt::CLAIMED_FLOOR,
        owner.clone(),
    );

    let monster_data = game_data.monsters.get("goblin").unwrap();
    let defender = CreatureState::new(
        "goblin".to_string(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        1,
    );
    state
        .entities
        .spawn_creature_for_owner(TilePos::new(2, 2), defender, owner.clone());
    let threat = CreatureState::new(
        "goblin".to_string(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        2,
    );
    let threat_pos = TilePos::new(3, 2);
    state
        .entities
        .spawn_creature_for_owner(threat_pos, threat, OwnerId::Player);

    let keeper = state.rival_keepers.keepers[0].clone();
    assert!(assign_defenders_to_threat(&mut state, &keeper));

    let defenders = state
        .entities
        .all()
        .filter(|entity| entity.owner == owner)
        .filter_map(|entity| entity.as_creature())
        .filter(|creature| creature.current_task == Some(Task::MoveTo(threat_pos)))
        .count();
    assert_eq!(defenders, 1);
}
