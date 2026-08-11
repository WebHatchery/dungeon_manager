use crate::state::game_state::GameState;

pub enum GamePhase {
    Loading,
    MainMenu,
    Settings,
    /// Campaign map / mission-select: the player picks which unlocked mission
    /// to play (needed so the M7 branch is actually choosable). Holds the
    /// campaign progress until a mission is chosen.
    MissionSelect(crate::data::campaign::CampaignProgress),
    /// Skirmish/sandbox setup: pick a procedural map type + size, then launch a
    /// generated one-off game (makes the map generator reachable from the UI).
    SkirmishSetup(crate::state::skirmish::SkirmishConfig),
    /// Save-slot browser reached from the main menu, where the only thing a
    /// player can do with a slot is load it. The in-game browser is an overlay
    /// on `GameState` instead, because leaving `Playing` to pick a slot would
    /// mean putting the running game somewhere while the player chooses.
    LoadGame(SlotBrowser),
    Playing(GameState),
}

/// What a slot browser does with the row the player clicks.
///
/// Save and load are the same list, the same geometry and the same rows — only
/// the verb differs, and the warning attached to it. Modelling that as one
/// browser with a purpose keeps a "save over slot 2" and a "load slot 2" from
/// being two screens that have to be kept looking alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotBrowserPurpose {
    Save,
    Load,
}

impl SlotBrowserPurpose {
    pub fn title(self) -> &'static str {
        match self {
            Self::Save => "SAVE TO SLOT",
            Self::Load => "LOAD FROM SLOT",
        }
    }

    /// An empty slot is a valid target to save into and nothing to load from.
    pub fn allows_empty_slot(self) -> bool {
        matches!(self, Self::Save)
    }
}

/// One row of the browser.
pub struct SlotBrowserEntry {
    pub slot: crate::state::save_system::SaveSlot,
    /// `None` for an empty slot, and also for a save written by the old build,
    /// which has no header to read. The second still loads — it just cannot be
    /// described, so it renders as occupied-but-unknown rather than as empty.
    pub meta: Option<crate::state::save_system::SaveMeta>,
    pub occupied: bool,
    /// Most recently written slot, for a one-word hint about where "continue"
    /// would go. Uses the stamp that already orders the slots.
    pub latest: bool,
}

/// An open slot browser, holding what it read when it opened.
///
/// The rows are gathered **once, on open** rather than per frame on purpose:
/// describing a slot means parsing its save, and a draw path that parses three
/// save files every frame is the per-frame-IO problem this project already has
/// a note about. A browser is a snapshot; it is reopened, not refreshed.
pub struct SlotBrowser {
    pub purpose: SlotBrowserPurpose,
    pub entries: Vec<SlotBrowserEntry>,
}

impl SlotBrowser {
    pub fn open(purpose: SlotBrowserPurpose) -> Self {
        use crate::state::save_system::{most_recent_slot, peek_slot, save_exists, SaveSlot};

        let latest = most_recent_slot();
        // Loading offers the autosave; saving does not list it, because the
        // next autosave would take it back.
        let slots: Vec<SaveSlot> = match purpose {
            SlotBrowserPurpose::Load => SaveSlot::all_loadable().collect(),
            SlotBrowserPurpose::Save => SaveSlot::all().collect(),
        };

        let entries = slots
            .into_iter()
            .map(|slot| SlotBrowserEntry {
                slot,
                meta: peek_slot(slot),
                occupied: save_exists(slot),
                latest: latest == Some(slot),
            })
            .collect();

        Self { purpose, entries }
    }

    /// Whether clicking this row should do anything.
    pub fn is_selectable(&self, entry: &SlotBrowserEntry) -> bool {
        // The autosave is never a save target even if a row for it is somehow on
        // screen — stated here rather than left to `open` not listing it, so the
        // rule survives someone changing what `open` lists.
        if entry.slot.is_auto() && !matches!(self.purpose, SlotBrowserPurpose::Load) {
            return false;
        }
        entry.occupied || self.purpose.allows_empty_slot()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionMode {
    None,
    Dig,
    BuildRoom(String), // room_type_id
    BuildTrap(String), // trap_type_id
    PlaceSpawner,
    Pickup,
    Drop,
    Sell,    // Sell room or cancel task
    Inspect, // Inspect minion details
    SetAttackMarker,
    SetDefendMarker,
    SaveGame,
    Summon(String, crate::state::entities::EntityCategory, u32), // id, category, level
}

#[cfg(test)]
mod slot_browser_tests;
