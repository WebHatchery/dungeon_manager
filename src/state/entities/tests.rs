use super::*;

#[test]
fn test_entity_manager_spawn() {
    let mut manager = EntityManager::new();
    let creature = CreatureState::new("imp".to_string(), 1, 100.0, 20.0, 0);
    let id = manager.spawn_creature(TilePos::new(5, 5), creature);

    assert_eq!(manager.count(), 1);
    assert_eq!(manager.count_creatures(), 1);

    let entity = manager.get(id).unwrap();
    assert_eq!(entity.pos, TilePos::new(5, 5));
}

#[test]
fn test_creature_needs() {
    let mut creature = CreatureState::new("goblin".to_string(), 1, 120.0, 0.0, 0);
    creature.set_need("sleep".to_string(), 80.0);
    creature.set_need("food".to_string(), 30.0);

    assert_eq!(creature.get_need("sleep"), 80.0);
    assert_eq!(creature.get_need("food"), 30.0);

    let (need, value) = creature.get_most_urgent_need().unwrap();
    assert_eq!(need, "food");
    assert_eq!(value, 30.0);
}

#[test]
fn test_remove_dead() {
    let mut manager = EntityManager::new();

    let mut creature1 = CreatureState::new("imp".to_string(), 1, 100.0, 20.0, 0);
    creature1.health = 0.0; // Dead

    let creature2 = CreatureState::new("goblin".to_string(), 1, 120.0, 0.0, 0);
    // Alive

    manager.spawn_creature(TilePos::new(0, 0), creature1);
    manager.spawn_creature(TilePos::new(1, 1), creature2);

    assert_eq!(manager.count(), 2);

    let removed = manager.remove_dead();
    assert_eq!(removed, 1);
    assert_eq!(manager.count(), 1);
}
