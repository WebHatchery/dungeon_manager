use crate::data::GameData;
use crate::state::faction::OwnerId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    pub meta: ScenarioMeta,
    pub map: ScenarioMap,
    #[serde(default)]
    pub players: Vec<ScenarioPlayer>,
    #[serde(default)]
    pub rules: ScenarioRules,
    #[serde(default)]
    pub creature_pool: HashMap<String, u32>,
    #[serde(default)]
    pub availability: ScenarioAvailability,
    #[serde(default)]
    pub objectives: Vec<ScenarioObjective>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
    #[serde(default)]
    pub action_points: Vec<ActionPointDefinition>,
    #[serde(default)]
    pub hero_parties: Vec<HeroPartyDefinition>,
    #[serde(default)]
    pub rival_keepers: Vec<RivalKeeperDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Story lines shown as an intro overlay when the scenario starts
    #[serde(default)]
    pub intro: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMap {
    pub path: String,
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub height: Option<usize>,
    #[serde(default)]
    pub biome: Option<String>,
    #[serde(default)]
    pub above_ground: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPlayer {
    pub id: String,
    pub owner: OwnerId,
    pub role: ScenarioPlayerRole,
    #[serde(default)]
    pub start_gold: i32,
    #[serde(default)]
    pub start_mana: i32,
    #[serde(default)]
    pub max_creatures: Option<u32>,
    #[serde(default)]
    pub ai_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioPlayerRole {
    Human,
    RivalKeeper,
    Heroes,
    Neutral,
    AboveGround,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRules {
    #[serde(default = "default_true")]
    pub fog_of_war: bool,
    #[serde(default)]
    pub starting_research: Vec<String>,
    #[serde(default)]
    pub disabled_systems: Vec<String>,
    #[serde(default = "default_threat_multiplier")]
    pub threat_multiplier: f32,
}

impl Default for ScenarioRules {
    fn default() -> Self {
        Self {
            fog_of_war: true,
            starting_research: Vec::new(),
            disabled_systems: Vec::new(),
            threat_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioAvailability {
    #[serde(default)]
    pub rooms: HashMap<String, AvailabilityRule>,
    #[serde(default)]
    pub spells: HashMap<String, AvailabilityRule>,
    #[serde(default)]
    pub traps: HashMap<String, AvailabilityRule>,
    #[serde(default)]
    pub creatures: HashMap<String, AvailabilityRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityRule {
    #[serde(default = "default_true")]
    pub available: bool,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(default)]
    pub min_level: Option<u32>,
    #[serde(default)]
    pub owner: Option<OwnerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScenarioObjective {
    DestroyHeart { owner: OwnerId },
    SurviveTime { seconds: f32 },
    DestroyAllHeroBuildings,
    GatherResource { resource: String, amount: i32 },
    Custom { id: String, description: String },
}

impl ScenarioObjective {
    pub fn objective_id(&self) -> String {
        match self {
            ScenarioObjective::DestroyHeart { owner } => {
                format!("destroy_heart:{}", owner.label())
            }
            ScenarioObjective::SurviveTime { .. } => "survive_time".to_string(),
            ScenarioObjective::DestroyAllHeroBuildings => "destroy_all_hero_buildings".to_string(),
            ScenarioObjective::GatherResource { resource, amount } => {
                format!("gather_resource:{resource}:{amount}")
            }
            ScenarioObjective::Custom { id, .. } => id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioEvent {
    pub id: String,
    pub trigger: EventTrigger,
    #[serde(default)]
    pub actions: Vec<EventAction>,
    #[serde(default)]
    pub once: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventTrigger {
    TimeElapsed {
        seconds: f32,
    },
    ObjectiveComplete {
        objective: String,
    },
    RoomClaimed {
        room: String,
        owner: OwnerId,
    },
    ActionPointReached {
        id: String,
        owner: OwnerId,
    },
    DungeonBreached {
        owner: OwnerId,
    },
    /// Fires once every hero of this `hero` id (heroes faction) that has been
    /// seen alive is dead/gone — i.e. a named boss has been defeated. The
    /// "seen" gate means a boss that spawns mid-mission (a later wave) doesn't
    /// trigger before it ever appears.
    HeroDefeated {
        hero: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum EventAction {
    UnlockRoom {
        room: String,
        owner: OwnerId,
    },
    UnlockSpell {
        spell: String,
        owner: OwnerId,
    },
    UnlockTrap {
        trap: String,
        owner: OwnerId,
    },
    SpawnCreature {
        creature: String,
        owner: OwnerId,
        x: i32,
        y: i32,
        level: u32,
    },
    SpawnHeroParty {
        party: String,
        x: i32,
        y: i32,
    },
    SetRule {
        key: String,
        value: serde_json::Value,
    },
    CompleteObjective {
        objective: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPointDefinition {
    pub id: String,
    pub x: i32,
    pub y: i32,
    #[serde(default = "default_action_point_radius")]
    pub radius: i32,
    #[serde(default)]
    pub owner: Option<OwnerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroPartyDefinition {
    pub id: String,
    #[serde(default)]
    pub behavior: HeroPartyBehavior,
    #[serde(default)]
    pub members: Vec<HeroPartyMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum HeroPartyBehavior {
    #[default]
    AttackDungeonHeart,
    DefendLocation,
    StealGold,
    Explore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroPartyMember {
    pub hero: String,
    #[serde(default = "default_level")]
    pub level: u32,
    #[serde(default = "default_count")]
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivalKeeperDefinition {
    pub owner: OwnerId,
    pub ai_profile: String,
    #[serde(default)]
    pub starting_rooms: Vec<String>,
    #[serde(default)]
    pub preferred_creatures: Vec<String>,
    #[serde(default = "crate::state::rival_keeper::default_min_garrison")]
    pub min_garrison: usize,
    #[serde(default = "crate::state::rival_keeper::default_raid_size")]
    pub raid_size: usize,
    #[serde(default = "crate::state::rival_keeper::default_dig_batch")]
    pub dig_batch: usize,
    #[serde(default = "crate::state::rival_keeper::default_room_size")]
    pub room_size: usize,
    #[serde(default = "crate::state::rival_keeper::default_attack_cooldown")]
    pub attack_cooldown: f32,
    #[serde(default = "crate::state::rival_keeper::default_first_attack_delay")]
    pub first_attack_delay: f32,
    #[serde(default = "crate::state::rival_keeper::default_attack_cooldown_growth")]
    pub attack_cooldown_growth: f32,
}

impl ScenarioDefinition {
    pub fn validate_references(&self, game_data: &GameData) -> Vec<String> {
        let mut errors = Vec::new();

        for id in self.creature_pool.keys() {
            if !game_data.monsters.contains_key(id) {
                errors.push(format!(
                    "Scenario '{}' references unknown creature '{}'",
                    self.meta.id, id
                ));
            }
        }

        check_keys(
            &mut errors,
            &self.meta.id,
            "room",
            self.availability.rooms.keys(),
            game_data.rooms.keys(),
        );
        check_keys(
            &mut errors,
            &self.meta.id,
            "spell",
            self.availability.spells.keys(),
            game_data.spells.keys(),
        );
        check_keys(
            &mut errors,
            &self.meta.id,
            "trap",
            self.availability.traps.keys(),
            game_data.traps.keys(),
        );
        check_keys(
            &mut errors,
            &self.meta.id,
            "creature",
            self.availability.creatures.keys(),
            game_data.monsters.keys(),
        );

        for party in &self.hero_parties {
            for member in &party.members {
                if !game_data.heroes.contains_key(&member.hero) {
                    errors.push(format!(
                        "Scenario '{}' party '{}' references unknown hero '{}'",
                        self.meta.id, party.id, member.hero
                    ));
                }
            }
        }

        for keeper in &self.rival_keepers {
            check_keys(
                &mut errors,
                &self.meta.id,
                "rival keeper room",
                keeper.starting_rooms.iter(),
                game_data.rooms.keys(),
            );
            check_keys(
                &mut errors,
                &self.meta.id,
                "rival keeper creature",
                keeper.preferred_creatures.iter(),
                game_data.monsters.keys(),
            );
        }

        errors
    }
}

/// Load every base-game scenario. Sourced from the build-time embedded manifest
/// (`data::embedded::EMBEDDED_SCENARIOS`), with a native runtime directory scan
/// of `assets/scenarios/` overlaid so a dropped-in mission loads without a
/// rebuild. Adding a scenario is pure content work — no code changes.
pub fn load_scenarios() -> Result<HashMap<String, ScenarioDefinition>, Box<dyn Error>> {
    let mut scenarios = crate::data::content_source::from_embedded(
        crate::data::embedded::EMBEDDED_SCENARIOS,
        |scenario: &ScenarioDefinition| scenario.meta.id.clone(),
    )?;
    #[cfg(not(target_arch = "wasm32"))]
    crate::data::content_source::overlay_from_disk(
        &mut scenarios,
        "assets/scenarios",
        |scenario: &ScenarioDefinition| scenario.meta.id.clone(),
    );
    Ok(scenarios)
}

fn check_keys<'a>(
    errors: &mut Vec<String>,
    scenario_id: &str,
    kind: &str,
    actual: impl Iterator<Item = &'a String>,
    known: impl Iterator<Item = &'a String>,
) {
    let known: HashSet<&String> = known.collect();
    for id in actual {
        if !known.contains(id) {
            errors.push(format!(
                "Scenario '{scenario_id}' references unknown {kind} '{id}'"
            ));
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_threat_multiplier() -> f32 {
    1.0
}

fn default_level() -> u32 {
    1
}

fn default_count() -> u32 {
    1
}

fn default_action_point_radius() -> i32 {
    1
}

#[cfg(test)]
mod tests;
