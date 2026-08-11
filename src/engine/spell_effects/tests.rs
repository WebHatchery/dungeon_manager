use super::*;
use crate::data::GameData;
use crate::state::entities::HeroState;

#[test]
fn heal_effect_heals_heroes() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);

    let mut hero = HeroState::new(
        "peasant".to_string(),
        1,
        100.0,
        10.0,
        TilePos::new(1, 1),
        1.0,
        1,
    );
    hero.health = 40.0;
    let hero_id = game_state.entities.spawn_hero(TilePos::new(1, 1), hero);

    let effect = SpellEffect {
        effect_type: "heal".to_string(),
        amount: 25.0,
        damage_type: None,
        stat: None,
        multiplier: None,
        duration: None,
        status: None,
        delay: None,
        entity: None,
        from_tile: None,
        to_tile: None,
        radius: None,
    };
    apply_heal_effect(hero_id, &effect, &mut game_state);

    let healed = game_state.entities.get(hero_id).unwrap().as_hero().unwrap();
    assert_eq!(healed.health, 65.0);
}

#[test]
fn spawn_entity_effect_supports_non_imp_creatures() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);

    let effect = SpellEffect {
        effect_type: "spawn_entity".to_string(),
        amount: 0.0,
        damage_type: None,
        stat: None,
        multiplier: None,
        duration: None,
        status: None,
        delay: None,
        entity: Some("goblin".to_string()),
        from_tile: None,
        to_tile: None,
        radius: None,
    };
    spawn_entity_effect(TilePos::new(2, 2), &effect, &mut game_state, &game_data);

    let goblin_exists = game_state
        .entities
        .all()
        .any(|e| matches!(&e.entity_type, crate::state::entities::EntityType::Creature(c) if c.creature_id == "goblin"));
    assert!(
        goblin_exists,
        "spawn_entity should support entity ids other than \"imp\""
    );
}

#[test]
fn stat_modifier_speed_buff_reverts_after_duration() {
    let game_data = GameData::load().expect("game data should load");
    let mut game_state = GameState::new(20, 20, &game_data);

    let creature =
        crate::state::entities::CreatureState::new("goblin".to_string(), 1, 20.0, 10.0, 1);
    let base_speed = creature.movement_speed;
    let creature_id = game_state
        .entities
        .spawn_creature(TilePos::new(1, 1), creature);

    let effect = SpellEffect {
        effect_type: "stat_modifier".to_string(),
        amount: 0.0,
        damage_type: None,
        stat: Some("speed".to_string()),
        multiplier: Some(2.0),
        duration: Some(5.0),
        status: None,
        delay: None,
        entity: None,
        from_tile: None,
        to_tile: None,
        radius: None,
    };
    apply_stat_modifier(creature_id, &effect, &mut game_state);

    let buffed_speed = game_state
        .entities
        .get(creature_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .movement_speed;
    assert_eq!(buffed_speed, base_speed * 2.0);

    // Tick past the buff's duration
    let entity = game_state.entities.get_mut(creature_id).unwrap();
    crate::engine::combat::update_status_effects(entity, 6.0);

    let reverted_speed = game_state
        .entities
        .get(creature_id)
        .unwrap()
        .as_creature()
        .unwrap()
        .movement_speed;
    assert_eq!(reverted_speed, base_speed);
}
