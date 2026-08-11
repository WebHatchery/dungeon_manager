//! Save-slot browser rendering.
//!
//! Pure view: it draws the rows a [`SlotBrowser`] already gathered and returns
//! nothing. Clicks are hit-tested by the input layer against the same
//! `menu_layout::slot_rows` rects, so what the player sees and what they hit
//! cannot drift apart.

use crate::state::interaction::{SlotBrowser, SlotBrowserEntry};
use crate::ui::core::colors;
use crate::ui::menu_layout;
use macroquad::prelude::*;
use macroquad_toolkit::ui::{button_rect_tone, draw_ui_text, measure_ui_text, ButtonTone};

/// In-game elapsed time as `h:mm:ss` (or `m:ss` under an hour) — a save is
/// identified by how far into a mission it is, so the number has to read as a
/// duration rather than as a count of seconds.
fn format_elapsed(seconds: f32) -> String {
    let total = seconds.max(0.0) as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Mission ids are snake_case in the data; render them as words.
fn humanize(scenario_id: &str) -> String {
    scenario_id
        .split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The line under a slot's name: what is in it, or that nothing is.
fn describe(entry: &SlotBrowserEntry) -> String {
    match (&entry.meta, entry.occupied) {
        (Some(meta), _) => format!(
            "{}   ·   Wave {}   ·   {}",
            humanize(&meta.scenario_id),
            meta.wave,
            format_elapsed(meta.in_game_seconds)
        ),
        // Occupied but unreadable: a save from before the header existed.
        (None, true) => "Saved game (earlier version)".to_string(),
        (None, false) => "Empty".to_string(),
    }
}

pub fn draw(browser: &SlotBrowser) {
    let title = browser.purpose.title();
    let dims = measure_ui_text(title, None, 48, 1.0);
    draw_ui_text(
        title,
        screen_width() / 2.0 - dims.width / 2.0,
        100.0,
        48.0,
        colors::ACCENT_GOLD,
    );

    let rows = menu_layout::slot_rows(browser.entries.len());
    let mouse = mouse_position();
    let mouse = vec2(mouse.0, mouse.1);

    for (entry, rect) in browser.entries.iter().zip(rows.iter()) {
        let selectable = browser.is_selectable(entry);
        let hovered = selectable && rect.contains(mouse);

        let mut bg = if entry.occupied {
            colors::PANEL
        } else {
            colors::PANEL_DARK
        };
        if hovered {
            bg = colors::PANEL_BORDER;
        }
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::PANEL_BORDER);

        let name_color = if selectable {
            colors::TEXT
        } else {
            colors::TEXT_DIM
        };
        let mut name = entry.slot.to_string();
        if entry.latest {
            name.push_str("   [latest]");
        }
        draw_ui_text(&name, rect.x + 18.0, rect.y + 28.0, 22.0, name_color);
        draw_ui_text(
            &describe(entry),
            rect.x + 18.0,
            rect.y + 56.0,
            17.0,
            colors::TEXT_DIM,
        );
    }

    let _ = button_rect_tone(
        menu_layout::slot_browser_back(),
        "BACK",
        true,
        ButtonTone::Secondary,
    );
}

#[cfg(test)]
mod tests;
