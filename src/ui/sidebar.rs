//! Sidebar UI - Combined build menu, spell bar, and minion management
//!
//! Unifies all player controls into a single bottom panel with tabs.

use crate::data::spells::SpellData;
use crate::state::entities::EntityId;
use crate::state::player_state::PlayerState;
use crate::InteractionMode;
use macroquad::prelude::*;
use std::collections::HashMap;

use crate::state::entities::EntityCategory;

pub const PANEL_HEIGHT: f32 = 180.0;
pub const TAB_HEIGHT: f32 = 30.0;
pub const BUTTON_SIZE: f32 = 48.0;
pub const BUTTON_SPACING: f32 = 10.0;
pub const PADDING: f32 = 10.0;
pub const RIGHT_MARGIN: f32 = 170.0;

/// Get color based on efficiency value
pub fn efficiency_color(efficiency: f32) -> macroquad::prelude::Color {
    use macroquad::prelude::*;
    if efficiency >= 0.9 {
        GREEN
    } else if efficiency >= 0.5 {
        YELLOW
    } else {
        RED
    }
}

#[derive(Debug, Clone)]
pub struct CheatState {
    pub category: EntityCategory,
    pub entity_id: String,
    pub level: u32,
    pub instant_dig_active: bool,
    pub immortal_heart_active: bool,
}

impl Default for CheatState {
    fn default() -> Self {
        Self {
            category: EntityCategory::Monster,
            entity_id: "goblin".to_string(),
            level: 1,
            instant_dig_active: false,
            immortal_heart_active: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarTab {
    Build,
    Magic,
    Minions,
    Traps,
    Research,
    Utils, // Save/Load, Settings, etc.
    Cheats,
}

pub struct Sidebar {
    pub current_tab: SidebarTab,
    pub is_expanded: bool,
    pub panel_y: f32, // Animated Y position

    // UI State
    pub selected_spell: Option<String>,
    pub cheat_state: CheatState,
    pub cheats_visible: bool, // Toggled with F1
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            current_tab: SidebarTab::Build,
            is_expanded: true,
            panel_y: screen_height() - PANEL_HEIGHT,
            selected_spell: None,
            cheat_state: CheatState::default(),
            cheats_visible: false,
        }
    }

    pub fn switch_to_tab(&mut self, tab: SidebarTab) {
        self.current_tab = tab;
        self.is_expanded = true;
    }

    pub fn update_layout(&mut self) {
        let target_y = if self.is_expanded {
            screen_height() - PANEL_HEIGHT
        } else {
            screen_height() - TAB_HEIGHT
        };

        // Simple lerp for smooth animation could go here, for now snap
        self.panel_y = target_y;
    }

    pub fn handle_input(
        &mut self,
        player: &PlayerState,
        game_data: &crate::data::GameData,
        _current_mode: &InteractionMode,
        held_entity: Option<EntityId>,
        action_queue: &mut crate::ui::actions::ActionQueue,
    ) -> Option<InteractionMode> {
        let mouse_pos = mouse_position();

        // F1 toggles cheats menu visibility (debug builds only)
        #[cfg(debug_assertions)]
        if is_key_pressed(KeyCode::F1) {
            self.cheats_visible = !self.cheats_visible;
            // If hiding cheats and currently on cheats tab, switch to build
            if !self.cheats_visible && self.current_tab == SidebarTab::Cheats {
                self.current_tab = SidebarTab::Build;
            }
        }

        // Handle Tab Switching
        if mouse_pos.1 >= self.panel_y - TAB_HEIGHT
            && mouse_pos.1 <= self.panel_y
            && is_mouse_button_pressed(MouseButton::Left)
        {
            let tab_width = 100.0;
            let start_x = 20.0;

            if mouse_pos.0 >= start_x && mouse_pos.0 < start_x + tab_width {
                self.current_tab = SidebarTab::Build;
                self.is_expanded = true;
            } else if mouse_pos.0 >= start_x + tab_width && mouse_pos.0 < start_x + tab_width * 2.0
            {
                self.current_tab = SidebarTab::Magic;
                self.is_expanded = true;
            } else if mouse_pos.0 >= start_x + tab_width * 2.0
                && mouse_pos.0 < start_x + tab_width * 3.0
            {
                self.current_tab = SidebarTab::Minions;
                self.is_expanded = true;
            } else if mouse_pos.0 >= start_x + tab_width * 3.0
                && mouse_pos.0 < start_x + tab_width * 4.0
            {
                self.current_tab = SidebarTab::Traps;
                self.is_expanded = true;
            } else if mouse_pos.0 >= start_x + tab_width * 4.0
                && mouse_pos.0 < start_x + tab_width * 5.0
            {
                self.current_tab = SidebarTab::Research;
                self.is_expanded = true;
            } else if mouse_pos.0 >= start_x + tab_width * 5.0
                && mouse_pos.0 < start_x + tab_width * 6.0
            {
                self.current_tab = SidebarTab::Utils;
                self.is_expanded = true;
            } else if self.cheats_visible
                && mouse_pos.0 >= start_x + tab_width * 6.0
                && mouse_pos.0 < start_x + tab_width * 7.0
            {
                self.current_tab = SidebarTab::Cheats;
                self.is_expanded = true;
            } else if mouse_pos.0 >= screen_width() - RIGHT_MARGIN - 40.0
                && mouse_pos.0 <= screen_width() - RIGHT_MARGIN
            {
                // Toggle expand/collapse
                self.is_expanded = !self.is_expanded;
            }
        }

        if !self.is_expanded {
            return None;
        }

        // Handle Content Clicks
        if mouse_pos.1 >= self.panel_y
            && mouse_pos.0 < screen_width() - RIGHT_MARGIN
            && is_mouse_button_pressed(MouseButton::Left)
        {
            match self.current_tab {
                SidebarTab::Build => {
                    return self.handle_build_tab_click(mouse_pos, player, game_data);
                }
                SidebarTab::Magic => {
                    return self.handle_magic_tab_click(mouse_pos, player, &game_data.spells);
                }
                SidebarTab::Minions => {
                    return self.handle_minions_tab_click(mouse_pos, held_entity);
                }
                SidebarTab::Traps => {
                    return self.handle_traps_tab_click(mouse_pos, player, game_data);
                }
                SidebarTab::Research => {
                    self.handle_research_tab_click(
                        mouse_pos,
                        player,
                        &game_data.technologies,
                        action_queue,
                    );
                    return None;
                }
                SidebarTab::Utils => {
                    self.handle_utils_tab_click(mouse_pos, action_queue);
                    return None;
                }
                SidebarTab::Cheats => {
                    return self.handle_cheats_tab_click(mouse_pos, action_queue, game_data);
                }
            }
        }

        // Handle Cancel / Deselect
        if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
            self.selected_spell = None;
            return Some(InteractionMode::None);
        }

        None
    }

