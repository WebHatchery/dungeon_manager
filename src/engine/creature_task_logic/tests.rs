use super::*;
use crate::data::monsters::{MonsterAIData, NeedData};
use std::collections::HashMap;

fn create_test_monster_data() -> MonsterData {
    let mut needs = HashMap::new();
    let sleep_need = NeedData {
        decay_per_minute: 1.0,
        satisfied_by: vec!["lair".to_string()],
        stash_amount: None,
    };
    needs.insert("sleep".to_string(), sleep_need);

    let _ai = MonsterAIData {
        base_mood: 70.0,
        anger_threshold: 30.0,
        desertion_threshold: 20.0,
        task_preferences: HashMap::new(),
        room_desires: HashMap::new(),
        discipline_response: HashMap::new(),
    };

    // Minimal test data skeleton
    serde_json::from_str(r#"{
        "id": "test_creature",
        "name": "Test",
        "description": "Test creature",
        "faction": "dungeon",
        "role": "worker",
        "stats": { "health": 100, "mana": 0, "attack": 5, "defense": 2, "speed": 1.0, "carry_capacity": 10, "sight_radius": 5 },
        "needs": { "sleep": { "decay_per_minute": 1.0, "satisfied_by": ["lair"] } },
        "traits": [],
        "ai": { "base_mood": 70, "anger_threshold": 30, "desertion_threshold": 20, "task_preferences": {}, "room_desires": {}, "discipline_response": {} },
        "combat": { "attack_type": "melee", "damage_range": [3, 6], "attack_speed": 1.0, "armor_type": "none", "resistances": {}, "abilities": [] },
        "progression": { "xp_to_level": [0, 100], "stat_growth_per_level": {}, "max_level": 2, "mutations": [] },
        "economy": { "wage_per_minute": 1, "steals_if_unpaid": false, "drops_gold_on_death": [5, 10] },
        "spawn": { "source": "portal", "min_dungeon_reputation": 0, "preferred_rooms": [], "spawn_weight": 1.0, "max_population": 10 },
        "visual": { "sprite": "test.png", "scale": 1.0, "animations": ["idle"], "voice_set": "test" }
    }"#).unwrap()
}

#[test]
fn test_should_desert() {
    let monster_data = create_test_monster_data();
    let mut creature = CreatureState::new("test_creature".to_string(), 1, 100.0, 0.0, 0);

    creature.mood = 50.0;
    assert!(!should_desert(&creature, &monster_data));

    creature.mood = 15.0;
    assert!(should_desert(&creature, &monster_data));
}

#[test]
fn test_task_desirability() {
    let monster_data = create_test_monster_data();
    let game_data = GameData::default();
    let mut creature = CreatureState::new("test_creature".to_string(), 1, 100.0, 0.0, 0);
    let task = Task::Sleep(0);

    // High satisfaction means a sleep task is not urgent.
    creature.set_need("sleep".to_string(), 90.0);
    let desirability = calculate_task_desirability(&task, &creature, &monster_data, &game_data);
    assert!(desirability < 1.5);

    // Low satisfaction makes sleep more desirable.
    creature.set_need("sleep".to_string(), 10.0);
    let desirability = calculate_task_desirability(&task, &creature, &monster_data, &game_data);
    assert!(desirability > 1.5);
}
