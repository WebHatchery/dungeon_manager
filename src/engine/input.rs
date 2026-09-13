//! Top-level input dispatch: routes per-frame input to the handler for the
//! current `GamePhase`. Phase-specific logic lives in the submodules
//! (`menus`, `playing`, `tile_actions`).

mod menus;
mod playing;
mod tile_actions;

use crate::data::GameData;
use crate::state::entities::EntityId;
use crate::state::{DragSelection, GamePhase, InteractionMode, MapType, TilePos};
use crate::ui::actions::ActionQueue;
use crate::ui::sidebar::Sidebar;
use macroquad::prelude::*;

pub struct InputHandler;

impl InputHandler {
    pub fn update(
        dt: f32,
        phase: &mut GamePhase,
        game_data: &mut Option<GameData>,
        interaction_mode: &mut InteractionMode,
        selected_map_type: &mut MapType,
        hovered_tile: &mut Option<TilePos>,
        held_entity: &mut Option<EntityId>,
        selected_entity: &mut Option<EntityId>,
        selected_room: &mut Option<usize>,
        sidebar: &mut Sidebar,
        action_queue: &mut ActionQueue,
        drag_selection: &mut DragSelection,
        settings: &mut crate::state::settings::GameSettings,
        save_available: bool,
    ) {
        match phase {
            GamePhase::Loading => {
                // Loading is handled asynchronously in main loop
            }
            GamePhase::MainMenu => {
                menus::handle_main_menu(selected_map_type, phase, game_data, save_available);
            }
            GamePhase::Settings => {
                menus::handle_settings(phase, settings);
            }
            GamePhase::MissionSelect(_) => {
                menus::handle_mission_select(phase, game_data, settings);
            }
            GamePhase::SkirmishSetup(_) => {
                menus::handle_skirmish_setup(phase, game_data, settings);
            }
            GamePhase::LoadGame(_) => {
                menus::handle_load_game(phase);
            }
            GamePhase::Playing(state) => {
                if let Some(ref data) = game_data {
                    // Scenario intro overlay: freeze the game until dismissed so
                    // the player can read the story before timers start
                    if crate::engine::tutorial_system::pending_intro(state, data).is_some() {
                        let begin = crate::ui::menu_layout::intro_begin(
                            crate::engine::tutorial_system::pending_intro(state, data)
                                .map(|intro| intro.len())
                                .unwrap_or_default(),
                        );
                        let mouse = mouse_position();
                        let begin_clicked = is_mouse_button_released(MouseButton::Left)
                            && begin.contains(vec2(mouse.0, mouse.1));
                        if begin_clicked
                            || is_key_pressed(KeyCode::Enter)
                            || is_key_pressed(KeyCode::Space)
                            || is_key_pressed(KeyCode::Escape)
                        {
                            state.tutorial.intro_dismissed = true;
                        }
                        return;
                    }

                    if !state.paused {
                        state.update(dt, data);
                    }

                    if playing::handle_playing(
                        dt,
                        state,
                        data,
                        interaction_mode,
                        hovered_tile,
                        held_entity,
                        selected_entity,
                        selected_room,
                        sidebar,
                        action_queue,
                        drag_selection,
                        save_available,
                    ) {
                        *phase = GamePhase::MainMenu;
                    }
                }
            }
        }
    }
}
