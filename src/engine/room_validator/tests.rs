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
