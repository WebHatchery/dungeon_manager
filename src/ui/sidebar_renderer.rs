use crate::state::entities::EntityId;
use crate::state::player_state::PlayerState;
use crate::ui::sidebar::{
    Sidebar, SidebarTab, BUTTON_SIZE, BUTTON_SPACING, PADDING, PANEL_HEIGHT, RIGHT_MARGIN,
    TAB_HEIGHT,
};
use crate::InteractionMode;
use macroquad::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text, measure_ui_text};
use std::collections::HashMap;

mod cheats;
mod magic;
mod minions;

pub fn draw_sidebar(
    sidebar: &Sidebar,
    current_mode: &InteractionMode,
    player: &PlayerState,
    game_data: &crate::data::GameData,
    held_entity: Option<EntityId>,
    selected_entity: Option<EntityId>,
    selected_room: Option<usize>,
    entities: &crate::state::entities::EntityManager,
    rooms: &[crate::engine::room_validator::Room],
    graphics: Option<&crate::ui::resources::GraphicsCache>,
    save_available: bool,
) {
    if !sidebar.is_expanded {
        // Draw just tabs
        draw_tabs(sidebar);
        return;
    }

    // Draw Panel Background
    draw_rectangle(
        0.0,
        sidebar.panel_y,
        screen_width() - RIGHT_MARGIN,
        PANEL_HEIGHT,
        Color::new(0.1, 0.1, 0.12, 0.95),
    );
    draw_line(
        0.0,
        sidebar.panel_y,
        screen_width() - RIGHT_MARGIN,
        sidebar.panel_y,
        2.0,
        Color::new(0.3, 0.3, 0.35, 1.0),
    );

    draw_tabs(sidebar);

    // Draw Content based on Tab
    match sidebar.current_tab {
        SidebarTab::Build => draw_build_content(sidebar, current_mode, player, game_data),
        SidebarTab::Magic => {
            magic::draw_magic_content(sidebar, player, &game_data.spells, graphics)
        }
        SidebarTab::Minions => minions::draw_minions_content(
            sidebar,
            current_mode,
            held_entity,
            selected_entity,
            selected_room,
            entities,
            rooms,
        ),
        SidebarTab::Traps => draw_traps_content(sidebar, current_mode, player, game_data),
        SidebarTab::Research => draw_research_content(sidebar, player, &game_data.technologies),
        SidebarTab::Utils => draw_utils_content(sidebar, current_mode, save_available),
        SidebarTab::Cheats => cheats::draw_cheats_content(sidebar),
    }

    // Draw selected spell hint if any
    if let Some(spell_id) = &sidebar.selected_spell {
        draw_ui_text(
            &format!(
                "Casting: {} (Left Click to Cast, Right Click to Cancel)",
                spell_id
            ),
            20.0,
            sidebar.panel_y - 40.0,
            20.0,
            WHITE,
        );
    }
}

fn draw_tabs(sidebar: &Sidebar) {
    let mut tabs: Vec<(SidebarTab, &str)> = vec![
        (SidebarTab::Build, "Build"),
        (SidebarTab::Magic, "Magic"),
        (SidebarTab::Minions, "Inspect"),
        (SidebarTab::Traps, "Traps"),
        (SidebarTab::Research, "Tech"),
        (SidebarTab::Utils, "Utils"),
    ];

    // Only show Cheats tab if enabled (F1 to toggle)
    if sidebar.cheats_visible {
        tabs.push((SidebarTab::Cheats, "Cheats"));
    }

    let tab_width = 100.0;
    let start_x = 20.0;

    for (i, (tab, label)) in tabs.iter().enumerate() {
        let x = start_x + i as f32 * tab_width;
        let y = sidebar.panel_y - TAB_HEIGHT;

        let is_active = sidebar.current_tab == *tab && sidebar.is_expanded;

        let color = if is_active {
            Color::new(0.1, 0.1, 0.12, 0.95) // Same as panel
        } else {
            Color::new(0.05, 0.05, 0.08, 0.9) // Darker
        };

        draw_rectangle(x, y, tab_width, TAB_HEIGHT, color);
        draw_rectangle_lines(
            x,
            y,
            tab_width,
            TAB_HEIGHT,
            1.0,
            Color::new(0.3, 0.3, 0.35, 1.0),
        );

        let text_color = if is_active { WHITE } else { GRAY };

        // Center text
        let text_size = measure_ui_text(label, None, 16, 1.0);
        draw_ui_text(
            label,
            x + (tab_width - text_size.width) / 2.0,
            y + 20.0,
            16.0,
            text_color,
        );
    }

    // Draw toggle button
    let toggle_x = screen_width() - RIGHT_MARGIN - 40.0;
    let toggle_y = sidebar.panel_y - TAB_HEIGHT;
    draw_rectangle(
        toggle_x,
        toggle_y,
        40.0,
        TAB_HEIGHT,
        Color::new(0.2, 0.2, 0.2, 0.9),
    );
    draw_ui_text(
        if sidebar.is_expanded { "v" } else { "^" },
        toggle_x + 15.0,
        toggle_y + 20.0,
        16.0,
        WHITE,
    );
}

