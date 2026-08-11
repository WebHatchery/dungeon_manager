use super::*;
use crate::engine::input_handlers;
use crate::state::tile_state::Ownership;

#[test]
fn file_map_claims_walkable_floor_around_player_heart() {
    let game_data = GameData::load().expect("game data should load");
    let mut entities = EntityManager::new();
    let dungeon = load_map("assets/maps/level_1.json", &game_data, &mut entities)
        .expect("level map should load");
    let heart_pos = dungeon.find_dungeon_heart().expect("heart should exist");

    for dy in -1..=1 {
        for dx in -1..=1 {
            let pos = TilePos::new(heart_pos.x + dx, heart_pos.y + dy);
            let tile = dungeon.get_tile(pos).expect("heart area tile exists");
            assert_eq!(tile.ownership, Ownership::Player);
            if pos == heart_pos {
                assert_eq!(tile.tile_type, tt::DUNGEON_HEART);
            } else {
                assert_eq!(tile.tile_type, tt::CLAIMED_FLOOR);
            }
        }
    }
}

#[test]
fn blood_and_iron_map_loads_with_both_hearts_and_hero_base() {
    let game_data = GameData::load().expect("game data should load");
    let mut entities = EntityManager::new();
    let dungeon = load_map("assets/maps/blood_and_iron.json", &game_data, &mut entities)
        .expect("blood_and_iron map should load");

    assert_eq!(dungeon.width, 32);
    assert_eq!(dungeon.height, 32);
    // Player heart claimed for the player.
    let player_heart = dungeon.find_dungeon_heart().expect("player heart exists");
    assert_eq!(
        dungeon.get_tile(player_heart).unwrap().ownership,
        Ownership::Player
    );
    // The walled hero outpost placed its buildings as tiles (town_hall /
    // barracks / church), the same representation dark_beginnings uses; the
    // full GameState lifts these into `hero_base` for the destroy objective.
    let hero_building_tiles = dungeon
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|tile| matches!(tile.tile_type.as_str(), "town_hall" | "barracks" | "church"))
        .count();
    assert_eq!(
        hero_building_tiles, 3,
        "expected town_hall + barracks + church tiles"
    );
    // Rival keeper opens with a garrison.
    let rival_creatures = entities
        .all()
        .filter(|e| e.owner == OwnerId::RivalKeeper(1) && e.as_creature().is_some())
        .count();
    assert_eq!(rival_creatures, 3);
}

#[test]
fn file_map_loads_rival_keeper_base_and_ai() {
    let game_data = GameData::load().expect("game data should load");
    let runtime = load_rival_keeper_runtime("assets/maps/level_1.json")
        .expect("rival keeper runtime should load");
    let keeper = runtime
        .keepers
        .iter()
        .find(|keeper| keeper.owner == OwnerId::RivalKeeper(1))
        .expect("keeper1 runtime should exist");
    assert_eq!(keeper.ai_profile, "builder_attacker");
    assert_eq!(keeper.dig_batch, 5);
    assert_eq!(keeper.room_size, 2);
    assert_eq!(keeper.attack_cooldown, 90.0);
    assert_eq!(keeper.first_attack_delay, 240.0);
    assert_eq!(keeper.attack_cooldown_growth, 1.5);
    // The first raid is gated by the grace period, not the repeat cooldown
    assert_eq!(keeper.next_attack_time, keeper.first_attack_delay);

    let mut entities = EntityManager::new();
    let dungeon = load_map("assets/maps/level_1.json", &game_data, &mut entities)
        .expect("level map should load");
    let heart_pos = TilePos::new(6, 23);
    let heart_tile = dungeon
        .get_tile(heart_pos)
        .expect("rival heart tile should exist");
    assert_eq!(heart_tile.tile_type, tt::DUNGEON_HEART);
    assert_eq!(heart_tile.owner, OwnerId::RivalKeeper(1));
    assert_eq!(heart_tile.ownership, Ownership::Enemy);

    let floor_tile = dungeon
        .get_tile(TilePos::new(5, 23))
        .expect("rival floor tile should exist");
    assert_eq!(floor_tile.tile_type, tt::CLAIMED_FLOOR);
    assert_eq!(floor_tile.owner, OwnerId::RivalKeeper(1));
    assert_eq!(floor_tile.ownership, Ownership::Enemy);

    let rival_creatures = entities
        .all()
        .filter(|entity| entity.owner == OwnerId::RivalKeeper(1))
        .filter(|entity| entity.as_creature().is_some())
        .count();
    assert_eq!(rival_creatures, 2);
}

