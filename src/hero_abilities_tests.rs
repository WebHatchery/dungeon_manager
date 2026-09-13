use crate::data::GameData;
use crate::engine::hero_abilities::update_hero_abilities;
use crate::engine::room_validator::Room;
use crate::state::entities::{CreatureState, HeroState};
use crate::state::game_state::GameState;
use crate::state::tile_state::{FogState, TilePos};
use std::collections::HashSet;

#[test]
fn passive_ability_fires_and_sets_cooldown() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);
    game_state.entities = crate::state::entities::EntityManager::new();

    // scout's "detect_traps" ability: passive -> reveal_map around itself
    let hero = HeroState::new(
        "scout".to_string(),
        1,
        100.0,
        10.0,
        TilePos::new(5, 5),
        1.0,
        1,
    );
    let hero_id = game_state.entities.spawn_hero(TilePos::new(5, 5), hero);

    let far_pos = TilePos::new(5, 12);
    if let Some(tile) = game_state.get_tile_mut(far_pos) {
        tile.fog_state = FogState::Hidden;
    }

    update_hero_abilities(&mut game_state, &game_data, 1.0);

    let tile = game_state.get_tile(far_pos).unwrap();
    assert_eq!(
        tile.fog_state,
        FogState::Visible,
        "detect_traps should reveal the map around the hero"
    );

    let hero = game_state.entities.get(hero_id).unwrap().as_hero().unwrap();
    assert!(
        hero.ability_cooldowns.contains_key("detect_traps"),
        "firing the ability should start its cooldown"
    );

    // Cooldown should prevent it from firing again immediately.
    if let Some(tile) = game_state.get_tile_mut(far_pos) {
        tile.fog_state = FogState::Hidden;
    }
    update_hero_abilities(&mut game_state, &game_data, 0.1);
    let tile = game_state.get_tile(far_pos).unwrap();
    assert_eq!(
        tile.fog_state,
        FogState::Hidden,
        "ability on cooldown should not fire again"
    );
}

#[test]
fn on_low_health_ability_heals_self() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);

    // knight_commander's "rally" ability: on_low_health -> heal self
    let mut hero = HeroState::new(
        "knight_commander".to_string(),
        1,
        100.0,
        10.0,
        TilePos::new(5, 5),
        1.0,
        1,
    );
    hero.health = 20.0; // well under the low-health threshold
    game_state.entities.spawn_hero(TilePos::new(5, 5), hero);

    update_hero_abilities(&mut game_state, &game_data, 1.0);

    let health_after = game_state
        .entities
        .all()
        .find_map(|e| e.as_hero())
        .unwrap()
        .health;
    assert!(
        health_after > 20.0,
        "rally should heal a hero below the low-health threshold"
    );
}