fn draw_utils_content(sidebar: &Sidebar, current_mode: &InteractionMode, save_available: bool) {
    let start_x = PADDING;
    let start_y = sidebar.panel_y + PADDING;

    // Save Game Button
    let btn_width = 150.0;
    let btn_height = BUTTON_SIZE;
    let spacing = 20.0;

    let save_x = start_x;
    let save_y = start_y;

    let is_save_selected = interaction_modes_match(current_mode, &InteractionMode::SaveGame);

    let save_color = if is_save_selected {
        Color::new(0.2, 0.6, 0.3, 1.0)
    } else {
        Color::new(0.25, 0.25, 0.3, 1.0)
    };

    draw_rectangle(save_x, save_y, btn_width, btn_height, save_color);
    draw_rectangle_lines(save_x, save_y, btn_width, btn_height, 2.0, WHITE);
    draw_ui_text("Save Game", save_x + 30.0, save_y + 30.0, 16.0, WHITE);

    // Load Game Button
    let load_x = save_x + btn_width + spacing;
    let load_y = start_y;

    let load_color = if save_available {
        Color::new(0.3, 0.4, 0.6, 1.0)
    } else {
        Color::new(0.2, 0.2, 0.2, 0.5)
    };

    draw_rectangle(load_x, load_y, btn_width, btn_height, load_color);
    draw_rectangle_lines(
        load_x,
        load_y,
        btn_width,
        btn_height,
        2.0,
        if save_available { WHITE } else { GRAY },
    );
    draw_ui_text(
        "Load Game",
        load_x + 30.0,
        load_y + 30.0,
        16.0,
        if save_available { WHITE } else { GRAY },
    );
}

