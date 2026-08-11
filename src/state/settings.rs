//! Persistent game settings (fullscreen, UI scale, difficulty)
//!
//! Wraps the shared `macroquad_toolkit::settings::GameSettings` (fullscreen,
//! UI text scale, audio/display flags) and adds dungeon_manager's
//! game-specific `difficulty` field. Persisted via the toolkit's
//! platform-uniform JSON-key persistence (native app-data file / WASM
//! localStorage), the same mechanism `save_system` uses.

use macroquad_toolkit::settings::GameSettings as ToolkitSettings;
use serde::{Deserialize, Serialize};

const GAME_NAME: &str = "dungeon_manager";
const SETTINGS_KEY: &str = "settings";

/// UI text scale steps cycled by the settings menu.
pub const UI_SCALE_STEPS: &[f32] = &[0.8, 1.0, 1.25];

/// Global game difficulty. Scales hero *threat* — how fast the surface's
/// garrisons replenish and how often they launch waves — layered on top of each
/// mission's authored `threat_multiplier`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    /// Multiplier on hero threat. `> 1.0` = harder (faster/bigger pressure).
    pub fn threat_scale(self) -> f32 {
        match self {
            Difficulty::Easy => 0.7,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.4,
        }
    }

    pub fn next(self) -> Difficulty {
        let idx = Self::ALL.iter().position(|d| *d == self).unwrap_or(1);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameSettings {
    /// Shared fullscreen/UI-scale/audio settings from the toolkit.
    #[serde(flatten)]
    pub base: ToolkitSettings,
    #[serde(default)]
    pub difficulty: Difficulty,
}

impl GameSettings {
    pub fn load() -> Self {
        macroquad_toolkit::persistence::load_json_key(GAME_NAME, SETTINGS_KEY).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Err(e) = macroquad_toolkit::persistence::save_json_key(GAME_NAME, SETTINGS_KEY, self)
        {
            eprintln!("Failed to save settings: {}", e);
        }
    }

    /// Push the current settings into the window and UI systems.
    pub fn apply(&self) {
        self.base.apply_display();
    }

    pub fn toggle_fullscreen(&mut self) {
        self.base.toggle_fullscreen();
        self.save();
    }

    pub fn cycle_ui_text_scale(&mut self) {
        let current = UI_SCALE_STEPS
            .iter()
            .position(|scale| (scale - self.base.ui_text_scale).abs() < 0.01)
            .unwrap_or(1);
        self.base.ui_text_scale = UI_SCALE_STEPS[(current + 1) % UI_SCALE_STEPS.len()];
        self.apply();
        self.save();
    }

    pub fn cycle_difficulty(&mut self) {
        self.difficulty = self.difficulty.next();
        self.save();
    }
}

#[cfg(test)]
mod tests;
