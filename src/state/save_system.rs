//! Saving and loading, and the slot identity that both hang off.
//!
//! # Two things were wrong here, and the slot literal was the visible one
//!
//! `"slot_1"` used to be a string literal at eleven call sites, so the game had
//! exactly one save and every new game silently overwrote it. Underneath that,
//! the native path wrote `save_slot_1.json` as a **relative** path — i.e. into
//! whatever directory the process happened to start in. Launch the game from a
//! shortcut, from Explorer, or from a store client and the save is somewhere
//! else, or the directory is not writable at all.
//!
//! Both are fixed by going through `macroquad_toolkit::persistence::slots`,
//! which this project had never adopted despite the toolkit carrying it: it
//! resolves `{app_data}/dungeon_manager/save_<slot>.json` on native and a
//! game-qualified key in the browser. See [`load_legacy`] for what happens to
//! saves written by the old build.

use crate::state::game_state::GameState;
// Imported by function rather than by module: the toolkit exports its own
// `SaveSlot` (a metadata header), and this module's `SaveSlot` is a slot
// *identity*. Naming both would be the drift hazard, not the convenience.
use macroquad_toolkit::persistence::{load_from_slot, save_to_slot_with_version, slot_exists};
use serde::{Deserialize, Serialize};

const GAME_NAME: &str = "dungeon_manager";
const SAVE_FORMAT_VERSION: &str = "0.1.0";

/// How many numbered slots the player gets.
pub const SLOT_COUNT: u8 = 3;

/// Which slot a session reads and writes.
///
/// An enum rather than a bare number because the autosave is a *kind* of slot,
/// not a fourth numbered one: the whole point of it is that it cannot land on
/// the save a player chose. Numbered slots stay validated at construction, so an
/// out-of-range slot cannot resolve to a filename nothing else knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SaveSlot {
    Numbered(u8),
    /// Written by the timer, never by the player. Loadable like any other.
    Auto,
}

impl Default for SaveSlot {
    fn default() -> Self {
        Self::Numbered(1)
    }
}

impl SaveSlot {
    /// `None` outside `1..=SLOT_COUNT`.
    pub fn new(number: u8) -> Option<Self> {
        (1..=SLOT_COUNT)
            .contains(&number)
            .then_some(Self::Numbered(number))
    }

    /// The numbered slots, in order. Excludes the autosave: this is the list a
    /// player picks *into*, and the autosave is not a choice they make.
    pub fn all() -> impl Iterator<Item = Self> {
        (1..=SLOT_COUNT).map(Self::Numbered)
    }

    /// Every slot that can hold a save, autosave included — what a *load* list
    /// offers.
    pub fn all_loadable() -> impl Iterator<Item = Self> {
        Self::all().chain(std::iter::once(Self::Auto))
    }

    pub fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

    /// The name the persistence layer files this slot under.
    fn key(self) -> String {
        match self {
            Self::Numbered(n) => format!("slot_{n}"),
            // Matches the name the toolkit's own slot enumeration expects.
            Self::Auto => "autosave".to_string(),
        }
    }
}

impl std::fmt::Display for SaveSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numbered(n) => write!(f, "Slot {n}"),
            Self::Auto => write!(f, "Autosave"),
        }
    }
}

/// Written beside the game state so a slot can be *described* without
/// deserializing a whole dungeon — which is what a save picker needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMeta {
    /// Mission the save is in, or `"skirmish"` when there is no scenario.
    pub scenario_id: String,
    /// Hero wave reached.
    pub wave: u32,
    /// In-game seconds elapsed.
    pub in_game_seconds: f32,
    /// Wall-clock seconds since the Unix epoch, for ordering slots by recency.
    pub saved_at: f64,
    pub version: String,
}

impl SaveMeta {
    fn describe(state: &GameState) -> Self {
        Self {
            scenario_id: state
                .scenario_runtime
                .as_ref()
                .map(|runtime| runtime.scenario_id.clone())
                .unwrap_or_else(|| "skirmish".to_string()),
            wave: state.hero_base.current_wave_number,
            in_game_seconds: state.time_elapsed,
            // `date::now` is the one clock that exists on both native and wasm;
            // `std::time` does not compile for the web build.
            saved_at: macroquad::miniquad::date::now(),
            version: SAVE_FORMAT_VERSION.to_string(),
        }
    }
}

#[derive(Serialize)]
struct SaveFile<'a> {
    meta: SaveMeta,
    game_state: &'a GameState,
}

#[derive(Deserialize)]
struct LoadFile {
    game_state: GameState,
}

/// Just the header. Unknown fields are ignored, so this reads a full save
/// without paying to construct the `GameState` inside it.
#[derive(Deserialize)]
struct MetaOnly {
    meta: SaveMeta,
}

/// The shape the pre-slot build wrote: no `meta`, and on native at a relative
/// path. Kept only to be read.
#[derive(Deserialize)]
struct LegacySave {
    game_state: GameState,
}

/// Save to a slot.
pub fn save_game(game_state: &GameState, slot: SaveSlot) -> Result<(), String> {
    let file = SaveFile {
        meta: SaveMeta::describe(game_state),
        game_state,
    };
    save_to_slot_with_version(GAME_NAME, &slot.key(), &file, SAVE_FORMAT_VERSION)
}

