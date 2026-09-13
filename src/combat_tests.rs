use crate::data::GameData;
use crate::engine::combat;
use crate::state::entities::{CreatureState, HeroState, StatusEffect, Task};
use crate::state::game_state::GameState;
use crate::state::tile_state::TilePos;

#[test]
fn test_ranged_monster_spawns_projectile() {
    // 1. Setup Game
    // Ensure we can find assets.
    // Cargo test runs in workspace root usually.
    let game_data = GameData::load().expect("Failed to load game data");
    let mut game_state = GameState::new(20, 20, &game_data);

    // Clear all default entities (walls, etc.) to ensure a clean test environment
    // This prevents the Warlock from targeting a closer wall instead of the Hero
    game_state.entities = crate::state::entities::EntityManager::new();

    // Set all tiles to floor to ensure Line of Sight is not blocked by residual wall tiles
    let (w, h) = crate::engine::tile_grid::get_grid_dimensions(&game_state.dungeon.grid);
    for y in 0..h {
        for x in 0..w {
            let pos = TilePos::new(x as i32, y as i32);
            if let Some(tile) =
                crate::engine::tile_grid::get_tile_mut(&mut game_state.dungeon.grid, pos)
            {
                tile.tile_type = "floor".to_string();
            }
        }
    }

    // 2. Spawn Ranged Monster (Warlock)
    let warlock_pos = TilePos::new(5, 5);
    let mut warlock_state = CreatureState::new("warlock".to_string(), 1, 100.0, 100.0, 12345);
    // Ensure warlock is happy enough not to desert
    warlock_state.mood = 100.0;

    let warlock_id = game_state
        .entities
        .spawn_creature(warlock_pos, warlock_state);

    // 3. Spawn Hero target
    let hero_pos = TilePos::new(5, 8); // Within range (distance 3)
    let hero_state = HeroState::new("knight".to_string(), 1, 100.0, 50.0, hero_pos, 5.0, 54321);
    let hero_id = game_state.entities.spawn_hero(hero_pos, hero_state);

    // 4. Run update to trigger AI decision
    // Step 1: Decision phase
    game_state.update(0.1, &game_data);

    // Verify Warlock selected Attack task
    let warlock = game_state
        .entities
        .get(warlock_id)
        .unwrap()
        .as_creature()
        .unwrap();

    // Debug info
    println!(
        "Warlock (ID {}) Task: {:?}",
        warlock_id, warlock.current_task
    );
    println!("Hero ID: {}", hero_id);
    println!("Entities in game: {}", game_state.entities.count());
    for entity in game_state.entities.all() {
        println!(
            "Entity {}: Type={:?}, Pos={:?}",
            entity.id, entity.entity_type, entity.pos
        );
    }

    if let Some(Task::Attack(target_id)) = warlock.current_task.clone() {
        if target_id != hero_id {
            let target = game_state.entities.get(target_id).unwrap();
            println!(
                "WRONG TARGET: Targeted Entity {} ({:?}) at {:?}",
                target_id, target.entity_type, target.pos
            );
        }
        assert_eq!(target_id, hero_id, "Warlock should target the hero");
    } else {
        panic!(
            "Warlock did not select Attack task. Current: {:?}",
            warlock.current_task
        );
    }

    // Step 2: Combat phase. Resolve a deterministic full attack interval
    // instead of relying on probabilistic fixed-timestep combat.
    let attacker = game_state.entities.get(warlock_id).unwrap().clone();
    let defender = game_state.entities.get(hero_id).unwrap().clone();
    let result =
        combat::resolve_combat_tick(&attacker, &defender, 1.0, &game_data, 0.0, (0.0, 0.0));

    if let Some((projectile_type, damage)) = result.projectile_spawned {
        game_state.projectiles.spawn(
            attacker.visual_pos,
            defender.visual_pos,
            &projectile_type,
            warlock_id,
            hero_id,
            damage,
        );
    } else {
        panic!("Ranged warlock attack should spawn a projectile");
    }

    // 5. Verify Projectile Spawned
    let projectiles: Vec<_> = game_state.projectiles.active_projectiles().collect();
    assert!(
        !projectiles.is_empty(),
        "Projectile should have been spawned"
    );

    let projectile = &projectiles[0].payload;
    assert_eq!(projectile.attacker_id, warlock_id);
    assert_eq!(projectile.defender_id, hero_id);
    assert!(projectile.damage > 0.0, "Projectile should carry damage");

    println!(
        "Projectile spawned: Type={:?}, Damage={}",
        projectile.projectile_type, projectile.damage
    );

    // 6. Projectile Impact
    // Projectile duration for Magic is 0.4s. Advance only the projectile
    // manager so this assertion is not affected by a new combat tick.
    let impacts = game_state.projectiles.update(0.5);
    for impact in impacts {
        crate::engine::combat::apply_projectile_impact(
            &impact,
            &mut game_state.entities,
            &game_data,
            game_state.time_elapsed,
        );
    }

    // 7. Verify Damage
    let hero = game_state.entities.get(hero_id).unwrap().as_hero().unwrap();
    assert!(
        hero.health < 100.0,
        "Hero should have taken damage. Health: {}",
        hero.health
    );

    // 8. Verify Projectile Removed
    assert!(
        game_state.projectiles.active_projectiles().next().is_none(),
        "Projectile should be removed after impact"
    );
}

