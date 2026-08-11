use super::*;
use crate::state::save_system::{SaveMeta, SaveSlot};

fn entry(slot: u8, occupied: bool) -> SlotBrowserEntry {
    SlotBrowserEntry {
        slot: SaveSlot::new(slot).expect("test slot should be in range"),
        meta: occupied.then(|| SaveMeta {
            scenario_id: "the_iron_siege".to_string(),
            wave: 2,
            in_game_seconds: 300.0,
            saved_at: 1.0,
            version: "0.1.0".to_string(),
        }),
        occupied,
        latest: false,
    }
}

fn browser(purpose: SlotBrowserPurpose) -> SlotBrowser {
    SlotBrowser {
        purpose,
        entries: vec![entry(1, true), entry(2, false)],
    }
}

/// The asymmetry that makes one browser serve both verbs: you may save into
/// an empty slot, and there is nothing to load from one.
#[test]
fn an_empty_slot_can_be_saved_into_but_not_loaded_from() {
    let saving = browser(SlotBrowserPurpose::Save);
    let loading = browser(SlotBrowserPurpose::Load);

    assert!(saving.is_selectable(&saving.entries[1]), "save into empty");
    assert!(
        !loading.is_selectable(&loading.entries[1]),
        "load from empty"
    );
}

/// An occupied slot is a target for both — saving over it is the player's
/// business, and refusing would make slot reuse impossible.
#[test]
fn an_occupied_slot_is_selectable_either_way() {
    for purpose in [SlotBrowserPurpose::Save, SlotBrowserPurpose::Load] {
        let browser = browser(purpose);
        assert!(browser.is_selectable(&browser.entries[0]), "{purpose:?}");
    }
}

/// A save from before `SaveMeta` existed has no header to describe it, but
/// it does load. Occupancy is what gates selection, not describability —
/// keying on `meta` would hide exactly the saves a player most wants back.
#[test]
fn a_headerless_save_is_still_loadable() {
    let loading = SlotBrowser {
        purpose: SlotBrowserPurpose::Load,
        entries: vec![SlotBrowserEntry {
            slot: SaveSlot::default(),
            meta: None,
            occupied: true,
            latest: false,
        }],
    };
    assert!(loading.is_selectable(&loading.entries[0]));
}