fn interaction_modes_match(m1: &InteractionMode, m2: &InteractionMode) -> bool {
    m1 == m2
}
fn draw_build_content(
    sidebar: &Sidebar,
    current_mode: &InteractionMode,
    player: &PlayerState,
    game_data: &crate::data::GameData,
) {
    // Start with Dig
    let mut layout = vec![("Dig".to_string(), InteractionMode::Dig, 0, "1".to_string())];

    // Add rooms dynamically from data
    let mut rooms: Vec<&crate::data::rooms::RoomData> = game_data.rooms.values().collect();
    // Sort by cost, then by name for stability
    rooms.sort_by(
        |a, b| match a.build.cost_per_tile.cmp(&b.build.cost_per_tile) {
            std::cmp::Ordering::Equal => a.name.cmp(&b.name),
            other => other,
        },
    );

    for room in rooms {
        // Skip dungeon heart
        if room.id == "dungeon_heart" {
            continue;
        }

        // Assign hotkeys for common rooms to maintain familiarity
        let hotkey = match room.id.as_str() {
            "lair" => "2",
            "hatchery" => "3",
            "treasury" => "4",
            "training_hall" => "T",
            "library" => "L",
            "workshop" => "W",
            "guard_post" => "G",
            "prison" => "P",
            _ => "",
        }
        .to_string();

        layout.push((
            room.name.clone(),
            InteractionMode::BuildRoom(room.id.clone()),
            room.build.cost_per_tile,
            hotkey,
        ));
    }

    // Add utility items
    let spawner_cost = game_data
        .tiles
        .get("monster_spawner")
        .and_then(|t| t.cost)
        .unwrap_or(50);
    layout.push((
        "Spawner".to_string(),
        InteractionMode::PlaceSpawner,
        spawner_cost,
        "5".to_string(),
    ));
    let bridge_cost = game_data
        .tiles
        .get("bridge")
        .and_then(|tile| tile.cost)
        .unwrap_or(50);
    layout.push((
        "Bridge".to_string(),
        InteractionMode::BuildRoom("bridge".to_string()),
        bridge_cost,
        "B".to_string(),
    ));
    layout.push((
        "Sell/Cancel".to_string(),
        InteractionMode::Sell,
        0,
        "X".to_string(),
    ));

    let start_x = PADDING;
    let start_y = sidebar.panel_y + PADDING;
    let mut current_x = start_x;
    let mut current_y = start_y;
    let mut tooltip_to_draw: Option<((f32, f32), Vec<String>)> = None;

    for (label, mode, cost, hotkey) in layout {
        let width = BUTTON_SIZE * 2.5;
        if current_x + width > screen_width() - RIGHT_MARGIN {
            current_x = start_x;
            current_y += BUTTON_SIZE + BUTTON_SPACING;
        }

        let is_selected = interaction_modes_match(current_mode, &mode);
        let can_afford = player.gold >= cost;

        // Check lock status
        let is_locked = if let InteractionMode::BuildRoom(room_id) = &mode {
            room_id != "bridge" && !player.is_room_unlocked(room_id)
        } else {
            false
        };

        let color = if is_locked {
            Color::new(0.1, 0.1, 0.1, 0.8) // Dark Gray for Locked
        } else if is_selected {
            Color::new(0.2, 0.6, 0.3, 1.0)
        } else if !can_afford {
            Color::new(0.3, 0.1, 0.1, 1.0)
        } else {
            Color::new(0.25, 0.25, 0.3, 1.0)
        };

        draw_rectangle(current_x, current_y, width, BUTTON_SIZE, color);
        draw_rectangle_lines(
            current_x,
            current_y,
            width,
            BUTTON_SIZE,
            2.0,
            Color::new(0.4, 0.4, 0.5, 1.0),
        );

        draw_ui_text(
            &label,
            current_x + 5.0,
            current_y + 18.0,
            16.0,
            if is_locked { GRAY } else { WHITE },
        );
        if is_locked {
            draw_ui_text("LOCKED", current_x + 5.0, current_y + 40.0, 14.0, RED);
        } else {
            if cost > 0 {
                draw_ui_text(
                    &format!("{}g", cost),
                    current_x + 5.0,
                    current_y + 40.0,
                    14.0,
                    GOLD,
                );
            }
            if !hotkey.is_empty() {
                draw_ui_text(
                    &hotkey,
                    current_x + width - 15.0,
                    current_y + 15.0,
                    12.0,
                    GRAY,
                );
            }
        }

        // Tooltip logic for locked rooms
        let mouse = mouse_position();
        let is_hovered = mouse.0 >= current_x
            && mouse.0 <= current_x + width
            && mouse.1 >= current_y
            && mouse.1 <= current_y + BUTTON_SIZE;

        if is_hovered && is_locked {
            if let InteractionMode::BuildRoom(room_id) = &mode {
                let mut lines = Vec::new();
                lines.push("LOCKED".to_string());

                if let Some(room_data) = game_data.rooms.get(room_id) {
                    // Research requirement, read off the tech that actually
                    // unlocks the room rather than a parallel list that could
                    // name a different tech (the scavenger room did).
                    if let Some(tech) = crate::data::technologies::tech_unlocking_room(
                        room_id,
                        &game_data.technologies,
                    ) {
                        lines.push(format!("Requires: {}", tech.name));
                    }

                    // Room requirements
                    for req_room_id in &room_data.requirements.global_rooms_required {
                        let room_name = game_data
                            .rooms
                            .get(req_room_id)
                            .map(|r| r.name.clone())
                            .unwrap_or(req_room_id.clone());
                        lines.push(format!("Requires: {}", room_name));
                    }
                }

                tooltip_to_draw = Some((mouse, lines));
            }
        }

        current_x += width + BUTTON_SPACING;
    }

    if let Some((pos, lines)) = tooltip_to_draw {
        draw_tooltip(pos, lines);
    }
}