#[test]
fn poison_and_burn_status_effects_deal_damage_over_time() {
    let game_data = GameData::load().expect("Failed to load game data");
    let mut game_state = GameState::new(20, 20, &game_data);

    let mut creature = CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 1);
    creature.status_effects.push(StatusEffect {
        effect_type: "poison".to_string(),
        duration: 3.0,
        strength: 5.0, // 5 damage/sec
    });
    let creature_id = game_state
        .entities
        .spawn_creature(TilePos::new(1, 1), creature);

    let entity = game_state.entities.get_mut(creature_id).unwrap();
    combat::update_status_effects(entity, 1.0);
    let after_one_tick = entity.as_creature().unwrap().health;
    assert_eq!(after_one_tick, 95.0, "poison should deal 5 dmg/sec");
    assert_eq!(entity.as_creature().unwrap().status_effects.len(), 1);

    // Advance past the remaining duration (2s left): effect should expire and stop ticking.
    combat::update_status_effects(entity, 2.5);
    let after_expiry = entity.as_creature().unwrap().health;
    assert!(
        entity.as_creature().unwrap().status_effects.is_empty(),
        "poison effect should be removed once its duration elapses"
    );

    combat::update_status_effects(entity, 1.0);
    assert_eq!(
        entity.as_creature().unwrap().health,
        after_expiry,
        "expired poison should no longer deal damage"
    );
}

#[test]
fn freeze_status_effect_slows_movement_and_reverts_on_expiry() {
    let game_data = GameData::load().expect("Failed to load game data");
    let mut game_state = GameState::new(20, 20, &game_data);

    let attacker = CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 1);
    let defender = CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 2);
    let base_speed = defender.movement_speed;

    let attacker_id = game_state
        .entities
        .spawn_creature(TilePos::new(1, 1), attacker);
    let defender_id = game_state
        .entities
        .spawn_creature(TilePos::new(2, 2), defender);

    let result = combat::CombatResult {
        damage_dealt: 0.0,
        status_applied: vec![StatusEffect {
            effect_type: "freeze".to_string(),
            duration: 2.0,
            strength: 0.5, // 50% slow
        }],
        attacker_status_applied: Vec::new(),
        defender_died: false,
        projectile_spawned: None,
    };
    combat::apply_combat_result(
        &result,
        attacker_id,
        defender_id,
        game_state.entities.entities_mut(),
        &game_data,
        0.0,
    );

    let slowed_speed = game_state
        .entities
        .get(defender_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .movement_speed;
    assert_eq!(
        slowed_speed,
        base_speed * 0.5,
        "freeze should immediately slow movement"
    );

    // Tick past the freeze's duration: speed should revert exactly to baseline.
    let entity = game_state.entities.get_mut(defender_id).unwrap();
    combat::update_status_effects(entity, 3.0);
    let reverted_speed = entity.as_creature().unwrap().movement_speed;
    assert_eq!(
        reverted_speed, base_speed,
        "speed should revert once freeze expires"
    );
}

