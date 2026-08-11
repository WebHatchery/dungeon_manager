use crate::data::GameData;

#[test]
fn orphan_sprite_monsters_are_wired() {
    let game_data = GameData::load().expect("game data should load");
    // The four previously-orphaned sprites now have data entries.
    for id in ["zombie", "ghost", "bat_swarm", "dark_elf"] {
        let monster = game_data
            .monsters
            .get(id)
            .unwrap_or_else(|| panic!("{id} should be in the roster"));
        assert!(!monster.name.is_empty());
        assert!(monster.stats.health > 0.0);
    }
}

#[test]
fn every_monster_trait_resolves_against_the_trait_system() {
    // Traits are data-driven; a typo'd trait id would silently do nothing.
    // Guard the whole roster (incl. the new monsters) against dangling ids.
    let game_data = GameData::load().expect("game data should load");
    for monster in game_data.monsters.values() {
        for trait_id in &monster.traits {
            assert!(
                game_data.traits.contains_key(trait_id),
                "monster '{}' references unknown trait '{}'",
                monster.id,
                trait_id
            );
        }
    }
}
