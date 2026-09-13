//! Input handling for the main menu, settings, mission select, and skirmish
//! setup screens. Layout rects are shared with the renderer via
//! `crate::ui::menu_layout`.

use crate::data::GameData;
use crate::state::game_state::GameState;
use crate::state::interaction::{SlotBrowser, SlotBrowserPurpose};
use crate::state::{GamePhase, MapType};
use macroquad::prelude::*;

/// Which row of an open browser the mouse just clicked, if any.
///
/// Shared by the main-menu browser and the in-game overlay so the two cannot
/// hit-test the same rows differently. Returns `None` for an unselectable row
/// (an empty slot when loading), which is the same as clicking the background.
pub(crate) fn clicked_slot(browser: &SlotBrowser) -> Option<crate::state::save_system::SaveSlot> {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return None;
    }
    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);
    let rows = crate::ui::menu_layout::slot_rows(browser.entries.len());

    browser
        .entries
        .iter()
        .zip(rows.iter())
        .find(|(entry, rect)| rect.contains(mouse) && browser.is_selectable(entry))
        .map(|(entry, _)| entry.slot)
}

/// Which occupied row's destructive button was just pressed, if any.
pub(crate) fn clicked_slot_delete(
    browser: &SlotBrowser,
) -> Option<crate::state::save_system::SaveSlot> {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return None;
    }
    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);
    let rows = crate::ui::menu_layout::slot_rows(browser.entries.len());

    browser
        .entries
        .iter()
        .zip(rows.iter())
        .find(|(entry, rect)| {
            entry.occupied && crate::ui::menu_layout::slot_delete_button(**rect).contains(mouse)
        })
        .map(|(entry, _)| entry.slot)
}

/// Did the mouse just click the browser's Back button?
pub(crate) fn clicked_slot_browser_back() -> bool {
    let mouse = mouse_position();
    is_mouse_button_pressed(MouseButton::Left)
        && crate::ui::menu_layout::slot_browser_back().contains(vec2(mouse.0, mouse.1))
}

/// The main-menu slot browser: load the chosen slot, or go back.
pub(super) fn handle_load_game(phase: &mut GamePhase) {
    let GamePhase::LoadGame(browser) = phase else {
        return;
    };

    if clicked_slot_browser_back() || is_key_pressed(KeyCode::Escape) {
        *phase = GamePhase::MainMenu;
        return;
    }

    if let Some(slot) = clicked_slot_delete(browser) {
        match crate::state::save_system::delete_game(slot) {
            Ok(()) => {
                *phase = GamePhase::LoadGame(SlotBrowser::open(browser.purpose));
            }
            Err(error) => {
                eprintln!("Failed to delete {slot}: {error}");
            }
        }
        return;
    }

    let Some(slot) = clicked_slot(browser) else {
        return;
    };

    match crate::state::save_system::load_game(slot) {
        Ok(loaded_state) => {
            println!("Game loaded successfully from {slot}!");
            *phase = GamePhase::Playing(loaded_state);
        }
        Err(e) => {
            // Nothing here can notify — the notification queue lives on
            // `GameState`, and at the main menu there is no game yet. Returning
            // to the menu at least does not strand the player on a dead screen.
            eprintln!("Failed to load {slot}: {e}");
            *phase = GamePhase::MainMenu;
        }
    }
}

pub(super) fn handle_main_menu(
    selected_map_type: &mut MapType,
    phase: &mut GamePhase,
    game_data: &mut Option<GameData>,
    save_available: bool,
) {
    let layout = crate::ui::menu_layout::main_menu();
    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);
    let clicked = |rect: macroquad::math::Rect| {
        is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse)
    };

    // Start Game (Space also starts)
    if is_key_pressed(KeyCode::Space) || clicked(layout.start) {
        // Force Standard Map type
        *selected_map_type = MapType::Standard;

        if let Some(ref data) = game_data {
            if let Some(campaign) = data.campaigns.get("deep_dominion") {
                // Open the mission-select screen so the player can pick which
                // unlocked mission to play (notably the M7 branch).
                let progress = crate::data::campaign::CampaignProgress::new(campaign);
                *phase = GamePhase::MissionSelect(progress);
            } else {
                let game_state = GameState::new_with_map_type(
                    data.config.map_size.width,
                    data.config.map_size.height,
                    data,
                    selected_map_type.clone(),
                );
                *phase = GamePhase::Playing(game_state);
            }
        }
        return;
    }

    // Load Game opens the slot browser rather than loading anything itself. The
    // browser reads every slot once, here, on the click — never in a draw.
    if clicked(layout.load) && save_available {
        *phase = GamePhase::LoadGame(SlotBrowser::open(SlotBrowserPurpose::Load));
        return;
    }

    // Skirmish setup (procedural sandbox)
    if clicked(layout.skirmish) {
        *phase = GamePhase::SkirmishSetup(crate::state::skirmish::SkirmishConfig::default());
        return;
    }

    // Settings
    if clicked(layout.settings) {
        *phase = GamePhase::Settings;
        return;
    }

    // Exit Game (native only)
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(exit) = layout.exit {
        if clicked(exit) {
            std::process::exit(0);
        }
    }
}

