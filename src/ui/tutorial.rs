//! Tutorial objective panel and scenario intro overlay

use crate::data::GameData;
use crate::engine::tutorial_system;
use crate::state::game_state::GameState;
use crate::ui::core::{colors, HUD_HEIGHT};
use macroquad::prelude::*;
use macroquad_toolkit::ui::{
    button_rect_tone, draw_surface_with_title, draw_text_centered_in_box, draw_ui_text, wrap_text,
    ButtonTone, SurfaceStyle, TextStyle,
};

const PANEL_WIDTH: f32 = 320.0;

/// Draw the current tutorial objective in a small panel below the HUD.
pub fn draw_tutorial_panel(state: &GameState) {
    let Some(step) = tutorial_system::current_step(state) else {
        return;
    };

    let title = match tutorial_system::step_progress(state) {
        Some((current, target)) => format!("Objective: {} ({}/{})", step.title, current, target),
        None => format!("Objective: {}", step.title),
    };

    let hint_lines = wrap_text(step.hint, PANEL_WIDTH - 24.0, 16.0);
    let panel_height = 42.0 + hint_lines.len() as f32 * 20.0 + 10.0;
    let rect = Rect::new(10.0, HUD_HEIGHT + 10.0, PANEL_WIDTH, panel_height);

    let style = SurfaceStyle::new(Color::new(0.08, 0.08, 0.12, 0.92))
        .with_border(1.0, colors::PANEL_BORDER)
        .with_header(30.0, Color::new(0.14, 0.12, 0.08, 1.0))
        .with_header_divider(1.0, colors::ACCENT_GOLD);
    draw_surface_with_title(
        rect,
        Some(&title),
        &style,
        TextStyle::new(18.0, colors::ACCENT_GOLD),
    );

    let mut y = rect.y + 50.0;
    for line in &hint_lines {
        draw_ui_text(line, rect.x + 12.0, y, 16.0, colors::TEXT);
        y += 20.0;
    }
}

/// Draw the full-screen scenario intro. Returns true while the intro is up.
pub fn draw_intro_overlay(state: &GameState, game_data: &GameData) -> bool {
    let Some(intro) = tutorial_system::pending_intro(state, game_data) else {
        return false;
    };

    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.02, 0.02, 0.04, 0.88),
    );

    let scenario_name = state
        .active_scenario_id
        .as_deref()
        .and_then(|id| game_data.scenarios.get(id))
        .map(|scenario| scenario.meta.name.as_str())
        .unwrap_or("A New Dominion");

    let center_x = screen_width() / 2.0;
    let panel_width = 640.0_f32.min(screen_width() - 40.0);
    let line_height = 28.0;
    let panel_height = 120.0 + intro.len() as f32 * line_height + 70.0;
    let rect = Rect::new(
        center_x - panel_width / 2.0,
        (screen_height() - panel_height) / 2.0,
        panel_width,
        panel_height,
    );

    let style = SurfaceStyle::new(Color::new(0.07, 0.06, 0.09, 0.97))
        .with_border(2.0, colors::ACCENT_GOLD)
        .with_shadow(vec2(6.0, 6.0), Color::new(0.0, 0.0, 0.0, 0.6));
    draw_surface_with_title(rect, None, &style, TextStyle::new(20.0, colors::TEXT));

    draw_text_centered_in_box(
        scenario_name,
        rect.x,
        rect.y + 24.0,
        rect.w,
        40.0,
        36.0,
        colors::ACCENT_GOLD,
    );

    let mut y = rect.y + 100.0;
    for line in intro {
        draw_text_centered_in_box(
            line,
            rect.x + 20.0,
            y,
            rect.w - 40.0,
            line_height,
            18.0,
            colors::TEXT,
        );
        y += line_height;
    }

    let begin = crate::ui::menu_layout::intro_begin(intro.len());
    let _ = button_rect_tone(begin, "BEGIN MISSION", true, ButtonTone::Primary);
    draw_text_centered_in_box(
        "Tap Begin Mission or press Enter",
        rect.x,
        rect.y + rect.h - 92.0,
        rect.w,
        24.0,
        16.0,
        GRAY,
    );

    true
}
