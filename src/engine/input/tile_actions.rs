//! Mouse interactions with the dungeon itself: spell casting, right-click
//! actions, single-click tile interactions per `InteractionMode`, and
//! drag-selection application.

use crate::data::GameData;
use crate::state::entities::EntityId;
use crate::state::game_state::GameState;
use crate::state::{DragSelection, InteractionMode, TilePos};
use crate::ui::actions::ActionQueue;
use crate::ui::sidebar::{Sidebar, SidebarTab};
use macroquad::prelude::*;

use crate::engine::creature_ai;
use crate::engine::spell_effects;

/// Handle spell casting when a spell is selected
pub(super) fn handle_spell_casting(
    state: &mut GameState,
    game_data: &GameData,
    sidebar: &Sidebar,
    tile_pos: TilePos,
    mouse_over_ui: bool,
) {
    if let Some(spell_id) = sidebar.get_selected_spell().cloned() {
        if is_mouse_button_pressed(MouseButton::Left) && !mouse_over_ui {
            let target_pos = Some(tile_pos);
            let target_entity = None;

            let cast_result =
                spell_effects::cast_spell(&spell_id, state, game_data, target_pos, target_entity);

            // Every refusal used to be an `eprintln!`, so a spell that would
            // not cast simply did nothing on screen. Deliberately exhaustive:
            // a new `CastResult` should fail to compile here rather than fall
            // into a catch-all and become invisible again.
            let spell_name = game_data
                .spells
                .get(&spell_id)
                .map(|spell| spell.name.clone())
                .unwrap_or_else(|| spell_id.clone());

            let refusal = match cast_result {
                spell_effects::CastResult::Success => {
                    state.player.record_spell_cast(spell_id.clone());
                    None
                }
                spell_effects::CastResult::MaxCapReached => {
                    Some("already at maximum capacity".to_string())
                }
                spell_effects::CastResult::InsufficientMana => Some("not enough mana".to_string()),
                spell_effects::CastResult::InsufficientGold => Some("not enough gold".to_string()),
                spell_effects::CastResult::OnCooldown => Some("still recharging".to_string()),
                spell_effects::CastResult::OutOfRange => {
                    Some("that is beyond the heart's reach".to_string())
                }
                spell_effects::CastResult::NotVisible => {
                    Some("you cannot see that place".to_string())
                }
                spell_effects::CastResult::WrongAllegiance => {
                    Some("not a valid target for that spell".to_string())
                }
                spell_effects::CastResult::InvalidTarget => Some("invalid target".to_string()),
            };

            if let Some(reason) = refusal {
                state
                    .notifications
                    .warning(format!("{spell_name}: {reason}."));
            }
        }
    }
}

/// Handle right-click actions (cancel, unmark, slap)
pub(super) fn handle_right_click(
    state: &mut GameState,
    game_data: &GameData,
    interaction_mode: &mut InteractionMode,
    selected_entity: &mut Option<EntityId>,
    sidebar: &mut Sidebar,
    tile_pos: TilePos,
    mouse_over_ui: bool,
    drag_selection: &mut DragSelection,
) {
    if !is_mouse_button_pressed(MouseButton::Right) || mouse_over_ui {
        return;
    }

    drag_selection.cancel();

    match interaction_mode {
        InteractionMode::Dig => {
            if let Some(tile) = state.get_tile_mut(tile_pos) {
                tile.marked_for_dig = false;
            }
        }
        InteractionMode::None => {
            if !try_slap_creature(state, game_data, tile_pos) {
                *selected_entity = None;
                sidebar.clear_selection();
            }
        }
        _ => {
            *interaction_mode = InteractionMode::None;
            sidebar.clear_selection();
        }
    }
}

/// Try to slap a creature at the given position, returns true if successful
pub(super) fn try_slap_creature(
    state: &mut GameState,
    game_data: &GameData,
    tile_pos: TilePos,
) -> bool {
    let entity = match state.entities.at_position_mut(tile_pos).next() {
        Some(e) => e,
        None => return false,
    };
    let creature = match entity.as_creature_mut() {
        Some(c) => c,
        None => return false,
    };
    let monster_data = match game_data.monsters.get(&creature.creature_id) {
        Some(d) => d,
        None => return false,
    };

    creature_ai::apply_slap(creature, monster_data, game_data, state.time_elapsed);
    eprintln!("Slapped creature {}!", creature.creature_id);
    true
}

pub(super) fn unmark_tile(state: &mut GameState, tile_pos: TilePos) {
    if let Some(tile) = state.get_tile_mut(tile_pos) {
        tile.marked_for_dig = false;
    }
}