#[test]
fn authored_monster_abilities_modify_combat_stats() {
    let mut game_data = GameData::load().expect("game data should load");

    let mut entities = crate::state::entities::EntityManager::new();
    let orc_id = entities.spawn_creature(
        TilePos::new(1, 1),
        CreatureState::new("orc".to_string(), 1, 100.0, 20.0, 1),
    );
    let charged = combat::extract_combat_stats(entities.get(orc_id).unwrap(), &game_data);
    game_data
        .monsters
        .get_mut("orc")
        .unwrap()
        .combat
        .abilities
        .clear();
    let uncharged = combat::extract_combat_stats(entities.get(orc_id).unwrap(), &game_data);
    assert_eq!(charged.attack, uncharged.attack * 1.5);

    let demon_id = entities.spawn_creature(
        TilePos::new(2, 1),
        CreatureState::new("demon_spawn".to_string(), 1, 100.0, 20.0, 2),
    );
    let demon_full = combat::extract_combat_stats(entities.get(demon_id).unwrap(), &game_data);
    entities
        .get_mut(demon_id)
        .unwrap()
        .as_creature_mut()
        .unwrap()
        .health = 40.0;
    let demon_enraged = combat::extract_combat_stats(entities.get(demon_id).unwrap(), &game_data);
    assert!(demon_enraged.attack > demon_full.attack);
    assert!(demon_enraged.attack_speed > demon_full.attack_speed);
}

#[test]
fn projectile_impacts_apply_and_expire_speed_statuses() {
    let game_data = GameData::load().expect("game data should load");
    let mut entities = crate::state::entities::EntityManager::new();
    let attacker_id = entities.spawn_creature(
        TilePos::new(1, 1),
        CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 1),
    );
    let defender_id = entities.spawn_creature(
        TilePos::new(2, 1),
        CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 2),
    );
    let base_speed = entities
        .get(defender_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .movement_speed;
    let impact = crate::state::projectiles::Impact {
        attacker_id,
        defender_id,
        damage: 0.0,
        status_effects: vec![StatusEffect {
            effect_type: "speed_modifier".to_string(),
            duration: 1.0,
            strength: 1.5,
        }],
    };
    combat::apply_projectile_impact(&impact, &mut entities, &game_data, 0.0);
    let defender = entities.get_mut(defender_id).unwrap();
    assert_eq!(
        defender.as_creature().unwrap().movement_speed,
        base_speed * 1.5
    );
    combat::update_status_effects(defender, 1.1);
    assert_eq!(defender.as_creature().unwrap().movement_speed, base_speed);
}

#[test]
fn stunned_attacker_cannot_land_an_attack() {
    let game_data = GameData::load().expect("Failed to load game data");
    let mut game_state = GameState::new(20, 20, &game_data);

    let mut attacker = CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 1);
    attacker.status_effects.push(StatusEffect {
        effect_type: "stun".to_string(),
        duration: 1.0,
        strength: 0.0,
    });
    let defender = CreatureState::new("goblin".to_string(), 1, 100.0, 10.0, 2);

    let attacker_id = game_state
        .entities
        .spawn_creature(TilePos::new(1, 1), attacker);
    let defender_id = game_state
        .entities
        .spawn_creature(TilePos::new(1, 2), defender);

    let attacker_entity = game_state.entities.get(attacker_id).unwrap();
    let defender_entity = game_state.entities.get(defender_id).unwrap();
    let result = combat::resolve_combat_tick(
        attacker_entity,
        defender_entity,
        10.0,
        &game_data,
        0.0,
        (0.0, 0.0),
    );

    assert_eq!(
        result.damage_dealt, 0.0,
        "a stunned attacker cannot deal damage"
    );
    assert!(
        result.projectile_spawned.is_none(),
        "a stunned attacker cannot spawn a projectile either"
    );
}