    fn handle_build_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        player: &PlayerState,
        game_data: &crate::data::GameData,
    ) -> Option<InteractionMode> {
        let start_x = PADDING;
        let start_y = self.panel_y + PADDING;

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
            // Check if room is dungeon_heart - skip it
            if room.id == "dungeon_heart" {
                continue;
            }

            layout.push((
                room.name.clone(),
                InteractionMode::BuildRoom(room.id.clone()),
                room.build.cost_per_tile,
                String::new(), // Hotkey doesn't matter for click handler
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

        let mut current_x = start_x;
        let mut current_y = start_y;

        for (label, mode, cost, _hotkey) in layout {
            // Suppress unused label warning
            let _ = label;

            let width = BUTTON_SIZE * 2.5;
            if current_x + width > screen_width() - RIGHT_MARGIN {
                current_x = start_x;
                current_y += BUTTON_SIZE + BUTTON_SPACING;
            }

            if mouse_pos.0 >= current_x
                && mouse_pos.0 <= current_x + width
                && mouse_pos.1 >= current_y
                && mouse_pos.1 <= current_y + BUTTON_SIZE
            {
                // Check if unlocked
                let is_locked = if let InteractionMode::BuildRoom(room_id) = &mode {
                    room_id != "bridge" && !player.is_room_unlocked(room_id)
                } else {
                    false
                };

                if !is_locked && player.gold >= cost {
                    self.selected_spell = None; // Deselect spell if switching to build
                    return Some(mode);
                }
            }

            current_x += width + BUTTON_SPACING;
        }

        None
    }

    fn handle_magic_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        player: &PlayerState,
        spells: &HashMap<String, SpellData>,
    ) -> Option<InteractionMode> {
        let start_x = PADDING;
        let start_y = self.panel_y + PADDING;

        // Dynamic sorted list of unlocked spells
        let mut sorted_spells: Vec<&String> = spells
            .keys()
            .filter(|id| player.is_spell_unlocked(id))
            .collect();

        // Sort by mana cost, then name
        sorted_spells.sort_by(|a, b| {
            let cost_a = spells.get(*a).map(|s| s.cost.mana).unwrap_or(0);
            let cost_b = spells.get(*b).map(|s| s.cost.mana).unwrap_or(0);
            match cost_a.cmp(&cost_b) {
                std::cmp::Ordering::Equal => a.cmp(b),
                other => other,
            }
        });

        let mut current_x = start_x;
        let mut current_y = start_y;

        for spell_id in sorted_spells.iter() {
            let width = BUTTON_SIZE;

            if current_x + width > screen_width() - RIGHT_MARGIN {
                current_x = start_x;
                current_y += BUTTON_SIZE + BUTTON_SPACING;
            }

            if mouse_pos.0 >= current_x
                && mouse_pos.0 <= current_x + width
                && mouse_pos.1 >= current_y
                && mouse_pos.1 <= current_y + BUTTON_SIZE
            {
                // Check cost and cooldown
                if let Some(data) = spells.get(*spell_id) {
                    if player.gold >= data.cost.gold && player.mana >= data.cost.mana {
                        // Set selected spell
                        self.selected_spell = Some(spell_id.to_string());
                        return Some(InteractionMode::None);
                    }
                }
            }

            current_x += width + BUTTON_SPACING;
        }

        None
    }

    fn handle_minions_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        held_entity: Option<EntityId>,
    ) -> Option<InteractionMode> {
        let start_x = PADDING;
        let start_y = self.panel_y + PADDING;

        // Pickup / Drop Button
        if mouse_pos.0 >= start_x
            && mouse_pos.0 <= start_x + BUTTON_SIZE * 2.5
            && mouse_pos.1 >= start_y
            && mouse_pos.1 <= start_y + BUTTON_SIZE
        {
            if held_entity.is_some() {
                return Some(InteractionMode::Drop);
            } else {
                return Some(InteractionMode::Pickup);
            }
        }

        // Inspect Button
        let inspect_x = start_x + BUTTON_SIZE * 2.5 + BUTTON_SPACING;
        if mouse_pos.0 >= inspect_x
            && mouse_pos.0 <= inspect_x + BUTTON_SIZE * 2.5
            && mouse_pos.1 >= start_y
            && mouse_pos.1 <= start_y + BUTTON_SIZE
        {
            return Some(InteractionMode::Inspect);
        }

        // Marker Buttons (New Line)
        let marker_y = start_y + BUTTON_SIZE + BUTTON_SPACING;

        // Attack Marker
        if mouse_pos.0 >= start_x
            && mouse_pos.0 <= start_x + BUTTON_SIZE * 2.5
            && mouse_pos.1 >= marker_y
            && mouse_pos.1 <= marker_y + BUTTON_SIZE
        {
            return Some(InteractionMode::SetAttackMarker);
        }

        // Defend Marker
        let defend_x = start_x + BUTTON_SIZE * 2.5 + BUTTON_SPACING;
        if mouse_pos.0 >= defend_x
            && mouse_pos.0 <= defend_x + BUTTON_SIZE * 2.5
            && mouse_pos.1 >= marker_y
            && mouse_pos.1 <= marker_y + BUTTON_SIZE
        {
            return Some(InteractionMode::SetDefendMarker);
        }

        None
    }

    fn handle_traps_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        player: &PlayerState,
        game_data: &crate::data::GameData,
    ) -> Option<InteractionMode> {
        if let Some(mode) =
            crate::ui::trap_buttons::trap_button_at(self.panel_y, mouse_pos, player, game_data)
        {
            self.selected_spell = None;
            return Some(mode);
        }
        None
    }

    pub fn draw(
        &self,
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
        crate::ui::sidebar_renderer::draw_sidebar(
            self,
            current_mode,
            player,
            game_data,
            held_entity,
            selected_entity,
            selected_room,
            entities,
            rooms,
            graphics,
            save_available,
        );
    }

    pub fn handle_research_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        player: &PlayerState,
        technologies: &HashMap<String, crate::data::TechData>,
        action_queue: &mut crate::ui::actions::ActionQueue,
    ) {
        let start_x = PADDING;
        let start_y = self.panel_y + PADDING;

        let mut sorted_techs: Vec<&crate::data::TechData> = technologies.values().collect();
        sorted_techs.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());

        let mut current_x = start_x;
        let mut current_y = start_y;

        for tech in sorted_techs {
            let width = BUTTON_SIZE * 3.5;
            if current_x + width > screen_width() - RIGHT_MARGIN {
                current_x = start_x;
                current_y += BUTTON_SIZE * 1.2 + BUTTON_SPACING;
            }

            // Hitbox
            if mouse_pos.0 >= current_x
                && mouse_pos.0 <= current_x + width
                && mouse_pos.1 >= current_y
                && mouse_pos.1 <= current_y + BUTTON_SIZE * 1.2
            {
                let is_completed = player.is_tech_completed(&tech.id);
                let is_active = player
                    .active_research
                    .as_ref()
                    .map(|id| id == &tech.id)
                    .unwrap_or(false);
                let prereqs_met = tech
                    .prerequisites
                    .iter()
                    .all(|req| player.is_tech_completed(req));

                if !is_completed && !is_active && prereqs_met {
                    // Start research!
                    action_queue.push(crate::ui::actions::UiAction::StartResearch(tech.id.clone()));
                }
            }

            // Move to next position (must match draw logic)
            current_x += width + BUTTON_SPACING;
        }
    }

    pub fn handle_utils_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        action_queue: &mut crate::ui::actions::ActionQueue,
    ) {
        let start_x = PADDING;
        let start_y = self.panel_y + PADDING;

        // Save Game Button
        let btn_width = 150.0;
        let btn_height = BUTTON_SIZE;
        let spacing = 20.0;

        let save_x = start_x;
        let save_y = start_y;

        if mouse_pos.0 >= save_x
            && mouse_pos.0 <= save_x + btn_width
            && mouse_pos.1 >= save_y
            && mouse_pos.1 <= save_y + btn_height
        {
            action_queue.push(crate::ui::actions::UiAction::SaveGame);
        }

        // Load Game Button
        let load_x = save_x + btn_width + spacing;
        let load_y = start_y;

        if mouse_pos.0 >= load_x
            && mouse_pos.0 <= load_x + btn_width
            && mouse_pos.1 >= load_y
            && mouse_pos.1 <= load_y + btn_height
        {
            action_queue.push(crate::ui::actions::UiAction::LoadGame);
        }
    }

    pub fn get_selected_spell(&self) -> Option<&String> {
        self.selected_spell.as_ref()
    }

    pub fn is_mouse_over(&self) -> bool {
        let mouse_pos = mouse_position();
        mouse_pos.1 >= self.panel_y - TAB_HEIGHT
    }

    pub fn handle_cheats_tab_click(
        &mut self,
        mouse_pos: (f32, f32),
        action_queue: &mut crate::ui::actions::ActionQueue,
        game_data: &crate::data::GameData,
    ) -> Option<InteractionMode> {
        let start_x = PADDING;
        let start_y = self.panel_y + PADDING;

        // --- Row 1: Spawning Controls ---
        let row1_y = start_y;

        // 1. Category Button (Width 120)
        let cat_btn_width = 120.0;
        if mouse_pos.0 >= start_x
            && mouse_pos.0 <= start_x + cat_btn_width
            && mouse_pos.1 >= row1_y
            && mouse_pos.1 <= row1_y + BUTTON_SIZE
        {
            self.cheat_state.category = match self.cheat_state.category {
                EntityCategory::Monster => EntityCategory::Hero,
                EntityCategory::Hero => EntityCategory::Monster,
            };
            // Reset ID
            self.cheat_state.entity_id = match self.cheat_state.category {
                EntityCategory::Monster => game_data
                    .monsters
                    .keys()
                    .next()
                    .unwrap_or(&"goblin".to_string())
                    .clone(),
                EntityCategory::Hero => game_data
                    .heroes
                    .keys()
                    .next()
                    .unwrap_or(&"knight".to_string())
                    .clone(),
            };
        }

        // 2. Level -
        let lvl_minus_x = start_x + cat_btn_width + BUTTON_SPACING;
        if mouse_pos.0 >= lvl_minus_x
            && mouse_pos.0 <= lvl_minus_x + BUTTON_SIZE
            && mouse_pos.1 >= row1_y
            && mouse_pos.1 <= row1_y + BUTTON_SIZE
            && self.cheat_state.level > 1
        {
            self.cheat_state.level -= 1;
        }

        // 3. Level +
        // Logic matches renderer: lvl_plus_x = lvl_text_x + 50.0 + BUTTON_SPACING;
        let lvl_text_x = lvl_minus_x + BUTTON_SIZE + BUTTON_SPACING;
        let lvl_plus_x = lvl_text_x + 50.0 + BUTTON_SPACING;
        if mouse_pos.0 >= lvl_plus_x
            && mouse_pos.0 <= lvl_plus_x + BUTTON_SIZE
            && mouse_pos.1 >= row1_y
            && mouse_pos.1 <= row1_y + BUTTON_SIZE
            && self.cheat_state.level < 10
        {
            self.cheat_state.level += 1;
        }

        // 4. Entity ID (Width 200)
        let id_x = lvl_plus_x + BUTTON_SIZE + BUTTON_SPACING;
        let id_btn_width = 200.0;
        if mouse_pos.0 >= id_x
            && mouse_pos.0 <= id_x + id_btn_width
            && mouse_pos.1 >= row1_y
            && mouse_pos.1 <= row1_y + BUTTON_SIZE
        {
            let ids: Vec<&String> = match self.cheat_state.category {
                EntityCategory::Monster => game_data.monsters.keys().collect(),
                EntityCategory::Hero => game_data.heroes.keys().collect(),
            };
            let current_idx = ids
                .iter()
                .position(|id| **id == self.cheat_state.entity_id)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % ids.len();
            self.cheat_state.entity_id = ids[next_idx].clone();
        }

        // 5. Spawn Button (Width 140)
        let spawn_x = id_x + id_btn_width + BUTTON_SPACING;
        let spawn_btn_width = 140.0;
        if mouse_pos.0 >= spawn_x
            && mouse_pos.0 <= spawn_x + spawn_btn_width
            && mouse_pos.1 >= row1_y
            && mouse_pos.1 <= row1_y + BUTTON_SIZE
        {
            return Some(InteractionMode::Summon(
                self.cheat_state.entity_id.clone(),
                self.cheat_state.category.clone(),
                self.cheat_state.level,
            ));
        }

        // --- Row 2: Cheats & Toggles ---
        let row2_y = row1_y + BUTTON_SIZE + BUTTON_SPACING;

        // 1. Gold (Width 120)
        let gold_btn_width = 120.0;
        if mouse_pos.0 >= start_x
            && mouse_pos.0 <= start_x + gold_btn_width
            && mouse_pos.1 >= row2_y
            && mouse_pos.1 <= row2_y + BUTTON_SIZE
        {
            action_queue.push(crate::ui::actions::UiAction::CheatAddGold(100));
        }

        // 2. Toggle Fog (Width 150)
        let fow_x = start_x + gold_btn_width + BUTTON_SPACING;
        let fow_width = 150.0;
        if mouse_pos.0 >= fow_x
            && mouse_pos.0 <= fow_x + fow_width
            && mouse_pos.1 >= row2_y
            && mouse_pos.1 <= row2_y + BUTTON_SIZE
        {
            action_queue.push(crate::ui::actions::UiAction::CheatToggleFog);
        }

        // 3. Instant Dig (Width 130)
        let dig_x = fow_x + fow_width + BUTTON_SPACING;
        let dig_width = 130.0;
        if mouse_pos.0 >= dig_x
            && mouse_pos.0 <= dig_x + dig_width
            && mouse_pos.1 >= row2_y
            && mouse_pos.1 <= row2_y + BUTTON_SIZE
        {
            self.cheat_state.instant_dig_active = !self.cheat_state.instant_dig_active;
        }

        // 4. God Mode (Width 140)
        let heart_x = dig_x + dig_width + BUTTON_SPACING;
        let heart_width = 140.0;
        if mouse_pos.0 >= heart_x
            && mouse_pos.0 <= heart_x + heart_width
            && mouse_pos.1 >= row2_y
            && mouse_pos.1 <= row2_y + BUTTON_SIZE
        {
            self.cheat_state.immortal_heart_active = !self.cheat_state.immortal_heart_active;
            action_queue.push(crate::ui::actions::UiAction::CheatToggleImmortalHeart);
        }

        None
    }

    pub fn clear_selection(&mut self) {
        self.selected_spell = None;
    }
}
