use super::*;

#[test]
fn test_is_contiguous() {
    // Test a contiguous 2x2 square
    let mut tiles = HashSet::new();
    tiles.insert(TilePos::new(0, 0));
    tiles.insert(TilePos::new(1, 0));
    tiles.insert(TilePos::new(0, 1));
    tiles.insert(TilePos::new(1, 1));
    assert!(is_contiguous(&tiles));

    // Test a non-contiguous set
    let mut tiles = HashSet::new();
    tiles.insert(TilePos::new(0, 0));
    tiles.insert(TilePos::new(5, 5));
    assert!(!is_contiguous(&tiles));
}

#[test]
fn test_shape_metrics() {
    // Test a 4x1 corridor (thin corridor)
    let mut tiles = HashSet::new();
    for x in 0..5 {
        tiles.insert(TilePos::new(x, 0));
    }
    let metrics = calculate_shape_metrics(&tiles);
    assert!(metrics.is_thin_corridor);
    assert!(metrics.aspect_ratio > 4.0);

    // Test a 3x3 square (good shape)
    let mut tiles = HashSet::new();
    for x in 0..3 {
        for y in 0..3 {
            tiles.insert(TilePos::new(x, y));
        }
    }
    let metrics = calculate_shape_metrics(&tiles);
    assert!(!metrics.is_thin_corridor);
    assert_eq!(metrics.compactness, 1.0); // Perfect square
}

#[test]
fn test_room_center() {
    let mut tiles = HashSet::new();
    tiles.insert(TilePos::new(0, 0));
    tiles.insert(TilePos::new(2, 0));
    tiles.insert(TilePos::new(1, 1));

    let room = Room::new(0, "test".to_string(), tiles, Vec::new());
    let center = room.get_center();

    // Average: x = (0+2+1)/3 = 1, y = (0+0+1)/3 = 0
    assert_eq!(center.x, 1);
    assert_eq!(center.y, 0);
}

#[test]
fn authored_room_productivity_scales_per_tile() {
    let game_data = GameData::load().expect("game data should load");
    let room = Room::new(
        1,
        "hatchery".to_string(),
        [TilePos::new(0, 0), TilePos::new(1, 0), TilePos::new(0, 1)]
            .into_iter()
            .collect(),
        Vec::new(),
    );
    let mut room_data = game_data.rooms["hatchery"].clone();
    room_data.scaling.per_tile_multiplier = 1.1;

    let multiplier = room_productivity_multiplier(&room, &room_data);
    assert!((multiplier - 1.331).abs() < 0.001);
}

#[test]
fn room_entry_rules_filter_forbidden_and_underlevel_creatures() {
    let game_data = GameData::load().expect("game data should load");
    let room = Room::new(
        1,
        "training_room".to_string(),
        [TilePos::new(0, 0)].into_iter().collect(),
        Vec::new(),
    );

    assert!(!creature_can_enter_room(
        &room, "imp", 10, 50.0, &game_data
    ));
    assert!(!creature_can_enter_room(
        &room, "goblin", 1, 50.0, &game_data
    ));
    assert!(creature_can_enter_room(
        &room, "goblin", 2, 50.0, &game_data
    ));
}