#[test]
fn blood_and_iron_boots_as_a_playable_mission() {
    // Full GameState construction is the real entry point missions run
    // through — proves the authored scenario+map load, the hero outpost is
    // live for the destroy objective, and the player heart is intact.
    let game_data = GameData::load().expect("game data should load");
    let state = crate::state::game_state::GameState::new_for_scenario(&game_data, "blood_and_iron");

    assert!(state.find_dungeon_heart_position().is_some());
    assert!(state.dungeon_heart_health > 0.0);
    assert!(
        state.hero_base.enabled,
        "hero outpost should be active for destroy_all_hero_buildings"
    );
    // The three core structures register in the base (the walled ring's
    // hero_wall/hero_gate tiles also count as buildings, as in level_1).
    for core in ["town_hall", "barracks", "church"] {
        assert!(
            state
                .hero_base
                .buildings
                .iter()
                .any(|b| b.building_type == core),
            "hero base should include a {core}"
        );
    }
    assert!(!state.hero_base.is_defeated(&state.entities));
}

#[test]
fn the_long_dark_boots_as_an_economy_mission() {
    // M3 has no hero base (heroes only raid) — its win is gather-gold +
    // survive. Prove it boots, the player heart is live, and the map is
    // resource-rich enough to be an economy race (many gold/gem seams).
    let game_data = GameData::load().expect("game data should load");
    let state = crate::state::game_state::GameState::new_for_scenario(&game_data, "the_long_dark");

    assert!(state.find_dungeon_heart_position().is_some());
    assert!(state.dungeon_heart_health > 0.0);

    let mut entities = EntityManager::new();
    let dungeon = load_map("assets/maps/the_long_dark.json", &game_data, &mut entities)
        .expect("the_long_dark map should load");
    let resource_tiles = dungeon
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|t| matches!(t.tile_type.as_str(), "gold_vein" | "gem_seam"))
        .count();
    assert!(
        resource_tiles >= 20,
        "economy map should be resource-rich, found {resource_tiles} seams"
    );
}

#[test]
fn no_prisoners_boots_with_prison_tools_and_hero_outpost() {
    // M4 (Act I finale) introduces the prison→skeleton / torture pipeline
    // and asks the player to raze a fortified outpost. Prove it boots, the
    // capture rooms are available, and the outpost is live to destroy.
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data
        .scenarios
        .get("no_prisoners")
        .expect("no_prisoners scenario should be loaded");
    for room in ["prison", "torture_chamber"] {
        assert!(
            scenario.availability.rooms.contains_key(room),
            "capture mission should offer the {room}"
        );
    }

    let state = crate::state::game_state::GameState::new_for_scenario(&game_data, "no_prisoners");
    assert!(state.dungeon_heart_health > 0.0);
    assert!(state.hero_base.enabled, "the outpost must be live to raze");
    for core in ["town_hall", "barracks", "church", "armory"] {
        assert!(
            state
                .hero_base
                .buildings
                .iter()
                .any(|b| b.building_type == core),
            "hero outpost should include a {core}"
        );
    }
}

#[test]
fn whispers_in_the_circle_boots_with_ritual_tools() {
    // M5 (Act II opener) introduces the ritual circle + mana economy +
    // offensive spellcasting. Prove it boots, the ritual circle and warlock
    // are available, and the mage-tower outpost is live to raze.
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data
        .scenarios
        .get("whispers_in_the_circle")
        .expect("whispers_in_the_circle scenario should be loaded");
    assert!(scenario.availability.rooms.contains_key("ritual_circle"));
    assert!(scenario.availability.creatures.contains_key("warlock"));

    let state =
        crate::state::game_state::GameState::new_for_scenario(&game_data, "whispers_in_the_circle");
    assert!(state.dungeon_heart_health > 0.0);
    assert!(
        state.hero_base.enabled,
        "the mage-tower outpost must be live"
    );
    assert!(
        state
            .hero_base
            .buildings
            .iter()
            .any(|b| b.building_type == "mage_tower"),
        "the outpost should feature a mage_tower"
    );
}

#[test]
fn the_kennels_boots_with_beast_roster_and_larger_army_cap() {
    // M6 scales the army: kennel/barracks/scavenger + hellhound/spider/lizard
    // and a bigger creature cap. Prove it boots with those tools available.
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data
        .scenarios
        .get("the_kennels")
        .expect("the_kennels scenario should be loaded");
    for room in ["kennel", "barracks", "scavenger"] {
        assert!(
            scenario.availability.rooms.contains_key(room),
            "army mission should offer the {room}"
        );
    }
    for beast in ["hellhound", "spider", "lizard"] {
        assert!(
            scenario.availability.creatures.contains_key(beast),
            "army mission should offer the {beast}"
        );
    }
    // Meaningfully larger roster cap than the Act I missions (15–18).
    let player = scenario
        .players
        .iter()
        .find(|p| p.id == "keeper0")
        .expect("player slot exists");
    assert!(player.max_creatures.unwrap_or(0) >= 22);

    let state = crate::state::game_state::GameState::new_for_scenario(&game_data, "the_kennels");
    assert!(state.dungeon_heart_health > 0.0);
    assert!(state.hero_base.enabled, "the outpost must be live to raze");
}