/// Load from a slot, falling back to a save the old build wrote.
///
/// Stamps `active_slot` on the way out. `active_slot` is `#[serde(skip)]`, so a
/// freshly loaded state defaults to slot 1 no matter which slot it came from —
/// without this, loading slot 3 and then saving would write over slot 1. Doing
/// it here rather than at each call site means a future loader cannot forget.
pub fn load_game(slot: SaveSlot) -> Result<GameState, String> {
    let mut state = match load_from_slot::<LoadFile>(GAME_NAME, &slot.key()) {
        Ok(file) => file.game_state,
        Err(current) => load_legacy(slot)
            .map_err(|legacy| format!("No save in {slot} ({current}; legacy: {legacy})"))?,
    };
    state.active_slot = slot;
    Ok(state)
}

/// Whether the autosave timer should be running at all this frame.
///
/// A paused game, a finished one, and an unread mission briefing all mean the
/// player is not making progress worth preserving. The pause case is the one
/// that matters most: a timer that ran while the pause menu sat open would
/// autosave the instant someone came back to the keyboard, which is exactly
/// when they are least expecting their save to change.
pub fn should_autosave(state: &GameState) -> bool {
    !state.paused && !state.game_over && state.tutorial.intro_dismissed
}

/// Is there anything in this slot?
///
/// Deliberately existence-only: the menus call this every frame while they are
/// on screen, so it must not parse the save. Use [`peek_slot`] when the
/// contents actually matter — from a click, not from a draw.
pub fn save_exists(slot: SaveSlot) -> bool {
    slot_exists(GAME_NAME, &slot.key()) || legacy_exists(slot)
}

/// Any slot at all — what a "LOAD GAME" button needs to know. Counts the
/// autosave: it is the slot most likely to be the only one a new player has.
pub fn any_save_exists() -> bool {
    SaveSlot::all_loadable().any(save_exists)
}

/// Describe a slot for a picker. `None` if empty, or if it is a legacy save
/// (which carries no header — it still *loads*, it just cannot be summarised).
pub fn peek_slot(slot: SaveSlot) -> Option<SaveMeta> {
    load_from_slot::<MetaOnly>(GAME_NAME, &slot.key())
        .ok()
        .map(|peeked| peeked.meta)
}

/// The slot to offer when the player has not picked one — most recently saved,
/// falling back to the lowest occupied slot for legacy saves with no timestamp.
///
/// Parses every occupied slot, so call it from a click and not from a draw.
pub fn most_recent_slot() -> Option<SaveSlot> {
    let occupied: Vec<SaveSlot> = SaveSlot::all_loadable()
        .filter(|slot| save_exists(*slot))
        .collect();
    occupied
        .iter()
        .filter_map(|slot| peek_slot(*slot).map(|meta| (*slot, meta.saved_at)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(slot, _)| slot)
        .or_else(|| occupied.first().copied())
}

/// Read a save written before slots went through the toolkit.
///
/// Native builds wrote a *relative* `save_slot_N.json`, so this looks where the
/// process is running rather than in app data — it finds the file only if the
/// game is launched from the same directory as before, which is the same
/// fragility that motivated the move. The browser key happens to be unchanged
/// (`keys::storage_key` and `slots::storage_key` both produce
/// `dungeon_manager_save_slot_N`), so only the wrapper shape differs there.
///
/// Nothing writes the legacy shape any more: the next save lands in the new
/// location, which is the migration.
fn load_legacy(slot: SaveSlot) -> Result<GameState, String> {
    let key = format!("save_{}", slot.key());

    #[cfg(target_arch = "wasm32")]
    {
        // Two browser shapes, both old. The game-qualified key is what the
        // previous build wrote; the bare key is what the build before *that*
        // wrote, back when localStorage was unqualified and every macroquad game
        // on the host shared a keyspace. The toolkit's slot loader already tries
        // both keys, but parses them as the current wrapper — so a save in
        // either place with the old shape reaches here, and dropping this branch
        // would quietly orphan a browser player's game.
        macroquad_toolkit::persistence::load_json_key::<LegacySave>(GAME_NAME, &key)
            .map(|legacy| legacy.game_state)
            .or_else(|qualified| {
                let raw = crate::state::wasm_storage::storage_get(&key)
                    .ok_or_else(|| format!("no unqualified browser save either ({qualified})"))?;
                serde_json::from_str::<LegacySave>(&raw)
                    .map(|legacy| legacy.game_state)
                    .map_err(|e| format!("unqualified browser save is unreadable: {e}"))
            })
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let legacy: LegacySave = macroquad_toolkit::persistence::load_json(format!("{key}.json"))
            .map_err(|e| e.to_string())?;
        Ok(legacy.game_state)
    }
}

fn legacy_exists(slot: SaveSlot) -> bool {
    let key = format!("save_{}", slot.key());

    #[cfg(target_arch = "wasm32")]
    {
        macroquad_toolkit::persistence::json_key_exists(GAME_NAME, &key)
            || crate::state::wasm_storage::storage_exists(&key)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        macroquad_toolkit::persistence::file_exists(format!("{key}.json"))
    }
}

#[cfg(test)]
mod tests;
