use super::*;

#[test]
fn spawns_authored_hero_party_members() {
    let game_data = GameData::load().expect("game data should load");
    let scenario = game_data.scenarios.get("dark_beginnings").unwrap();
    let party = scenario
        .hero_parties
        .iter()
        .find(|party| party.id == "first_raid")
        .unwrap();
    let mut state = GameState::new_for_scenario(&game_data, "dark_beginnings");

    let ids = spawn_hero_party(&mut state, party, TilePos::new(10, 10), &game_data);

    assert_eq!(ids.len(), 4);
    assert!(ids.iter().all(|id| {
        state
            .entities
            .get(*id)
            .map(|entity| entity.owner == OwnerId::Heroes)
            .unwrap_or(false)
    }));
}
