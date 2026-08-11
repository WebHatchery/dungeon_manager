use crate::data::GameData;

#[test]
fn orphan_sprite_heroes_are_wired() {
    let game_data = GameData::load().expect("game data should load");
    // The three previously-orphaned hero sprites now have data entries.
    for id in ["peasant", "champion", "dragon_knight"] {
        let hero = game_data
            .heroes
            .get(id)
            .unwrap_or_else(|| panic!("{id} should be in the hero roster"));
        assert!(!hero.name.is_empty());
        assert!(hero.stats.health > 0.0);
    }
}