#[test]
fn in_room_ability_only_fires_in_matching_room_type() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);
    game_state.entities = crate::state::entities::EntityManager::new();

    // rogue's "sabotage" ability: trigger "in_room:workshop" -> damage
    let hero = HeroState::new(
        "rogue".to_string(),
        1,
        100.0,
        10.0,
        TilePos::new(3, 3),
        1.0,
        1,
    );
    let hero_id = game_state.entities.spawn_hero(TilePos::new(3, 3), hero);
    // Keep the unrelated opening-strike trigger from changing this room-only
    // test; the ability system uses the live damage timestamp as its stealth
    // signal.
    game_state.entities.get_mut(hero_id).unwrap().last_damage_time = 0.0;

    let mut creature = CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 2);
    creature.health = 100.0;
    let creature_id = game_state
        .entities
        .spawn_creature(TilePos::new(3, 3), creature);

    // Not in a workshop yet: sabotage shouldn't fire.
    update_hero_abilities(&mut game_state, &game_data, 1.0);
    let health_before = game_state
        .entities
        .get(creature_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .health;
    assert_eq!(
        health_before, 100.0,
        "sabotage shouldn't fire outside a workshop"
    );
    assert!(game_state
        .entities
        .get(hero_id)
        .unwrap()
        .as_hero()
        .unwrap()
        .ability_cooldowns
        .is_empty());

    // Now place a workshop under the hero.
    let mut room = Room::new(
        1,
        "workshop".to_string(),
        [TilePos::new(3, 3)].into_iter().collect::<HashSet<_>>(),
        Vec::new(),
    );
    room.active = true;
    game_state.room_manager.rooms.push(room);

    update_hero_abilities(&mut game_state, &game_data, 1.0);
    let health_after = game_state
        .entities
        .get(creature_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .health;
    assert!(
        health_after < 100.0,
        "sabotage should deal damage once the hero is standing in a workshop"
    );
}

#[test]
fn undead_trait_drives_turn_undead_ability() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);

    // paladin's "turn_undead" ability: trigger "on_undead_nearby" -> stun status
    let hero = HeroState::new(
        "paladin".to_string(),
        1,
        100.0,
        10.0,
        TilePos::new(4, 4),
        1.0,
        1,
    );
    game_state.entities.spawn_hero(TilePos::new(4, 4), hero);

    // skeleton is authored with the "undead" trait (see traits.json / monsters.json)
    let skeleton = CreatureState::new("skeleton".to_string(), 1, 90.0, 0.0, 2);
    let skeleton_id = game_state
        .entities
        .spawn_creature(TilePos::new(4, 5), skeleton);

    update_hero_abilities(&mut game_state, &game_data, 1.0);

    let skeleton = game_state
        .entities
        .get(skeleton_id)
        .unwrap()
        .as_creature()
        .unwrap();
    assert!(
        skeleton
            .status_effects
            .iter()
            .any(|e| e.effect_type == "stun"),
        "turn_undead should stun a nearby undead creature"
    );
}

#[test]
fn authored_environment_and_stealth_triggers_fire() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);
    game_state.entities = crate::state::entities::EntityManager::new();

    let wizard_pos = TilePos::new(4, 4);
    let wizard_id = game_state.entities.spawn_hero(
        wizard_pos,
        HeroState::new("wizard".to_string(), 1, 100.0, 10.0, wizard_pos, 1.0, 1),
    );
    game_state.dungeon.get_tile_mut(wizard_pos).unwrap().trap = Some(
        crate::state::tile_state::TrapState {
            trap_type: "spike_trap".to_string(),
            constructed: true,
            construction_progress: 1.0,
            active: true,
            funded: true,
            cooldown: 0.0,
            triggered: false,
        },
    );
    update_hero_abilities(&mut game_state, &game_data, 1.0);
    let wizard = game_state.entities.get(wizard_id).unwrap().as_hero().unwrap();
    assert!(wizard.ability_cooldowns.contains_key("teleport"));
    assert!(wizard.movement_speed > 1.5);

    let cleric_pos = TilePos::new(8, 8);
    let cleric_id = game_state.entities.spawn_hero(
        cleric_pos,
        HeroState::new(
            "battle_cleric".to_string(),
            1,
            100.0,
            10.0,
            cleric_pos,
            1.0,
            2,
        ),
    );
    game_state.dungeon.get_tile_mut(cleric_pos).unwrap().tile_type =
        "corrupted_floor".to_string();
    update_hero_abilities(&mut game_state, &game_data, 1.0);
    assert!(game_state
        .entities
        .get(cleric_id)
        .unwrap()
        .as_hero()
        .unwrap()
        .status_effects
        .iter()
        .any(|effect| effect.effect_type == "purified"));

    let rogue_pos = TilePos::new(12, 12);
    let rogue_id = game_state.entities.spawn_hero(
        rogue_pos,
        HeroState::new("rogue".to_string(), 1, 100.0, 10.0, rogue_pos, 1.0, 3),
    );
    let creature_id = game_state.entities.spawn_creature(
        TilePos::new(12, 13),
        CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 4),
    );
    update_hero_abilities(&mut game_state, &game_data, 1.0);
    assert!(game_state
        .entities
        .get(rogue_id)
        .unwrap()
        .as_hero()
        .unwrap()
        .ability_cooldowns
        .contains_key("backstab"));
    assert!(game_state
        .entities
        .get(creature_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .health
        < 100.0);
}
