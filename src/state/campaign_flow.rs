use crate::data::campaign::{CampaignMission, CampaignProgress};
use crate::data::GameData;
use crate::state::game_state::GameState;

impl GameState {
    pub fn active_campaign_mission<'a>(
        &self,
        game_data: &'a GameData,
    ) -> Option<&'a CampaignMission> {
        let progress = self.campaign_progress.as_ref()?;
        let campaign = game_data.campaigns.get(&progress.campaign_id)?;
        progress.active_mission(campaign)
    }

    pub fn active_campaign_briefing<'a>(&self, game_data: &'a GameData) -> Option<&'a str> {
        self.active_campaign_mission(game_data)
            .map(|mission| mission.briefing.as_str())
            .filter(|briefing| !briefing.is_empty())
    }

    pub fn has_pending_campaign_mission(&self, game_data: &GameData) -> bool {
        let Some(progress) = &self.campaign_progress else {
            return false;
        };
        let Some(campaign) = game_data.campaigns.get(&progress.campaign_id) else {
            return false;
        };
        let Some(mission) = progress.active_mission(campaign) else {
            return false;
        };

        progress.unlocked_missions.contains(&mission.id)
            && !progress.completed_missions.contains(&mission.id)
            && mission
                .required_completed
                .iter()
                .all(|id| progress.completed_missions.contains(id))
    }

    pub fn new_for_campaign_progress(
        game_data: &GameData,
        progress: CampaignProgress,
    ) -> Option<Self> {
        let campaign = game_data.campaigns.get(&progress.campaign_id)?;
        let mission = progress.active_mission(campaign)?;

        let mut state = Self::new_for_scenario(game_data, &mission.scenario_id);
        state.apply_campaign_unlocks(&progress.persistent_unlocks);
        state.campaign_progress = Some(progress);
        Some(state)
    }

    pub fn start_pending_campaign_mission(&self, game_data: &GameData) -> Option<Self> {
        if !self.has_pending_campaign_mission(game_data) {
            return None;
        }

        let mut next =
            Self::new_for_campaign_progress(game_data, self.campaign_progress.as_ref()?.clone())?;
        // Keep the player's chosen difficulty across the whole campaign.
        next.difficulty = self.difficulty;
        Some(next)
    }
}

#[cfg(test)]
mod tests;