/// A single active room of `room_type` covering `pos`.
fn push_room(game_state: &mut GameState, id: usize, room_type: &str, pos: TilePos) {
    let mut room = crate::engine::room_validator::Room::new(
        id,
        room_type.to_string(),
        [pos].into_iter().collect(),
        Vec::new(),
    );
    room.active = true;
    game_state.room_manager.rooms.push(room);
}

fn spawn_goblin(game_state: &mut GameState, game_data: &GameData, pos: TilePos) -> usize {
    let data = game_data.monsters.get("goblin").expect("goblin");
    let goblin = CreatureState::new(
        "goblin".to_string(),
        1,
        data.stats.health,
        data.stats.mana,
        1,
    );
    game_state.entities.spawn_creature(pos, goblin)
}

fn spawn_knight(game_state: &mut GameState, game_data: &GameData, pos: TilePos) -> usize {
    let data = game_data.heroes.get("knight").expect("knight");
    let hero = HeroState::new(
        "knight".to_string(),
        1,
        data.stats.health,
        data.stats.mana,
        pos,
        1.0,
        0,
    );
    game_state.entities.spawn_hero(pos, hero)
}

#[test]
fn room_defense_reads_the_authored_modifier() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let pos = TilePos::new(4, 4);
    push_room(&mut game_state, 900, "gatehouse", pos);

    let expected = game_data.rooms["gatehouse"]
        .effects
        .creature_defense_modifier;
    assert!(expected > 0.0, "the gatehouse should author a modifier");
    assert_eq!(
        crate::engine::room_validator::room_defense_at(pos, &game_state.room_manager, &game_data),
        expected
    );
    assert_eq!(
        crate::engine::room_validator::room_defense_at(
            TilePos::new(20, 20),
            &game_state.room_manager,
            &game_data
        ),
        0.0
    );
}

#[test]
fn the_barracks_modifier_is_no_longer_inert() {
    // The barracks has authored `creature_defense_modifier: 2` since it
    // shipped, against a field nothing read.
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let pos = TilePos::new(6, 6);
    push_room(&mut game_state, 902, "barracks", pos);

    assert_eq!(
        crate::engine::room_validator::room_defense_at(pos, &game_state.room_manager, &game_data),
        game_data.rooms["barracks"]
            .effects
            .creature_defense_modifier
    );
}

#[test]
fn a_fortified_room_absorbs_damage_for_a_creature() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let attacker_id = spawn_knight(&mut game_state, &game_data, TilePos::new(1, 1));
    let defender_id = spawn_goblin(&mut game_state, &game_data, TilePos::new(1, 2));

    let attacker = game_state.entities.get(attacker_id).unwrap();
    let defender = game_state.entities.get(defender_id).unwrap();

    // Damage rolls within a range, so pin the attack and compare the
    // deterministic defence term rather than two sampled swings.
    let fixed_attack = combat::CombatStats {
        damage_range: [1000.0, 1000.0],
        ..combat::extract_combat_stats(attacker, &game_data)
    };
    let bare = combat::extract_combat_stats(defender, &game_data);
    let room_defense = game_data.rooms["gatehouse"]
        .effects
        .creature_defense_modifier;
    let held = combat::CombatStats {
        defense: combat::fortified_defense(defender, bare.defense, room_defense),
        ..bare.clone()
    };

    let open_damage = combat::calculate_damage(&fixed_attack, &bare, &game_data);
    let held_damage = combat::calculate_damage(&fixed_attack, &held, &game_data);
    let expected_drop = room_defense * game_data.config.combat.defense_reduction;

    assert!(
        (open_damage - held_damage - expected_drop).abs() < 1e-3,
        "expected the room to absorb {expected_drop}, saw {}",
        open_damage - held_damage
    );
}

#[test]
fn a_hero_gains_nothing_from_the_keepers_stonework() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new_for_scenario(&game_data, "dark_beginnings");
    let hero_id = spawn_knight(&mut game_state, &game_data, TilePos::new(4, 4));
    let hero = game_state.entities.get(hero_id).unwrap();

    let bare = combat::extract_combat_stats(hero, &game_data).defense;
    assert_eq!(combat::fortified_defense(hero, bare, 999.0), bare);
}
