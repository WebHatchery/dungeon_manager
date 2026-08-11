use super::*;

#[test]
fn test_spend_resources() {
    // manually construct player state to avoid needing GameData for simple unit tests
    let mut player = PlayerState {
        gold: 200,
        mana: 1000,
        food: 100,
        max_gold: 10000,
        max_mana: 5000,
        max_food: 1000,
        materials: 100,
        max_materials: 500,
        warned_keys: std::collections::HashSet::new(),
        pending_messages: Vec::new(),
        accumulators: ResourceAccumulator::default(),
        unlocked_rooms: HashSet::new(),
        unlocked_creatures: HashSet::new(),
        unlocked_spells: HashSet::new(),
        unlocked_traps: HashSet::new(),
        trap_inventory: HashMap::new(),
        trap_manufacturing_progress: HashMap::new(),
        completed_technologies: HashSet::new(),
        research_points: 0,
        active_research: None,
        research_progress: 0.0,
        dungeon_heart_health: 100.0,
        max_creatures: 20,
        current_creature_count: 0,
        claimed_tile_count: 0,
        spell_cooldowns: HashMap::new(),
        kills: HashMap::new(),
        deaths: HashMap::new(),
        gold_mined: 0,
        spells_cast: HashMap::new(),
        game_time: 0.0,
        graveyard_corpses: 0,
        scavenger_conversion_progress: HashMap::new(),
    };

    // Assert initial state matches what test expects
    assert!(player.can_afford(100, 50));
    assert!(player.spend(100, 50));
    assert_eq!(player.gold, 100);
    assert_eq!(player.mana, 950);
}

#[test]
fn test_resource_accumulation() {
    let mut player = PlayerState {
        gold: 0,
        mana: 0,
        food: 0,
        max_gold: 100,
        max_mana: 100,
        max_food: 100,
        materials: 0,
        max_materials: 100,
        warned_keys: std::collections::HashSet::new(),
        pending_messages: Vec::new(),
        accumulators: ResourceAccumulator::default(),
        unlocked_rooms: HashSet::new(),
        unlocked_creatures: HashSet::new(),
        unlocked_spells: HashSet::new(),
        unlocked_traps: HashSet::new(),
        trap_inventory: HashMap::new(),
        trap_manufacturing_progress: HashMap::new(),
        completed_technologies: HashSet::new(),
        research_points: 0,
        active_research: None,
        research_progress: 0.0,
        dungeon_heart_health: 100.0,
        max_creatures: 20,
        current_creature_count: 0,
        claimed_tile_count: 0,
        spell_cooldowns: HashMap::new(),
        kills: HashMap::new(),
        deaths: HashMap::new(),
        gold_mined: 0,
        spells_cast: HashMap::new(),
        game_time: 0.0,
        graveyard_corpses: 0,
        scavenger_conversion_progress: HashMap::new(),
    };

    // Add small fractional amount 10 times
    for _ in 0..10 {
        player.add_resources_precise(0.1, 0.0, 0.0, 0.0);
    }

    // Should have 1 gold
    assert_eq!(player.gold, 1);
    // Accumulator should be effectively 0 (or close to it due to float precision)
    assert!(player.accumulators.gold < 0.001);

    // Add 0.3 three times
    for _ in 0..3 {
        player.add_resources_precise(0.0, 0.35, 0.0, 0.0);
    }
    // 0.35 * 3 = 1.05
    assert_eq!(player.mana, 1);
    assert!(player.accumulators.mana > 0.04); // Remaining 0.05
}

#[test]
fn test_cannot_overspend() {
    let mut player = PlayerState {
        gold: 200,
        mana: 1000,
        food: 100,
        max_gold: 10000,
        max_mana: 5000,
        max_food: 1000,
        materials: 100,
        max_materials: 500,
        warned_keys: std::collections::HashSet::new(),
        pending_messages: Vec::new(),
        accumulators: ResourceAccumulator::default(),
        unlocked_rooms: HashSet::new(),
        unlocked_creatures: HashSet::new(),
        unlocked_spells: HashSet::new(),
        unlocked_traps: HashSet::new(),
        trap_inventory: HashMap::new(),
        trap_manufacturing_progress: HashMap::new(),
        completed_technologies: HashSet::new(),
        research_points: 0,
        active_research: None,
        research_progress: 0.0,
        dungeon_heart_health: 100.0,
        max_creatures: 20,
        current_creature_count: 0,
        claimed_tile_count: 0,
        spell_cooldowns: HashMap::new(),
        kills: HashMap::new(),
        deaths: HashMap::new(),
        gold_mined: 0,
        spells_cast: HashMap::new(),
        game_time: 0.0,
        graveyard_corpses: 0,
        scavenger_conversion_progress: HashMap::new(),
    };

    assert!(!player.can_afford(10000, 0));
    assert!(!player.spend(10000, 0));
    assert_eq!(player.gold, 200); // Unchanged
}

#[test]
fn test_resource_caps() {
    let mut player = PlayerState {
        gold: 200,
        mana: 1000,
        food: 100,
        max_gold: 10000,
        max_mana: 5000,
        max_food: 1000,
        materials: 100,
        max_materials: 500,
        warned_keys: std::collections::HashSet::new(),
        pending_messages: Vec::new(),
        accumulators: ResourceAccumulator::default(),
        unlocked_rooms: HashSet::new(),
        unlocked_creatures: HashSet::new(),
        unlocked_spells: HashSet::new(),
        unlocked_traps: HashSet::new(),
        trap_inventory: HashMap::new(),
        trap_manufacturing_progress: HashMap::new(),
        completed_technologies: HashSet::new(),
        research_points: 0,
        active_research: None,
        research_progress: 0.0,
        dungeon_heart_health: 100.0,
        max_creatures: 20,
        current_creature_count: 0,
        claimed_tile_count: 0,
        spell_cooldowns: HashMap::new(),
        kills: HashMap::new(),
        deaths: HashMap::new(),
        gold_mined: 0,
        spells_cast: HashMap::new(),
        game_time: 0.0,
        graveyard_corpses: 0,
        scavenger_conversion_progress: HashMap::new(),
    };

    player.add_resources(0, 20000, 1000, 1000);
    assert_eq!(player.mana, player.max_mana);
    assert_eq!(player.food, player.max_food);
    assert_eq!(player.materials, player.max_materials);
}
