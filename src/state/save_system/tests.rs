use super::*;
use crate::data::GameData;

#[test]
fn campaign_progress_survives_save_serialization() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_campaign_start(&game_data, "deep_dominion");
    let progress = state
        .campaign_progress
        .as_mut()
        .expect("campaign progress should exist");
    progress
        .completed_missions
        .insert("dark_beginnings".to_string());

    let file = SaveFile {
        meta: SaveMeta::describe(&state),
        game_state: &state,
    };
    let json = serde_json::to_string(&file).expect("save should serialize");
    let loaded: LoadFile = serde_json::from_str(&json).expect("save should deserialize");

    let loaded_progress = loaded
        .game_state
        .campaign_progress
        .expect("campaign progress should load");
    assert_eq!(loaded_progress.campaign_id, "deep_dominion");
    assert!(loaded_progress
        .completed_missions
        .contains("dark_beginnings"));
}

/// A slot outside the range cannot be built, so no call site can invent a
/// filename the picker will never list.
#[test]
fn only_real_slots_exist() {
    assert!(SaveSlot::new(0).is_none());
    assert!(SaveSlot::new(SLOT_COUNT + 1).is_none());
    assert_eq!(SaveSlot::all().count(), SLOT_COUNT as usize);
    for number in 1..=SLOT_COUNT {
        assert_eq!(SaveSlot::new(number), Some(SaveSlot::Numbered(number)));
    }
}

/// The autosave is a slot the player never picks into, so it is absent from
/// `all()` and present in `all_loadable()`. Getting this backwards would
/// either hide autosaves from the load list or let a manual save land in the
/// slot the timer is about to overwrite.
#[test]
fn the_autosave_can_be_loaded_but_not_chosen() {
    assert!(!SaveSlot::all().any(|slot| slot.is_auto()));
    assert!(SaveSlot::all_loadable().any(|slot| slot.is_auto()));
    assert_eq!(
        SaveSlot::all_loadable().count(),
        SLOT_COUNT as usize + 1,
        "loadable is the numbered slots plus the autosave"
    );
}

/// The autosave must not collide with any numbered slot's file.
#[test]
fn the_autosave_has_its_own_file() {
    let numbered: Vec<String> = SaveSlot::all().map(|slot| slot.key()).collect();
    assert!(
        !numbered.contains(&SaveSlot::Auto.key()),
        "autosave shares a file with a numbered slot: {numbered:?}"
    );
}

/// Each slot must file under its own name. The whole point of the change is
/// that a second save stops overwriting the first.
#[test]
fn every_slot_has_its_own_key() {
    let keys: std::collections::HashSet<String> = SaveSlot::all().map(|slot| slot.key()).collect();
    assert_eq!(
        keys.len(),
        SLOT_COUNT as usize,
        "slots share a key: {keys:?}"
    );
}

/// The default is slot 1 — the slot the old single-save build used — so a
/// player upgrading lands on their existing save rather than an empty one.
#[test]
fn the_default_slot_is_the_one_the_old_build_wrote() {
    assert_eq!(SaveSlot::default().key(), "slot_1");
}

/// The header has to describe a save well enough to tell two apart without
/// loading either, and it has to survive the round trip.
#[test]
fn the_header_describes_the_save_without_loading_it() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "the_iron_siege");
    state.time_elapsed = 421.5;
    state.hero_base.current_wave_number = 3;

    let json = serde_json::to_string(&SaveFile {
        meta: SaveMeta::describe(&state),
        game_state: &state,
    })
    .expect("save should serialize");

    let peeked: MetaOnly = serde_json::from_str(&json).expect("header should read alone");
    assert_eq!(peeked.meta.scenario_id, "the_iron_siege");
    assert_eq!(peeked.meta.wave, 3);
    assert!((peeked.meta.in_game_seconds - 421.5).abs() < 0.01);
    assert!(
        peeked.meta.saved_at > 0.0,
        "saved_at should be a real clock reading, got {}",
        peeked.meta.saved_at
    );
}

/// The autosave must not fire while the game is not running. Each clause is
/// checked on its own so a future edit cannot drop one silently.
#[test]
fn the_autosave_timer_only_runs_while_the_game_does() {
    let game_data = GameData::load().expect("game data should load");
    let mut state = GameState::new_for_scenario(&game_data, "the_iron_siege");
    state.tutorial.intro_dismissed = true;
    assert!(should_autosave(&state), "a running game should autosave");

    state.paused = true;
    assert!(!should_autosave(&state), "paused");
    state.paused = false;

    state.game_over = true;
    assert!(!should_autosave(&state), "game over");
    state.game_over = false;

    state.tutorial.intro_dismissed = false;
    assert!(!should_autosave(&state), "briefing unread");
}

/// A save the old build wrote has no `meta`. It must still load — that is
/// the migration — so the legacy shape is checked against the current one.
#[test]
fn a_headerless_save_still_loads() {
    let game_data = GameData::load().expect("game data should load");
    let state = GameState::new_for_scenario(&game_data, "the_iron_siege");

    // Exactly what the pre-slot build wrote: game_state, and no meta.
    let legacy_json = serde_json::json!({
        "game_state": &state,
        "save_date": "Unknown Date",
        "version": "0.1.0",
    })
    .to_string();

    let legacy: LegacySave =
        serde_json::from_str(&legacy_json).expect("legacy save should still deserialize");
    assert_eq!(legacy.game_state.time_elapsed, state.time_elapsed);
}