fn draw_tooltip(pos: (f32, f32), lines: Vec<String>) {
    if lines.is_empty() {
        return;
    }

    let font_size = 18.0;
    let padding = 8.0;
    let mut max_width = 0.0f32;

    for line in &lines {
        let dims = measure_ui_text(line, None, font_size as u16, 1.0);
        if dims.width > max_width {
            max_width = dims.width;
        }
    }

    let box_width = max_width + padding * 2.0;
    let box_height = (font_size + 4.0) * lines.len() as f32 + padding * 2.0;

    // Offset slightly
    let mut draw_x = pos.0 + 15.0;
    let mut draw_y = pos.1 + 15.0;

    // Prevent going off screen
    if draw_x + box_width > screen_width() {
        draw_x = pos.0 - box_width - 5.0;
    }
    if draw_y + box_height > screen_height() {
        draw_y = pos.1 - box_height - 5.0;
    }

    let surface = macroquad_toolkit::ui::SurfaceStyle::new(Color::new(0.1, 0.1, 0.1, 0.95))
        .with_border(1.0, WHITE);
    macroquad_toolkit::ui::draw_surface(Rect::new(draw_x, draw_y, box_width, box_height), &surface);

    for (i, line) in lines.iter().enumerate() {
        let color = if i == 0 { RED } else { WHITE }; // Header is red
        draw_ui_text(
            line,
            draw_x + padding,
            draw_y + padding + (i as f32 * (font_size + 4.0)) + font_size - 4.0,
            font_size,
            color,
        );
    }
}

fn draw_traps_content(
    sidebar: &Sidebar,
    current_mode: &InteractionMode,
    player: &PlayerState,
    game_data: &crate::data::GameData,
) {
    for (button, rect) in
        crate::ui::trap_buttons::trap_button_layout(sidebar.panel_y, player, game_data)
    {
        let is_selected = interaction_modes_match(current_mode, &button.mode);
        let color = if !button.unlocked {
            Color::new(0.1, 0.1, 0.1, 0.8)
        } else if is_selected {
            Color::new(0.2, 0.6, 0.3, 1.0)
        } else if button.stock == 0 {
            Color::new(0.3, 0.1, 0.1, 1.0)
        } else {
            Color::new(0.25, 0.25, 0.3, 1.0)
        };

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            Color::new(0.4, 0.4, 0.5, 1.0),
        );

        draw_ui_text(
            &button.label,
            rect.x + 5.0,
            rect.y + 18.0,
            16.0,
            if button.unlocked { WHITE } else { GRAY },
        );
        if !button.unlocked {
            draw_ui_text("LOCKED", rect.x + 5.0, rect.y + 40.0, 14.0, RED);
        } else {
            draw_ui_text(
                &format!("Stock: {}  {}m", button.stock, button.cost),
                rect.x + 5.0,
                rect.y + 40.0,
                14.0,
                WHITE,
            );
        }
        draw_ui_text(
            &button.hotkey,
            rect.x + rect.w - 15.0,
            rect.y + 15.0,
            12.0,
            GRAY,
        );
    }

    draw_ui_text(
        &format!(
            "Workshop stock | Materials: {} / {}",
            player.materials, player.max_materials
        ),
        screen_width() - RIGHT_MARGIN - 250.0,
        sidebar.panel_y + 30.0,
        20.0,
        WHITE,
    );
}