pub(super) fn handle_mission_select(
    phase: &mut GamePhase,
    game_data: &Option<GameData>,
    settings: &crate::state::settings::GameSettings,
) {
    let GamePhase::MissionSelect(progress) = phase else {
        return;
    };
    // Decouple from `phase` so we can reassign it below.
    let progress = progress.clone();
    let Some(data) = game_data.as_ref() else {
        return;
    };
    let Some(campaign) = data.campaigns.get(&progress.campaign_id) else {
        *phase = GamePhase::MainMenu;
        return;
    };

    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);
    let clicked = |rect: macroquad::math::Rect| {
        is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse)
    };

    if is_key_pressed(KeyCode::Escape) || clicked(crate::ui::menu_layout::mission_select_back()) {
        *phase = GamePhase::MainMenu;
        return;
    }

    let entries = progress.mission_menu(campaign);
    let rows = crate::ui::menu_layout::mission_select_rows(entries.len());
    for (entry, rect) in entries.iter().zip(rows.iter()) {
        if matches!(entry.status, crate::data::campaign::MissionStatus::Locked) {
            continue;
        }
        if clicked(*rect) {
            let mut chosen = progress.clone();
            chosen.select_mission(campaign, &entry.id);
            *phase = match GameState::new_for_campaign_progress(data, chosen) {
                Some(mut state) => {
                    state.difficulty = settings.difficulty;
                    GamePhase::Playing(state)
                }
                None => GamePhase::MainMenu,
            };
            return;
        }
    }
}

pub(super) fn handle_skirmish_setup(
    phase: &mut GamePhase,
    game_data: &Option<GameData>,
    settings: &crate::state::settings::GameSettings,
) {
    let GamePhase::SkirmishSetup(config) = phase else {
        return;
    };
    let layout = crate::ui::menu_layout::skirmish_setup();
    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);
    let clicked = |rect: macroquad::math::Rect| {
        is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse)
    };

    if is_key_pressed(KeyCode::Escape) || clicked(layout.back) {
        *phase = GamePhase::MainMenu;
        return;
    }
    if clicked(layout.map_type) {
        config.cycle_map_type();
        return;
    }
    if clicked(layout.size) {
        config.cycle_size();
        return;
    }
    if clicked(layout.start) || is_key_pressed(KeyCode::Enter) {
        if let Some(data) = game_data.as_ref() {
            let (w, h) = config.dimensions();
            let mut state = GameState::new_with_map_type(w, h, data, config.map_type());
            state.difficulty = settings.difficulty;
            *phase = GamePhase::Playing(state);
        }
    }
}

pub(super) fn handle_settings(
    phase: &mut GamePhase,
    settings: &mut crate::state::settings::GameSettings,
) {
    let layout = crate::ui::menu_layout::settings_menu();
    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);
    let clicked = |rect: macroquad::math::Rect| {
        is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse)
    };

    if clicked(layout.fullscreen) {
        settings.toggle_fullscreen();
    }

    if clicked(layout.ui_scale) {
        settings.cycle_ui_text_scale();
    }

    if clicked(layout.difficulty) {
        settings.cycle_difficulty();
    }

    if clicked(layout.autosave) {
        settings.base.autosave_enabled = !settings.base.autosave_enabled;
        settings.save();
    }

    if clicked(layout.back) || is_key_pressed(KeyCode::Escape) {
        *phase = GamePhase::MainMenu;
    }
}
