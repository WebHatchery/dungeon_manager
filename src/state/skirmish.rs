//! Skirmish / sandbox setup.
//!
//! The procedural map generator (`engine::map_generator`) has always been
//! reachable in code but never from the UI — every launch forced
//! `MapType::Standard` at a fixed size. `SkirmishConfig` is the small,
//! testable model behind a skirmish setup screen: the player cycles a map
//! type and a size, and this maps those choices to the `(width, height,
//! MapType)` that `GameState::new_with_map_type` already understands.

use crate::state::game_state::MapType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishConfig {
    /// Index into [`SkirmishConfig::MAP_TYPE_LABELS`].
    pub map_type: usize,
    /// Index into [`SkirmishConfig::SIZE_LABELS`].
    pub size: usize,
}

impl Default for SkirmishConfig {
    fn default() -> Self {
        // Standard terrain, medium map — a sensible neutral starting point.
        Self {
            map_type: 0,
            size: 1,
        }
    }
}

impl SkirmishConfig {
    pub const MAP_TYPE_LABELS: [&'static str; 3] = ["Standard", "Rich", "Hazardous"];
    pub const SIZE_LABELS: [&'static str; 3] = ["Small", "Medium", "Large"];
    /// Square edge length for each size index.
    pub const SIZE_DIMS: [usize; 3] = [24, 32, 48];

    fn map_type_idx(&self) -> usize {
        self.map_type % Self::MAP_TYPE_LABELS.len()
    }

    fn size_idx(&self) -> usize {
        self.size % Self::SIZE_LABELS.len()
    }

    /// The generator map type for the chosen terrain.
    pub fn map_type(&self) -> MapType {
        match self.map_type_idx() {
            1 => MapType::Rich,
            2 => MapType::Hazardous,
            _ => MapType::Standard,
        }
    }

    /// `(width, height)` for the chosen size — the generator uses square maps.
    pub fn dimensions(&self) -> (usize, usize) {
        let dim = Self::SIZE_DIMS[self.size_idx()];
        (dim, dim)
    }

    pub fn map_type_label(&self) -> &'static str {
        Self::MAP_TYPE_LABELS[self.map_type_idx()]
    }

    pub fn size_label(&self) -> &'static str {
        Self::SIZE_LABELS[self.size_idx()]
    }

    pub fn cycle_map_type(&mut self) {
        self.map_type = (self.map_type_idx() + 1) % Self::MAP_TYPE_LABELS.len();
    }

    pub fn cycle_size(&mut self) {
        self.size = (self.size_idx() + 1) % Self::SIZE_LABELS.len();
    }
}

#[cfg(test)]
mod tests;