/// Handle left-click tile interactions based on current mode
pub(super) fn handle_tile_interaction(
    state: &mut GameState,
    game_data: &GameData,
    interaction_mode: &mut InteractionMode,
    held_entity: &mut Option<EntityId>,
    selected_entity: &mut Option<EntityId>,
    selected_room: &mut Option<usize>,
    tile_pos: TilePos,
    sidebar: &mut Sidebar,
    action_queue: &mut ActionQueue,
) {
    match interaction_mode {
        InteractionMode::Dig => {
            if sidebar.cheat_state.instant_dig_active {
                action_queue.push(crate::ui::actions::UiAction::CheatInstantDig(tile_pos));
            } else {
                crate::engine::input_handlers::handle_dig(state, game_data, tile_pos);
            }
        }
        InteractionMode::BuildRoom(room_type) => {
            crate::engine::input_handlers::handle_build_room(state, game_data, room_type, tile_pos);
        }
        InteractionMode::BuildTrap(trap_type) => {
            crate::engine::input_handlers::handle_build_trap(state, game_data, trap_type, tile_pos);
        }
        InteractionMode::PlaceSpawner => {
            crate::engine::input_handlers::handle_place_spawner(state, game_data, tile_pos);
        }
        InteractionMode::Pickup => {
            crate::engine::input_handlers::handle_pickup(
                state,
                held_entity,
                interaction_mode,
                tile_pos,
            );
        }
        InteractionMode::Drop => {
            crate::engine::input_handlers::handle_drop(
                state,
                held_entity,
                interaction_mode,
                tile_pos,
                game_data,
            );
        }
        InteractionMode::Sell => {
            crate::engine::input_handlers::handle_sell(state, game_data, tile_pos);
        }
        InteractionMode::Inspect => {
            crate::engine::input_handlers::handle_inspect(
                state,
                selected_entity,
                selected_room,
                tile_pos,
            );
        }
        InteractionMode::None => {
            let found = crate::engine::input_handlers::select_entity_or_room(
                state,
                selected_entity,
                selected_room,
                tile_pos,
            );
            if found {
                sidebar.switch_to_tab(SidebarTab::Minions);
            }
        }
        InteractionMode::SetAttackMarker => {
            handle_set_marker(state, "attack", tile_pos);
        }
        InteractionMode::SetDefendMarker => {
            handle_set_marker(state, "defend", tile_pos);
        }
        InteractionMode::SaveGame => {
            // No tile interaction for SaveGame
        }
        InteractionMode::Summon(id, category, level) => {
            match category {
                crate::state::entities::EntityCategory::Monster => {
                    let max_health = game_data
                        .monsters
                        .get(id)
                        .map(|m| m.stats.health)
                        .unwrap_or(100.0)
                        * (*level as f32);
                    let max_mana = game_data
                        .monsters
                        .get(id)
                        .map(|m| m.stats.mana)
                        .unwrap_or(20.0);
                    let creature_state = crate::state::entities::CreatureState::new(
                        id.clone(),
                        *level,
                        max_health,
                        max_mana,
                        macroquad_toolkit::rng::gen_range(0, 1000000),
                    );
                    state.entities.spawn_creature(tile_pos, creature_state);
                    state
                        .notifications
                        .success(format!("Summoned Lvl {} {}", level, id));
                }
                crate::state::entities::EntityCategory::Hero => {
                    let max_health = game_data
                        .heroes
                        .get(id)
                        .map(|h| h.stats.health)
                        .unwrap_or(100.0)
                        * (*level as f32);
                    let max_mana = game_data
                        .heroes
                        .get(id)
                        .map(|h| h.stats.mana)
                        .unwrap_or(20.0);
                    let dig_time = game_data
                        .heroes
                        .get(id)
                        .map(|h| h.stats.dig_time)
                        .unwrap_or(1.0);
                    let hero_state = crate::state::entities::HeroState::new(
                        id.clone(),
                        *level,
                        max_health,
                        max_mana,
                        tile_pos,
                        dig_time,
                        macroquad_toolkit::rng::gen_range(0, 1000000),
                    );
                    state.entities.spawn_hero(tile_pos, hero_state);
                    state
                        .notifications
                        .success(format!("Summoned Lvl {} {}", level, id));
                }
            }
            // Reset to None or keep summoning?
            // The user implies they want to summon multiple ("putting them in a room").
            // So we keep the mode active.
        }
    }
}

fn handle_set_marker(state: &mut GameState, marker_type: &str, tile_pos: TilePos) {
    let (width, height) = crate::engine::tile_grid::get_grid_dimensions(&state.dungeon.grid);
    // Allow placing markers on any walkable tile or known tile?
    // For now, allow anywhere inside map bounds
    if tile_pos.x >= 0 && tile_pos.y >= 0 && tile_pos.x < width as i32 && tile_pos.y < height as i32
    {
        if marker_type == "attack" {
            state.attack_marker = Some(tile_pos);
            eprintln!("Attack marker set at {:?}", tile_pos);
            state.notifications.info("Attack flag updated");
        } else if marker_type == "defend" {
            state.defend_marker = Some(tile_pos);
            eprintln!("Defend marker set at {:?}", tile_pos);
            state.notifications.info("Defend flag updated");
        }
    }
}

/// Check if the current interaction mode supports drag selection
pub(super) fn is_drag_mode(mode: &InteractionMode) -> bool {
    matches!(
        mode,
        InteractionMode::Dig
            | InteractionMode::BuildRoom(_)
            | InteractionMode::BuildTrap(_)
            | InteractionMode::PlaceSpawner
    )
}

/// Apply the drag action to all tiles in the selection
pub(super) fn apply_drag_action(
    state: &mut GameState,
    game_data: &GameData,
    mode: &InteractionMode,
    min: TilePos,
    max: TilePos,
    sidebar: &Sidebar,
    action_queue: &mut ActionQueue,
) {
    let tiles: Vec<TilePos> = (min.y..=max.y)
        .flat_map(|y| (min.x..=max.x).map(move |x| TilePos::new(x, y)))
        .collect();

    match mode {
        InteractionMode::Dig => {
            if sidebar.cheat_state.instant_dig_active {
                for tile in tiles {
                    action_queue.push(crate::ui::actions::UiAction::CheatInstantDig(tile));
                }
            } else {
                crate::engine::input_handlers::handle_dig_multi(state, game_data, &tiles);
            }
        }
        InteractionMode::BuildRoom(room_type) => {
            crate::engine::input_handlers::handle_build_room_multi(
                state, game_data, room_type, &tiles,
            );
        }
        InteractionMode::BuildTrap(trap_type) => {
            crate::engine::input_handlers::handle_build_trap_multi(
                state, game_data, trap_type, &tiles,
            );
        }
        InteractionMode::PlaceSpawner => {
            crate::engine::input_handlers::handle_place_spawner_multi(state, game_data, &tiles);
        }
        _ => {}
    }
}
