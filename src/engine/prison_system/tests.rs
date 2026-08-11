use super::*;
use crate::engine::room_validator::Room;
use crate::state::entities::{CreatureState, HeroState};

fn active_room(id: usize, room_type: &str, tiles: &[TilePos]) -> Room {
    let mut room = Room::new(
        id,
        room_type.to_string(),
        tiles.iter().copied().collect(),
        Vec::new(),
    );
    room.active = true;
    room
}

#[test]
fn prison_capture_respects_cell_capacity() {
    let game_data = GameData::load().expect("game data should load");
    let mut entities = EntityManager::new();
    let mut room_manager = RoomManager::new();
    let mut notifications = NotificationManager::new();
    room_manager.rooms.push(active_room(
        1,
        "prison",
        &[
            TilePos::new(1, 1),
            TilePos::new(2, 1),
            TilePos::new(3, 1),
            TilePos::new(4, 1),
            TilePos::new(5, 1),
        ],
    ));

    for index in 0..2 {
        let mut hero = HeroState::new(
            "knight".to_string(),
            1,
            100.0,
            10.0,
            TilePos::new(index, 0),
            1.0,
            index as u64,
        );
        hero.health = 0.0;
        entities.spawn_hero(TilePos::new(index, 0), hero);
    }

    handle_prison_captures(&mut entities, &room_manager, &mut notifications, &game_data);

    let captured = entities
        .heroes()
        .filter(|(_, hero)| hero.is_captured)
        .count();
    assert_eq!(captured, 1);
}

#[test]
fn active_torture_room_pulls_prisoner_and_converts() {
    let game_data = GameData::load().expect("game data should load");
    let mut entities = EntityManager::new();
    let mut room_manager = RoomManager::new();
    let mut notifications = NotificationManager::new();
    room_manager
        .rooms
        .push(active_room(1, "prison", &[TilePos::new(1, 1)]));
    room_manager.rooms.push(active_room(
        2,
        "torture_chamber",
        &[
            TilePos::new(5, 5),
            TilePos::new(6, 5),
            TilePos::new(7, 5),
            TilePos::new(5, 6),
            TilePos::new(6, 6),
        ],
    ));

    let mut hero = HeroState::new(
        "knight".to_string(),
        1,
        100.0,
        10.0,
        TilePos::new(1, 1),
        1.0,
        1,
    );
    hero.is_captured = true;
    hero.health = 10.0;
    let hero_id = entities.spawn_hero(TilePos::new(1, 1), hero);

    let monster_data = game_data.monsters.get("succubus").unwrap();
    let mut succubus = CreatureState::new(
        "succubus".to_string(),
        1,
        monster_data.stats.health,
        monster_data.stats.mana,
        2,
    );
    succubus.current_task = Some(Task::Work(2, TilePos::new(5, 5)));
    entities.spawn_creature(TilePos::new(5, 5), succubus);

    progress_prison_conversions(
        &mut entities,
        &room_manager,
        &mut notifications,
        &game_data,
        100.0,
    );

    let entity = entities.get(hero_id).expect("converted hero remains");
    assert_eq!(entity.owner, OwnerId::Player);
    let hero = entity.as_hero().unwrap();
    assert!(hero.is_converted);
    assert!(!hero.is_captured);
}
