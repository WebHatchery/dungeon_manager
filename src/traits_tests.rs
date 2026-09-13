use crate::data::monsters::MonsterData;
use crate::data::traits::TraitData;
use crate::data::GameData;
use crate::engine::combat;
use crate::engine::creature_ai::apply_slap;
use crate::engine::creature_ai::needs::update_needs;
use crate::state::entities::{CreatureState, EntityManager};
use crate::state::tile_state::TilePos;

#[test]
fn strong_trait_multiplies_creature_attack_in_combat_stats() {
    let game_data = GameData::load().expect("game data should load");
    let troll_data = game_data.monsters.get("troll").expect("troll should exist");
    assert!(troll_data.traits.contains(&"strong".to_string()));

    let mut entities = EntityManager::new();
    let creature = CreatureState::new("troll".to_string(), 1, 300.0, 0.0, 1);
    let creature_id = entities.spawn_creature(TilePos::new(0, 0), creature);
    let entity = entities.get(creature_id).unwrap();

    let stats = combat::extract_combat_stats(entity, &game_data);

    // Level 1 has no level bonus. Troll's authored smash multiplier is also
    // active, so this checks the trait and combat-ability layers together.
    assert_eq!(stats.attack, troll_data.stats.attack * 1.15 * 1.25);
}

#[test]
fn undead_trait_zeroes_food_and_sleep_need_decay() {
    let game_data = GameData::load().expect("game data should load");
    let vampire_data = game_data
        .monsters
        .get("vampire")
        .expect("vampire should exist");
    assert!(vampire_data.traits.contains(&"undead".to_string()));
    // Sanity check: the raw authored decay rates are non-zero, so a real multiplier is needed
    // to zero them out (this isn't just "the data already says 0").
    assert!(vampire_data.needs["food"].decay_per_minute > 0.0);
    assert!(vampire_data.needs["sleep"].decay_per_minute > 0.0);

    let mut creature = CreatureState::new("vampire".to_string(), 1, 150.0, 100.0, 1);
    creature.set_need("food".to_string(), 80.0);
    creature.set_need("sleep".to_string(), 80.0);
    creature.set_need("gold".to_string(), 80.0);

    update_needs(&mut creature, 60.0, vampire_data, &game_data);

    assert_eq!(
        creature.get_need("food"),
        80.0,
        "undead trait should zero out food decay"
    );
    assert_eq!(
        creature.get_need("sleep"),
        80.0,
        "undead trait should zero out sleep decay"
    );
    // Gold isn't covered by the undead trait's need_decay_multipliers, so it should still decay.
    assert!(
        creature.get_need("gold") < 80.0,
        "gold need should still decay normally"
    );
}

#[test]
fn discipline_response_multiplier_dampens_slap_mood_swing() {
    let mut game_data = GameData::load().expect("game data should load");
    game_data.traits.insert(
        "test_mindless".to_string(),
        TraitData {
            id: "test_mindless".to_string(),
            discipline_response_multiplier: 0.2,
            ..Default::default()
        },
    );

    let mut monster_data: MonsterData = serde_json::from_str(
        r#"{
            "id": "test_creature",
            "name": "Test",
            "description": "Test creature",
            "faction": "dungeon",
            "role": "worker",
            "stats": { "health": 100, "mana": 0, "attack": 5, "defense": 2, "speed": 1.0, "carry_capacity": 10, "sight_radius": 5 },
            "needs": {},
            "traits": ["test_mindless"],
            "ai": { "base_mood": 70, "anger_threshold": 30, "desertion_threshold": 20, "task_preferences": {}, "room_desires": {}, "discipline_response": { "slap": -20 } },
            "combat": { "attack_type": "melee", "damage_range": [3, 6], "attack_speed": 1.0, "armor_type": "none", "resistances": {}, "abilities": [] },
            "progression": { "xp_to_level": [0, 100], "stat_growth_per_level": {}, "max_level": 2, "mutations": [] },
            "economy": { "wage_per_minute": 1, "steals_if_unpaid": false, "drops_gold_on_death": [5, 10] },
            "spawn": { "source": "portal", "min_dungeon_reputation": 0, "preferred_rooms": [], "spawn_weight": 1.0, "max_population": 10 },
            "visual": { "sprite": "test", "scale": 1.0, "animations": [], "voice_set": "test" }
        }"#,
    )
    .unwrap();
    monster_data.id = "test_creature".to_string();

    let mut creature = CreatureState::new("test_creature".to_string(), 1, 100.0, 0.0, 1);
    creature.mood = 70.0;

    apply_slap(&mut creature, &monster_data, &game_data, 100.0);

    // Raw slap would be -20; a 0.2 discipline_response_multiplier should shrink it to -4.
    assert_eq!(creature.mood, 66.0);
}