fn draw_research_content(
    sidebar: &Sidebar,
    player: &PlayerState,
    technologies: &HashMap<String, crate::data::TechData>,
) {
    let start_x = PADDING;
    let start_y = sidebar.panel_y + PADDING;

    let mut sorted_techs: Vec<&crate::data::TechData> = technologies.values().collect();
    // Sort by cost
    sorted_techs.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());

    let mut current_x = start_x;
    let mut current_y = start_y;

    for tech in sorted_techs {
        let width = BUTTON_SIZE * 3.5;

        if current_x + width > screen_width() - RIGHT_MARGIN {
            current_x = start_x;
            current_y += BUTTON_SIZE * 1.2 + BUTTON_SPACING;
        }

        let is_completed = player.is_tech_completed(&tech.id);
        let is_active = player
            .active_research
            .as_ref()
            .map(|id| id == &tech.id)
            .unwrap_or(false);

        // Check prerequisites
        let prereqs_met = tech
            .prerequisites
            .iter()
            .all(|req| player.is_tech_completed(req));

        // Determine color/state
        let (bg_color, text_color) = if is_completed {
            (Color::new(0.2, 0.4, 0.2, 1.0), GRAY) // Dark Green
        } else if is_active {
            (Color::new(0.2, 0.6, 0.8, 1.0), WHITE) // Blue
        } else if prereqs_met {
            (Color::new(0.3, 0.3, 0.35, 1.0), WHITE) // Available
        } else {
            (Color::new(0.15, 0.15, 0.15, 0.5), DARKGRAY) // Locked
        };

        // Draw Button
        draw_rectangle(current_x, current_y, width, BUTTON_SIZE * 1.2, bg_color);
        draw_rectangle_lines(
            current_x,
            current_y,
            width,
            BUTTON_SIZE * 1.2,
            2.0,
            Color::new(0.4, 0.4, 0.5, 1.0),
        );

        // Tech Name
        draw_ui_text(
            &tech.name,
            current_x + 5.0,
            current_y + 15.0,
            16.0,
            text_color,
        );

        // Cost / Progress
        if is_completed {
            draw_ui_text(
                "Completed",
                current_x + 5.0,
                current_y + 35.0,
                14.0,
                text_color,
            );
        } else if is_active {
            let progress = player.research_progress;
            let pct = (progress / tech.cost).clamp(0.0, 1.0);

            // Draw progress bar
            let bar_w = width - 10.0;
            draw_rectangle(current_x + 5.0, current_y + 35.0, bar_w, 10.0, BLACK);
            draw_rectangle(current_x + 5.0, current_y + 35.0, bar_w * pct, 10.0, GREEN);

            draw_ui_text(
                &format!("{:.0}/{:.0}", progress, tech.cost),
                current_x + 5.0,
                current_y + 30.0,
                12.0,
                WHITE,
            );
        } else if prereqs_met {
            draw_ui_text(
                &format!("Cost: {:.0}", tech.cost),
                current_x + 5.0,
                current_y + 35.0,
                14.0,
                GOLD,
            );
        } else {
            draw_ui_text("Locked", current_x + 5.0, current_y + 35.0, 14.0, RED);
        }

        // Move to next position
        current_x += width + BUTTON_SPACING;
    }

    // Show Research Points generation rate (implied from libraries)
    // Hard to calculate here without iterating all rooms/creatures, but let's show status text
    if let Some(active) = &player.active_research {
        if let Some(tech) = technologies.get(active) {
            draw_ui_text(
                &format!(
                    "Researching: {} ({:.1}%)",
                    tech.name,
                    (player.research_progress / tech.cost) * 100.0
                ),
                screen_width() - RIGHT_MARGIN - 300.0,
                sidebar.panel_y + 30.0,
                18.0,
                LIGHTGRAY,
            );
        }
    } else {
        draw_ui_text(
            "No Active Research",
            screen_width() - RIGHT_MARGIN - 300.0,
            sidebar.panel_y + 30.0,
            18.0,
            GRAY,
        );
    }
}
