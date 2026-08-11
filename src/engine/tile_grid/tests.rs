use super::*;

#[test]
fn test_world_to_iso_roundtrip() {
    let tile_width = 64.0;
    let tile_height = 32.0;

    let (iso_x, iso_y) = world_to_iso(5.0, 3.0, tile_width, tile_height);
    let (world_x, world_y) = iso_to_world(iso_x, iso_y, tile_width, tile_height);

    assert!((world_x - 5.0).abs() < 0.001);
    assert!((world_y - 3.0).abs() < 0.001);
}

#[test]
fn test_neighbor_detection() {
    use crate::data::GameData;

    let game_data = GameData::default();

    let grid = create_grid(5, 5, &game_data);
    let center = TilePos::new(2, 2);

    let neighbors = get_neighbors(&grid, center);
    assert_eq!(neighbors.len(), 8);

    let cardinal = get_cardinal_neighbors(&grid, center);
    assert_eq!(cardinal.len(), 4);
}

#[test]
fn test_tile_pos_distance() {
    let pos1 = TilePos::new(0, 0);
    let pos2 = TilePos::new(3, 4);

    assert_eq!(pos1.manhattan_distance(&pos2), 7);
    assert!((pos1.distance_to(&pos2) - 5.0).abs() < 0.001);
}