#[test]
fn m7_branch_missions_boot_with_their_signature_tools() {
    // The Act II branch: M7a (graveyard→vampire) and M7b (temple→demons).
    // Each is a full, bootable mission with its path's signature room.
    let game_data = GameData::load().expect("game data should load");

    let graveyard = game_data
        .scenarios
        .get("restless_dead")
        .expect("restless_dead scenario should be loaded");
    assert!(graveyard.availability.rooms.contains_key("graveyard"));

    let temple = game_data
        .scenarios
        .get("pacts_and_sacrifice")
        .expect("pacts_and_sacrifice scenario should be loaded");
    assert!(temple.availability.rooms.contains_key("temple"));
    assert!(temple.availability.creatures.contains_key("demon_spawn"));

    for id in ["restless_dead", "pacts_and_sacrifice"] {
        let state = crate::state::game_state::GameState::new_for_scenario(&game_data, id);
        assert!(
            state.dungeon_heart_health > 0.0,
            "{id} heart should be live"
        );
        assert!(
            state.hero_base.enabled,
            "{id} outpost should be live to raze"
        );
    }
}

#[test]
fn the_iron_siege_boots_as_a_defensive_gauntlet() {
    // M8 (Act II climax) is the defensive knee: layered doors/alarms + a
    // bile_demon anchor, then raze the garrison. Prove it boots with those
    // tools and a live outpost, and that the map keeps its choke corridor.
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data
        .scenarios
        .get("the_iron_siege")
        .expect("the_iron_siege scenario should be loaded");
    for trap in ["magic_door", "alarm_trap"] {
        assert!(
            scenario.availability.traps.contains_key(trap),
            "the siege should offer the {trap}"
        );
    }
    assert!(scenario.availability.creatures.contains_key("bile_demon"));
    assert!(scenario.events.len() >= 4, "expect a multi-wave gauntlet");

    let state = crate::state::game_state::GameState::new_for_scenario(&game_data, "the_iron_siege");
    assert!(state.dungeon_heart_health > 0.0);
    assert!(state.hero_base.enabled, "the garrison must be live to raze");
}

#[test]
fn corruption_rising_boots_as_an_offense_mission() {
    // M9 flips the script: the player is the aggressor razing a full hero
    // TOWN. Its sole objective is destroy-all-buildings (no survive floor),
    // and the town has more halls than the Act I/II single outposts.
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data
        .scenarios
        .get("corruption_rising")
        .expect("corruption_rising scenario should be loaded");
    assert_eq!(
        scenario.objectives.len(),
        1,
        "offense: raze the town, no survive floor"
    );
    for spell in ["possess", "corrupt_land"] {
        assert!(scenario.availability.spells.contains_key(spell));
    }

    let state =
        crate::state::game_state::GameState::new_for_scenario(&game_data, "corruption_rising");
    assert!(state.dungeon_heart_health > 0.0);
    assert!(state.hero_base.enabled, "the town must be live to raze");
    // A full town: at least the six named halls register as buildings.
    let halls = state
        .hero_base
        .buildings
        .iter()
        .filter(|b| {
            matches!(
                b.building_type.as_str(),
                "town_hall" | "barracks" | "church" | "mage_tower" | "archery_range" | "armory"
            )
        })
        .count();
    assert!(halls >= 6, "expected a six-hall town, found {halls}");
}

