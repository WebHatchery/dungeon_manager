use crate::data::GameData;
use crate::engine::combat::{self, resolve_combat_tick};
use crate::state::entities::{EntityId, EntityType};
use crate::state::game_state::GameState;

fn get_entity_attack_type(entity_type: &EntityType, game_data: &GameData) -> String {
    match entity_type {
        EntityType::Creature(state) => game_data
            .monsters
            .get(&state.creature_id)
            .map(|data| data.combat.attack_type.clone())
            .unwrap_or_else(|| "melee".to_string()),
        EntityType::Hero(state) => game_data
            .heroes
            .get(&state.hero_id)
            .map(|data| data.combat.attack_type.clone())
            .unwrap_or_else(|| "melee".to_string()),
        EntityType::Structure(_) | EntityType::ResourcePile(_) => "none".to_string(),
    }
}

impl GameState {
    pub(in crate::state) fn resolve_combat(&mut self, game_data: &GameData, dt: f32) {
        let all_entities: Vec<EntityId> = self.entities.all().map(|entity| entity.id).collect();
        // Razed armouries and forges blunt every hero on the map.
        let hero_supply_penalty = (
            self.hero_base.hero_attack_penalty,
            self.hero_base.hero_defense_penalty,
        );

        for &attacker_id in &all_entities {
            let attacker = match self.entities.get(attacker_id) {
                Some(attacker) => attacker,
                None => continue,
            };

            let attack_type = get_entity_attack_type(&attacker.entity_type, game_data);
            let attacker_visual_pos = attacker.visual_pos;
            let targets = combat::find_combat_targets(
                attacker,
                self.entities.entities(),
                &self.dungeon,
                game_data,
            );

            for target_id in targets {
                let defender = match self.entities.get(target_id) {
                    Some(defender) => defender,
                    None => continue,
                };

                if !combat::in_combat_range(
                    attacker.pos,
                    defender.pos,
                    &attack_type,
                    &self.dungeon.grid,
                    game_data,
                ) {
                    continue;
                }

                let defender_visual_pos = defender.visual_pos;
                let defender_room_defense = crate::engine::room_validator::room_defense_at(
                    defender.pos,
                    &self.room_manager,
                    game_data,
                );
                let result = resolve_combat_tick(
                    attacker,
                    defender,
                    dt,
                    game_data,
                    defender_room_defense,
                    hero_supply_penalty,
                );

                if let Some((projectile_type, damage)) = result.projectile_spawned.clone() {
                    self.projectiles.spawn_with_status(
                        attacker_visual_pos,
                        defender_visual_pos,
                        &projectile_type,
                        attacker_id,
                        target_id,
                        damage,
                        result.status_applied.clone(),
                    );
                } else if result.damage_dealt > 0.0 {
                    self.projectiles.spawn(
                        attacker_visual_pos,
                        defender_visual_pos,
                        &attack_type,
                        attacker_id,
                        target_id,
                        0.0,
                    );
                }

                combat::apply_combat_result(
                    &result,
                    attacker_id,
                    target_id,
                    self.entities.entities_mut(),
                    game_data,
                    self.time_elapsed,
                );
                break;
            }
        }
    }
}