/// `trap_savvy` is the first trait that acts on the *world* rather than on the
/// creature carrying it — a kobold standing near a trap makes that trap hurt
/// more. It therefore needs a different kind of test from the ones above: the
/// subject is the trap, not the kobold.
#[cfg(test)]
mod trap_tending {
    use crate::data::GameData;
    use crate::engine::trap_system::process_trap_triggers;
    use crate::state::dungeon::Dungeon;
    use crate::state::entities::{CreatureState, EntityManager, HeroState};
    use crate::state::game_state::MapType;
    use crate::state::tile_state::{TilePos, TrapState};

    const TRAP_POS: TilePos = TilePos { x: 5, y: 5 };

    /// The damage a spike trap deals to a hero standing on it, with `tender`
    /// (if any) standing one tile away.
    ///
    /// Retries with fresh state until the trap actually fires. Every hero in
    /// the roster has non-zero `trap_awareness` — the lowest is the peasant's
    /// 0.2 — so there is no hero who is guaranteed to walk into it, and a
    /// single attempt would be flaky one time in five. The damage itself is
    /// deterministic once it fires, so the comparison stays exact.
    fn damage_with_tender(tender: Option<&str>) -> f32 {
        let game_data = GameData::load().expect("game data should load");
        assert!(
            game_data.heroes["peasant"].behavior.trap_awareness > 0.0,
            "if some hero ever reaches 0.0 awareness, use them and drop the retry loop"
        );

        for _ in 0..64 {
            let mut dungeon = Dungeon::new(16, 16, &game_data, MapType::Test);
            let tile = dungeon.get_tile_mut(TRAP_POS).expect("trap tile");
            tile.trap = Some(TrapState {
                trap_type: "spike_trap".to_string(),
                funded: true,
                constructed: true,
                active: true,
                triggered: false,
                cooldown: 0.0,
                construction_progress: 10.0,
            });

            let mut entities = EntityManager::new();
            let hero = HeroState::new("peasant".to_string(), 1, 200.0, 0.0, TRAP_POS, 1.0, 0);
            entities.spawn_hero(TRAP_POS, hero);

            if let Some(creature_id) = tender {
                let creature = CreatureState::new(creature_id.to_string(), 1, 90.0, 0.0, 1);
                entities.spawn_creature(TilePos::new(6, 6), creature);
            }

            let dealt: f32 = process_trap_triggers(&mut dungeon, &mut entities, &game_data, 0.1)
                .iter()
                .map(|r| r.damage_dealt)
                .sum();
            if dealt > 0.0 {
                return dealt;
            }
        }
        panic!("the trap never fired in 64 attempts, which should be impossible");
    }

    #[test]
    fn a_kobold_standing_by_a_trap_makes_it_hit_harder() {
        let alone = damage_with_tender(None);
        let tended = damage_with_tender(Some("kobold"));

        assert!(alone > 0.0, "the trap should fire at all");
        assert!(
            tended > alone,
            "a kobold nearby should raise trap damage: {tended} vs {alone}"
        );
    }

    /// The control: an ordinary creature in exactly the same spot changes
    /// nothing, so the bonus is the trait rather than mere proximity.
    #[test]
    fn an_ordinary_creature_nearby_changes_nothing() {
        let alone = damage_with_tender(None);
        let watched = damage_with_tender(Some("goblin"));
        assert_eq!(
            watched, alone,
            "only `trap_savvy` should matter, not any creature"
        );
    }
}