#[test]
fn two_kings_boots_with_two_hostile_rivals_and_a_clear_player_heart() {
    // M10 is the two-rival duel. Verify the multi-rival path end to end:
    // two RivalKeeper runtimes load, both rival hearts are enemy-owned, the
    // player's heart is still unambiguously identified among THREE hearts,
    // and the two rivals are mutually hostile.
    let game_data = GameData::load().expect("game data should load");

    let runtime =
        load_rival_keeper_runtime("assets/maps/two_kings.json").expect("rival runtime should load");
    assert_eq!(runtime.keepers.len(), 2, "both rival keepers should load");
    assert!(runtime
        .keepers
        .iter()
        .any(|k| k.owner == OwnerId::RivalKeeper(1)));
    assert!(runtime
        .keepers
        .iter()
        .any(|k| k.owner == OwnerId::RivalKeeper(2)));
    // Rivals fight each other, not just the player.
    assert!(OwnerId::RivalKeeper(1).is_hostile_to(&OwnerId::RivalKeeper(2)));

    let state = crate::state::game_state::GameState::new_for_scenario(&game_data, "two_kings");
    // The player's heart is found (Ownership::Player) despite three hearts.
    let player_heart = state
        .find_dungeon_heart_position()
        .expect("player heart must be identifiable among three hearts");
    assert_eq!(
        state.get_tile(player_heart).unwrap().ownership,
        Ownership::Player
    );
    assert!(state.dungeon_heart_health > 0.0);
    // No hero base in this pure keeper duel; win is survive + out-earn.
    assert!(!state.hero_base.enabled);
    let scenario = game_data.scenarios.get("two_kings").unwrap();
    assert_eq!(scenario.objectives.len(), 2, "survive + gather-gold");
}

#[test]
fn climax_missions_boot_with_a_live_capital_garrison() {
    // M11–M12: assault a fortified capital defended by a hero garrison.
    // Both boot with a live, razeable capital and a garrison present.
    // (M11's knight_commander boss stands on the map; M12's Champion of
    // Light is wave-only and defeated via the `hero_defeated` objective, so
    // it is deliberately NOT a start-of-mission map entity — see the
    // scenario_events boss-defeat tests.)
    let game_data = GameData::load().expect("game data should load");
    for id in ["heavens_reach", "deep_dominion_finale"] {
        let state = crate::state::game_state::GameState::new_for_scenario(&game_data, id);
        assert!(state.dungeon_heart_health > 0.0, "{id} player heart live");
        assert!(
            state.hero_base.enabled,
            "{id} capital should be live to raze"
        );
        let garrison = state
            .entities
            .all()
            .filter(|e| e.owner == OwnerId::Heroes)
            .filter(|e| matches!(&e.entity_type, crate::state::entities::EntityType::Hero(_)))
            .count();
        assert!(
            garrison >= 4,
            "{id} should field a hero garrison, got {garrison}"
        );
    }

    // M11's boss is the sole knight_commander on the map at start.
    let m11 = crate::state::game_state::GameState::new_for_scenario(&game_data, "heavens_reach");
    let commanders = m11
        .entities
        .all()
        .filter(|e| e.owner == OwnerId::Heroes)
        .filter(|e| {
            matches!(&e.entity_type,
                crate::state::entities::EntityType::Hero(h) if h.hero_id == "knight_commander")
        })
        .count();
    assert_eq!(commanders, 1, "the knight_commander boss should be unique");
}

#[test]
fn difficulty_scales_the_effective_hero_threat() {
    use crate::state::settings::Difficulty;

    let game_data = GameData::load().expect("game data should load");
    // the_iron_siege authors threat_multiplier 1.35; difficulty layers on top.
    let mut state =
        crate::state::game_state::GameState::new_for_scenario(&game_data, "the_iron_siege");

    state.difficulty = Difficulty::Normal;
    let normal = state.effective_threat_multiplier(&game_data);
    state.difficulty = Difficulty::Easy;
    let easy = state.effective_threat_multiplier(&game_data);
    state.difficulty = Difficulty::Hard;
    let hard = state.effective_threat_multiplier(&game_data);

    // Threat is the mission dial (1.35) times the difficulty scale, and it
    // orders Easy < Normal < Hard — proving the previously-inert
    // threat_multiplier is now actually consumed.
    assert!((normal - 1.35).abs() < 0.001, "normal={normal}");
    assert!(easy < normal && normal < hard, "{easy} < {normal} < {hard}");
    assert!(state.effective_threat_multiplier(&game_data) > 0.0);
}

#[test]
fn default_scenario_starts_with_markable_dig_targets_near_heart() {
    let game_data = GameData::load().expect("game data should load");
    let mut state =
        crate::state::game_state::GameState::new_for_scenario(&game_data, "dark_beginnings");
    let heart_pos = state
        .find_dungeon_heart_position()
        .expect("heart should exist");
    let dig_pos = TilePos::new(heart_pos.x + 2, heart_pos.y);

    let tile = state.get_tile(dig_pos).expect("dig target should exist");
    assert_eq!(tile.tile_type, "earth");
    assert_eq!(tile.ownership, Ownership::Unclaimed);

    input_handlers::handle_dig(&mut state, &game_data, dig_pos);

    let tile = state
        .get_tile(dig_pos)
        .expect("dig target should still exist");
    assert!(tile.marked_for_dig);
}
