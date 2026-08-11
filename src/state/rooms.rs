use crate::data::GameData;
use crate::engine::tile_grid;
use crate::state::entities::{EntityType, StructureState};
use crate::state::game_state::GameState;
use crate::state::hero_base::{HeroBuilding, SpawnTimer};
use crate::state::tile_state::TilePos;

impl GameState {
    pub fn detect_and_update_rooms(&mut self, game_data: &GameData) {
        self.room_manager
            .detect_and_update_rooms(&mut self.dungeon, game_data);

        self.detect_hero_base(game_data);

        let mut max_gold = game_data.config.player_initial_capacity.max_gold;
        let mut max_mana = game_data.config.player_initial_capacity.max_mana;
        let mut lair_tiles_count = 0;

        for room in &self.room_manager.rooms {
            if room.room_type == "lair" {
                lair_tiles_count += room.tiles.len();
            }

            if let Some(room_data) = game_data.rooms.get(&room.room_type) {
                if room_data.effects.gold_storage_capacity > 0 {
                    max_gold += room.tiles.len() as i32 * room_data.effects.gold_storage_capacity;
                }
                if room_data.effects.mana_storage_capacity > 0 {
                    max_mana += room.tiles.len() as i32 * room_data.effects.mana_storage_capacity;
                }
            }
        }

        self.player.max_gold = max_gold;
        self.player.max_mana = max_mana;
        self.player.max_creatures = lair_tiles_count;
    }

    fn detect_hero_base(&mut self, game_data: &GameData) {
        self.hero_base.buildings.clear();
        self.hero_base.enabled = false;

        let (w, h) = tile_grid::get_grid_dimensions(&self.dungeon.grid);
        let mut base_detected = false;
        let mut base_center_acc = (0, 0);
        let mut building_count = 0;

        for y in 0..h {
            for x in 0..w {
                let pos = TilePos::new(x as i32, y as i32);
                let Some(tile) = tile_grid::get_tile(&self.dungeon.grid, pos) else {
                    continue;
                };
                let Some(building_data) = game_data.hero_buildings.get(&tile.tile_type) else {
                    continue;
                };

                let mut building = HeroBuilding {
                    id: format!("{}_{}_{}", tile.tile_type, pos.x, pos.y),
                    building_type: tile.tile_type.clone(),
                    pos,
                    spawn_timers: building_data
                        .spawn_triggers
                        .iter()
                        .map(|trigger| SpawnTimer {
                            hero_id: trigger.hero_id.clone(),
                            time_until_spawn: 1.0,
                        })
                        .collect(),
                    entity_id: None,
                };

                let existing_id = self
                    .entities
                    .all()
                    .find(|entity| {
                        entity.pos == pos && matches!(&entity.entity_type, EntityType::Structure(_))
                    })
                    .map(|entity| entity.id);

                let entity_id = if let Some(id) = existing_id {
                    id
                } else {
                    let structure_state =
                        StructureState::new(tile.tile_type.clone(), building_data.hp as f32);
                    self.entities.spawn_structure(pos, structure_state)
                };

                building.entity_id = Some(entity_id);
                self.hero_base.buildings.push(building);

                base_detected = true;
                base_center_acc.0 += x as i32;
                base_center_acc.1 += y as i32;
                building_count += 1;
            }
        }

        if base_detected {
            self.hero_base.enabled = true;
            if building_count > 0 {
                self.hero_base.position = TilePos::new(
                    base_center_acc.0 / building_count,
                    base_center_acc.1 / building_count,
                );
            }
            eprintln!(
                "Hero Base detected with {} buildings at {:?}",
                building_count, self.hero_base.position
            );
        }
    }
}

#[cfg(test)]
mod tests;
