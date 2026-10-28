//! Mission objectives panel for authored campaign scenarios

use crate::data::GameData;
use crate::engine::{objectives, tutorial_system};
use crate::state::game_state::GameState;
use crate::ui::core::{colors, HUD_HEIGHT};
use macroquad::prelude::*;
use macroquad_toolkit::ui::{draw_surface_with_title, draw_ui_text, SurfaceStyle, TextStyle};

const PANEL_WIDTH: f32 = 400.0;
const ROW_HEIGHT: f32 = 22.0;

/// List the active scenario's objectives with progress, below the HUD. Yields
/// to the tutorial panel, which occupies the same corner while it is showing.
pub fn draw_mission_objectives(state: &GameState, game_data: &GameData) {
    if state.game_over || tutorial_system::current_step(state, game_data).is_some() {
        return;
    }
    let Some(scenario) = state
        .active_scenario_id
        .as_deref()
        .and_then(|id| game_data.scenarios.get(id))
    else {
        return;
    };
    if scenario.objectives.is_empty() {
        return;
    }

    let rect = Rect::new(
        10.0,
        HUD_HEIGHT + 10.0,
        PANEL_WIDTH,
        42.0 + scenario.objectives.len() as f32 * ROW_HEIGHT,
    );
    let style = SurfaceStyle::new(Color::new(0.08, 0.08, 0.12, 0.85))
        .with_border(1.0, colors::PANEL_BORDER)
        .with_header(30.0, Color::new(0.14, 0.12, 0.08, 1.0))
        .with_header_divider(1.0, colors::ACCENT_GOLD);
    draw_surface_with_title(
        rect,
        Some("Mission Objectives"),
        &style,
        TextStyle::new(18.0, colors::ACCENT_GOLD),
    );

    let mut y = rect.y + 50.0;
    for objective in &scenario.objectives {
        let done = objectives::objective_done(objective, state);
        let mut text = format!(
            "{} {}",
            if done { "[x]" } else { "[ ]" },
            objective.display_text()
        );
        if let (false, Some((current, target))) =
            (done, objectives::objective_progress(objective, state))
        {
            text.push_str(&format!(" ({current}/{target})"));
        }
        let color = if done { colors::TEXT_DIM } else { colors::TEXT };
        draw_ui_text(&text, rect.x + 12.0, y, 16.0, color);
        y += ROW_HEIGHT;
    }
}
