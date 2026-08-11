use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub starting_mission: String,
    #[serde(default)]
    pub missions: Vec<CampaignMission>,
    #[serde(default)]
    pub persistent_unlocks: CampaignUnlocks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMission {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
    #[serde(default)]
    pub briefing: String,
    #[serde(default)]
    pub unlocks_after: Vec<String>,
    #[serde(default)]
    pub required_completed: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignUnlocks {
    #[serde(default)]
    pub rooms: Vec<String>,
    #[serde(default)]
    pub spells: Vec<String>,
    #[serde(default)]
    pub traps: Vec<String>,
    #[serde(default)]
    pub creatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignProgress {
    pub campaign_id: String,
    pub active_mission: String,
    pub completed_missions: HashSet<String>,
    pub unlocked_missions: HashSet<String>,
    pub persistent_unlocks: CampaignUnlocks,
}

impl CampaignProgress {
    pub fn new(campaign: &CampaignDefinition) -> Self {
        let active_mission = if campaign.starting_mission.is_empty() {
            campaign
                .missions
                .first()
                .map(|mission| mission.id.clone())
                .unwrap_or_default()
        } else {
            campaign.starting_mission.clone()
        };

        let mut unlocked_missions = HashSet::new();
        if !active_mission.is_empty() {
            unlocked_missions.insert(active_mission.clone());
        }

        Self {
            campaign_id: campaign.id.clone(),
            active_mission,
            completed_missions: HashSet::new(),
            unlocked_missions,
            persistent_unlocks: campaign.persistent_unlocks.clone(),
        }
    }

    pub fn complete_mission(&mut self, campaign: &CampaignDefinition, mission_id: &str) {
        self.completed_missions.insert(mission_id.to_string());

        if let Some(mission) = campaign.missions.iter().find(|m| m.id == mission_id) {
            for unlock in &mission.unlocks_after {
                self.unlocked_missions.insert(unlock.clone());
            }
        }

        if let Some(next) = campaign
            .missions
            .iter()
            .find(|mission| {
                self.unlocked_missions.contains(&mission.id)
                    && !self.completed_missions.contains(&mission.id)
                    && mission
                        .required_completed
                        .iter()
                        .all(|id| self.completed_missions.contains(id))
            })
            .map(|mission| mission.id.clone())
        {
            self.active_mission = next;
        }
    }

    pub fn active_mission<'a>(
        &self,
        campaign: &'a CampaignDefinition,
    ) -> Option<&'a CampaignMission> {
        campaign
            .missions
            .iter()
            .find(|mission| mission.id == self.active_mission)
    }

    pub fn unlocked_missions<'a>(
        &self,
        campaign: &'a CampaignDefinition,
    ) -> Vec<&'a CampaignMission> {
        campaign
            .missions
            .iter()
            .filter(|mission| self.unlocked_missions.contains(&mission.id))
            .collect()
    }

    /// Status of a mission for the mission-select screen.
    fn mission_status(&self, mission: &CampaignMission) -> MissionStatus {
        if self.completed_missions.contains(&mission.id) {
            MissionStatus::Completed
        } else if self.unlocked_missions.contains(&mission.id)
            && mission
                .required_completed
                .iter()
                .all(|id| self.completed_missions.contains(id))
        {
            MissionStatus::Available
        } else {
            MissionStatus::Locked
        }
    }

    /// The full mission list for the campaign-map / mission-select UI: every
    /// mission in authored order, tagged with its per-player status.
    pub fn mission_menu(&self, campaign: &CampaignDefinition) -> Vec<MissionMenuEntry> {
        campaign
            .missions
            .iter()
            .map(|mission| MissionMenuEntry {
                id: mission.id.clone(),
                name: mission.name.clone(),
                scenario_id: mission.scenario_id.clone(),
                briefing: mission.briefing.clone(),
                status: self.mission_status(mission),
            })
            .collect()
    }

    /// Choose a mission to play from the mission-select screen. Only a mission
    /// the player can actually start (Available, or a Completed one being
    /// replayed) may be selected; a Locked mission is rejected. On success the
    /// mission becomes `active_mission`. Returns whether the selection took.
    pub fn select_mission(&mut self, campaign: &CampaignDefinition, mission_id: &str) -> bool {
        let Some(mission) = campaign.missions.iter().find(|m| m.id == mission_id) else {
            return false;
        };
        if matches!(self.mission_status(mission), MissionStatus::Locked) {
            return false;
        }
        self.active_mission = mission_id.to_string();
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionStatus {
    /// Beaten already — replayable.
    Completed,
    /// Unlocked and its prerequisites are met — playable now.
    Available,
    /// Not yet reachable.
    Locked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionMenuEntry {
    pub id: String,
    pub name: String,
    pub scenario_id: String,
    pub briefing: String,
    pub status: MissionStatus,
}

/// Load every base-game campaign. Sourced from the build-time embedded manifest
/// (`data::embedded::EMBEDDED_CAMPAIGNS`), with a native runtime directory scan
/// of `assets/campaigns/` overlaid so a dropped-in campaign loads without a
/// rebuild. Adding a campaign is pure content work — no code changes.
pub fn load_campaigns() -> Result<HashMap<String, CampaignDefinition>, Box<dyn Error>> {
    let mut campaigns = crate::data::content_source::from_embedded(
        crate::data::embedded::EMBEDDED_CAMPAIGNS,
        |campaign: &CampaignDefinition| campaign.id.clone(),
    )?;
    #[cfg(not(target_arch = "wasm32"))]
    crate::data::content_source::overlay_from_disk(
        &mut campaigns,
        "assets/campaigns",
        |campaign: &CampaignDefinition| campaign.id.clone(),
    );
    Ok(campaigns)
}

#[cfg(test)]
mod tests;
