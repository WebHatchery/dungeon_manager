use crate::data::GameData;
use crate::state::game_state::GameState;
use crate::state::tile_state::FogState;
use crate::ui::resources::GraphicsCache;
use macroquad::prelude::*;

pub fn draw_entities(
    graphics: &GraphicsCache,
    state: &GameState,
    camera: &Camera3D,
    game_data: &GameData,
) {
    let mut sorted_entities: Vec<_> = state.entities.all().collect();
    sorted_entities.sort_by(|a, b| {
        let dist_a = (camera.position - vec3(a.visual_pos.0, 0.5, a.visual_pos.1)).length_squared();
        let dist_b = (camera.position - vec3(b.visual_pos.0, 0.5, b.visual_pos.1)).length_squared();
        dist_b
            .partial_cmp(&dist_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for entity in sorted_entities {
        let (x, z) = entity.visual_pos;

        if game_data.config.fog_of_war.enabled && state.cheat_fog_enabled {
            let tile_pos = entity.pos;
            if let Some(tile) = state.get_tile(tile_pos) {
                if tile.fog_state == FogState::Hidden {
                    continue;
                }
            }
        }

        if let crate::state::entities::EntityType::Structure(structure) = &entity.entity_type {
            if let Some(texture) = graphics.building_texture(&structure.building_id) {
                crate::draw_utils::draw_billboard(
                    vec3(x, 0.5, z),
                    vec2(1.0, 1.0),
                    texture,
                    camera.position,
                );
                continue;
            }
        }

        if let crate::state::entities::EntityType::ResourcePile(pile) = &entity.entity_type {
            draw_resource_pile(graphics, x, z, &pile.resource_type);
            continue;
        }

        let texture: Option<Texture2D> = if is_polymorphed(entity) {
            None
        } else {
            match &entity.entity_type {
                crate::state::entities::EntityType::Creature(creature) => {
                    graphics.get_creature_texture(&creature.creature_id, creature.visual_seed)
                }
                crate::state::entities::EntityType::Hero(hero) => {
                    graphics.get_hero_texture(&hero.hero_id, hero.visual_seed)
                }
                crate::state::entities::EntityType::Structure(_) => None,
                crate::state::entities::EntityType::ResourcePile(_) => None,
            }
        };

        if let Some(ref texture) = texture {
            crate::draw_utils::draw_billboard(
                vec3(x, 0.5, z),
                vec2(0.8, 0.8),
                texture,
                camera.position,
            );
        } else {
            draw_entity_fallback(entity, x, z);
        }

        draw_recent_health_bar(state, entity, x, z);
    }
}

fn draw_resource_pile(graphics: &GraphicsCache, x: f32, z: f32, resource_type: &str) {
    if resource_type == "gold" {
        if let Some(texture) = graphics.tile_textures.get("gold_pile") {
            draw_cube(vec3(x, 0.1, z), vec3(0.5, 0.2, 0.5), Some(texture), WHITE);
        } else {
            draw_cube(vec3(x, 0.1, z), vec3(0.4, 0.2, 0.4), None, GOLD);
        }
    } else {
        draw_cube(vec3(x, 0.1, z), vec3(0.4, 0.2, 0.4), None, GRAY);
    }
}

fn draw_entity_fallback(entity: &crate::state::entities::Entity, x: f32, z: f32) {
    let color = if is_polymorphed(entity) {
        Color::new(1.0, 0.85, 0.2, 1.0)
    } else {
        match &entity.entity_type {
            crate::state::entities::EntityType::Hero(_) => Color::new(0.2, 0.8, 0.2, 1.0),
            crate::state::entities::EntityType::Creature(_) => Color::new(0.8, 0.2, 0.2, 1.0),
            crate::state::entities::EntityType::Structure(_) => Color::new(0.5, 0.5, 0.5, 1.0),
            crate::state::entities::EntityType::ResourcePile(_) => GOLD,
        }
    };
    draw_cube_wires(vec3(x, 0.5, z), vec3(0.5, 1.0, 0.5), color);
}

fn is_polymorphed(entity: &crate::state::entities::Entity) -> bool {
    let statuses = match &entity.entity_type {
        crate::state::entities::EntityType::Creature(creature) => &creature.status_effects,
        crate::state::entities::EntityType::Hero(hero) => &hero.status_effects,
        _ => return false,
    };
    statuses.iter().any(|status| status.effect_type == "polymorph")
}

fn draw_recent_health_bar(
    state: &GameState,
    entity: &crate::state::entities::Entity,
    x: f32,
    z: f32,
) {
    if state.time_elapsed - entity.last_damage_time >= 3.0 {
        return;
    }

    let (health, max_health) = match &entity.entity_type {
        crate::state::entities::EntityType::Creature(creature) => {
            (creature.health, creature.max_health)
        }
        crate::state::entities::EntityType::Hero(hero) => (hero.health, hero.max_health),
        crate::state::entities::EntityType::Structure(structure) => {
            (structure.health, structure.max_health)
        }
        crate::state::entities::EntityType::ResourcePile(_) => (0.0, 0.0),
    };

    if max_health <= 0.0 {
        return;
    }

    let health_pct = (health / max_health).clamp(0.0f32, 1.0f32);
    let bar_width = 0.8;
    let bar_height = 0.1;
    let y_offset = 1.2;

    draw_cube(
        vec3(x, y_offset, z),
        vec3(bar_width, bar_height, 0.05),
        None,
        BLACK,
    );

    let x_shift = -0.5 * bar_width * (1.0 - health_pct);
    let bar_color = if health_pct < 0.3 { RED } else { GREEN };

    draw_cube(
        vec3(x + x_shift, y_offset, z),
        vec3(bar_width * health_pct, bar_height, 0.06),
        None,
        bar_color,
    );
}
