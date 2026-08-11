use super::*;

#[test]
fn default_is_standard_medium() {
    let cfg = SkirmishConfig::default();
    assert_eq!(cfg.map_type_label(), "Standard");
    assert_eq!(cfg.size_label(), "Medium");
    assert!(matches!(cfg.map_type(), MapType::Standard));
    assert_eq!(cfg.dimensions(), (32, 32));
}

#[test]
fn every_skirmish_config_generates_a_playable_map() {
    // Drive the real path the setup screen uses: each map type + size must
    // produce a bootable game with a live player heart. This is what makes
    // the procedural generator reachable as a sandbox.
    let game_data = crate::data::GameData::load().expect("game data should load");
    let mut cfg = SkirmishConfig::default();
    for _type_step in 0..SkirmishConfig::MAP_TYPE_LABELS.len() {
        for _size_step in 0..SkirmishConfig::SIZE_LABELS.len() {
            let (w, h) = cfg.dimensions();
            let state = crate::state::game_state::GameState::new_with_map_type(
                w,
                h,
                &game_data,
                cfg.map_type(),
            );
            assert_eq!((state.dungeon.width, state.dungeon.height), (w, h));
            assert!(
                state.find_dungeon_heart_position().is_some(),
                "{} {} skirmish should have a player heart",
                cfg.map_type_label(),
                cfg.size_label()
            );
            cfg.cycle_size();
        }
        cfg.cycle_map_type();
    }
}

#[test]
fn cycling_wraps_and_maps_to_generator_inputs() {
    let mut cfg = SkirmishConfig::default();
    cfg.cycle_map_type(); // Rich
    assert!(matches!(cfg.map_type(), MapType::Rich));
    cfg.cycle_map_type(); // Hazardous
    assert!(matches!(cfg.map_type(), MapType::Hazardous));
    cfg.cycle_map_type(); // wraps to Standard
    assert!(matches!(cfg.map_type(), MapType::Standard));

    cfg.cycle_size(); // Large
    assert_eq!(cfg.dimensions(), (48, 48));
    cfg.cycle_size(); // wraps to Small
    assert_eq!(cfg.dimensions(), (24, 24));
    assert_eq!(cfg.size_label(), "Small");
}
