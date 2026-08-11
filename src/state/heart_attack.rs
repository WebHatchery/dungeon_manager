use crate::data::GameData;
use crate::state::entities::{EntityId, HeroGoal};
use crate::state::game_state::GameState;
use crate::state::OwnerId;
use macroquad_toolkit::rng;

impl GameState {
    pub(in crate::state) fn process_dungeon_heart_attacks(
        &mut self,
        game_data: &GameData,
        dt: f32,
    ) {
        let Some(target_pos) = self.find_dungeon_heart_position() else {
            return;
        };

        let mut total_damage = 0.0;
        let mut attackers: Vec<(EntityId, (f32, f32), String)> = Vec::new();

        for entity in self.entities.all() {
            let Some(hero) = entity.as_hero() else {
                continue;
            };
            if !matches!(hero.current_goal, HeroGoal::DestroyHeart) {
                continue;
            }

            let hero_data = game_data.heroes.get(&hero.hero_id);
            let attack_type = hero_data
                .map(|data| data.combat.attack_type.clone())
                .unwrap_or_else(|| "melee".to_string());
            let attack_speed = hero_data
                .map(|data| data.combat.attack_speed)
                .unwrap_or(1.0);

            let manhattan_dist =
                (entity.pos.x - target_pos.x).abs() + (entity.pos.y - target_pos.y).abs();
            let attack_range = match attack_type.as_str() {
                "melee" => game_data.config.combat_ranges.melee,
                "ranged" => game_data.config.combat_ranges.ranged,
                "magic" => game_data.config.combat_ranges.magic,
                _ => game_data.config.combat_ranges.melee,
            };

            if manhattan_dist > attack_range || rng::rand() >= attack_speed * dt {
                continue;
            }

            let base_damage = hero_data
                .map(|data| {
                    let min = data.combat.damage_range[0];
                    let max = data.combat.damage_range[1];
                    rng::gen_range(min, max)
                })
                .unwrap_or(5.0);

            total_damage += base_damage;
            attackers.push((entity.id, entity.visual_pos, attack_type));
        }

        for entity in self.entities.all() {
            let Some(creature) = entity.as_creature() else {
                continue;
            };
            if !entity.owner.is_hostile_to(&OwnerId::Player) {
                continue;
            }

            let Some(creature_data) = game_data.monsters.get(&creature.creature_id) else {
                continue;
            };
            let attack_type = creature_data.combat.attack_type.clone();
            let attack_range = match attack_type.as_str() {
                "melee" => game_data.config.combat_ranges.melee,
                "ranged" => game_data.config.combat_ranges.ranged,
                "magic" => game_data.config.combat_ranges.magic,
                _ => game_data.config.combat_ranges.melee,
            };
            let manhattan_dist =
                (entity.pos.x - target_pos.x).abs() + (entity.pos.y - target_pos.y).abs();

            if manhattan_dist > attack_range
                || rng::rand() >= creature_data.combat.attack_speed * dt
            {
                continue;
            }

            let min = creature_data.combat.damage_range[0];
            let max = creature_data.combat.damage_range[1];
            total_damage += rng::gen_range(min, max);
            attackers.push((entity.id, entity.visual_pos, attack_type));
        }

        for (attacker_id, visual_pos, attack_type) in attackers {
            self.projectiles.spawn_at_position(
                visual_pos,
                target_pos,
                &attack_type,
                attacker_id,
                0.0,
            );
        }

        if total_damage > 0.0 && !self.cheat_immortal_heart {
            self.dungeon_heart_health -= total_damage;
            eprintln!(
                "Heart taking damage! Health: {:.1} -> {:.1}",
                self.dungeon_heart_health + total_damage,
                self.dungeon_heart_health
            );
            if self.dungeon_heart_health <= 0.0 && !self.game_over {
                self.game_over = true;
                self.notifications
                    .danger("DUNGEON HEART DESTROYED! GAME OVER");
            }
        }
    }
}

#[cfg(test)]
mod tests;
