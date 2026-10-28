//! Campaign progress that outlives a play session.
//!
//! Mission-select used to rebuild a fresh `CampaignProgress` every time it
//! opened, so completed missions only survived inside a save slot. This keeps
//! the latest progress per campaign under its own persistence key, using the
//! same native-file / WASM-localStorage mechanism as settings.

use crate::data::campaign::{CampaignDefinition, CampaignProgress};

const GAME_NAME: &str = "dungeon_manager";

fn key(campaign_id: &str) -> String {
    format!("campaign_progress_{campaign_id}")
}

/// Stored progress for `campaign`, or a fresh start when nothing usable is
/// stored. Progress naming missions the campaign no longer has is discarded.
pub fn load_or_new(campaign: &CampaignDefinition) -> CampaignProgress {
    #[cfg(not(test))]
    {
        let stored: Result<CampaignProgress, String> =
            macroquad_toolkit::persistence::load_json_key(GAME_NAME, &key(&campaign.id));
        if let Some(progress) = stored.ok().filter(|progress| is_usable(progress, campaign)) {
            return progress;
        }
    }
    CampaignProgress::new(campaign)
}

pub fn save(progress: &CampaignProgress) {
    // Unit tests complete missions constantly; they must not overwrite a
    // player's real progress file.
    #[cfg(not(test))]
    if let Err(error) = macroquad_toolkit::persistence::save_json_key(
        GAME_NAME,
        &key(&progress.campaign_id),
        progress,
    ) {
        eprintln!("Failed to save campaign progress: {error}");
    }
    #[cfg(test)]
    let _ = progress;
}

#[cfg(not(test))]
fn is_usable(progress: &CampaignProgress, campaign: &CampaignDefinition) -> bool {
    let known = |id: &String| campaign.missions.iter().any(|mission| &mission.id == id);
    progress.campaign_id == campaign.id
        && known(&progress.active_mission)
        && progress.completed_missions.iter().all(known)
        && progress.unlocked_missions.iter().all(known)
}

/// Seconds a first reset click stays armed waiting for its confirming click.
const RESET_CONFIRM_SECONDS: f64 = 4.0;

thread_local! {
    static RESET_ARMED_UNTIL: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

fn now() -> f64 {
    macroquad::time::get_time()
}

pub fn reset_is_armed() -> bool {
    RESET_ARMED_UNTIL.with(|until| now() < until.get())
}

/// Register a click on the reset button. The first click arms it and returns
/// false; a second click inside the window returns true.
pub fn confirm_reset() -> bool {
    if reset_is_armed() {
        RESET_ARMED_UNTIL.with(|until| until.set(0.0));
        return true;
    }
    RESET_ARMED_UNTIL.with(|until| until.set(now() + RESET_CONFIRM_SECONDS));
    false
}

/// Wipe stored progress and return a fresh campaign start.
pub fn reset(campaign: &CampaignDefinition) -> CampaignProgress {
    let fresh = CampaignProgress::new(campaign);
    save(&fresh);
    fresh
}
