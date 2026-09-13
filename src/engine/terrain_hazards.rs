//! Environmental damage from authored hazardous terrain.

use crate::data::GameData;
use crate::state::entities::{EntityId, EntityType};
use crate::state::game_state::GameState;

/// Apply each tile's `damage_per_second` to entities standing on it.
///
/// Movement normally avoids blocking hazards such as lava, but scripted maps,
/// bridges being removed, and forced movement can still leave an entity on a
/// dangerous tile. Keeping the rule here makes the authored field meaningful
/// for every faction and gives hero ability timing a reliable damage event.
pub fn apply_terrain_damage(state: &mut GameState, game_data: &GameData, dt: f32) -> usize {
    if dt <= 0.0 {
        return 0;
    }

    let affected: Vec<(EntityId, f32)> = state
        .entities
        .all()
        .filter_map(|entity| {
            let damage_per_second = state
                .get_tile(entity.pos)
                .and_then(|tile| game_data.tiles.get(&tile.tile_type))
                .and_then(|tile| tile.damage_per_second)
                .filter(|damage| damage.is_finite() && *damage > 0.0)?;
            Some((entity.id, damage_per_second * dt))
        })
        .collect();

    for (entity_id, damage) in &affected {
        let Some(entity) = state.entities.get_mut(*entity_id) else {
            continue;
        };
        match &mut entity.entity_type {
            EntityType::Creature(creature) => {
                creature.health = (creature.health - damage).max(0.0)
            }
            EntityType::Hero(hero) => {
                hero.health = (hero.health - damage).max(0.0);
                entity.last_damage_time = state.time_elapsed;
            }
            EntityType::Structure(structure) => {
                structure.health = (structure.health - damage).max(0.0)
            }
            EntityType::ResourcePile(_) => continue,
        }
    }

    affected.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::entities::HeroState;
    use crate::state::tile_state::TilePos;

    #[test]
    fn authored_lava_damage_reaches_entities_on_the_tile() {
        let game_data = GameData::load().expect("game data should load");
        let mut state = GameState::new(20, 20, &game_data);
        state.entities = crate::state::entities::EntityManager::new();
        let pos = TilePos::new(4, 4);
        state.dungeon.get_tile_mut(pos).unwrap().tile_type = "lava".to_string();
        let hero_id = state.entities.spawn_hero(
            pos,
            HeroState::new("peasant".to_string(), 1, 100.0, 0.0, pos, 1.0, 1),
        );

        assert_eq!(apply_terrain_damage(&mut state, &game_data, 2.0), 1);
        assert_eq!(
            state.entities.get(hero_id).unwrap().as_hero().unwrap().health,
            50.0
        );
    }
}
