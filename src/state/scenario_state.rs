use crate::data::scenario::{AvailabilityRule, ScenarioDefinition, ScenarioRules};
use crate::state::OwnerId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRuntimeState {
    pub scenario_id: String,
    #[serde(default)]
    pub fired_events: HashSet<String>,
    #[serde(default)]
    pub completed_objectives: HashSet<String>,
    #[serde(default)]
    pub available_rooms: HashSet<String>,
    #[serde(default)]
    pub unlocked_rooms: HashSet<String>,
    #[serde(default)]
    pub available_spells: HashSet<String>,
    #[serde(default)]
    pub unlocked_spells: HashSet<String>,
    #[serde(default)]
    pub available_traps: HashSet<String>,
    #[serde(default)]
    pub unlocked_traps: HashSet<String>,
    #[serde(default)]
    pub available_creatures: HashSet<String>,
    #[serde(default)]
    pub unlocked_creatures: HashSet<String>,
    #[serde(default)]
    pub reached_action_points: HashSet<String>,
    /// Hero ids (heroes faction) that have been seen alive at least once; used
    /// by the `HeroDefeated` trigger so a boss is only "defeated" after it has
    /// actually appeared.
    #[serde(default)]
    pub seen_hero_ids: HashSet<String>,
    #[serde(default)]
    pub active_rules: ScenarioRules,
}

impl ScenarioRuntimeState {
    pub fn from_definition(scenario: &ScenarioDefinition) -> Self {
        let (available_rooms, unlocked_rooms) = collect_availability(&scenario.availability.rooms);
        let (available_spells, unlocked_spells) =
            collect_availability(&scenario.availability.spells);
        let (available_traps, unlocked_traps) = collect_availability(&scenario.availability.traps);
        let (available_creatures, unlocked_creatures) =
            collect_availability(&scenario.availability.creatures);

        Self {
            scenario_id: scenario.meta.id.clone(),
            fired_events: HashSet::new(),
            completed_objectives: HashSet::new(),
            available_rooms,
            unlocked_rooms,
            available_spells,
            unlocked_spells,
            available_traps,
            unlocked_traps,
            available_creatures,
            unlocked_creatures,
            reached_action_points: HashSet::new(),
            seen_hero_ids: HashSet::new(),
            active_rules: scenario.rules.clone(),
        }
    }

    pub fn mark_event_fired(&mut self, event_id: &str) {
        self.fired_events.insert(event_id.to_string());
    }

    pub fn mark_objective_complete(&mut self, objective_id: &str) {
        self.completed_objectives.insert(objective_id.to_string());
    }

    pub fn unlock_room(&mut self, room_id: &str) {
        if self.available_rooms.contains(room_id) {
            self.unlocked_rooms.insert(room_id.to_string());
        }
    }

    pub fn unlock_spell(&mut self, spell_id: &str) {
        if self.available_spells.contains(spell_id) {
            self.unlocked_spells.insert(spell_id.to_string());
        }
    }

    pub fn unlock_trap(&mut self, trap_id: &str) {
        if self.available_traps.contains(trap_id) {
            self.unlocked_traps.insert(trap_id.to_string());
        }
    }

    pub fn mark_action_point_reached(&mut self, point_id: &str, owner: &OwnerId) {
        self.reached_action_points
            .insert(action_point_key(point_id, owner));
    }

    pub fn action_point_reached(&self, point_id: &str, owner: &OwnerId) -> bool {
        self.reached_action_points
            .contains(&action_point_key(point_id, owner))
    }

    pub fn set_rule(&mut self, key: &str, value: &serde_json::Value) -> bool {
        match key {
            "fog_of_war" => value.as_bool().map(|enabled| {
                self.active_rules.fog_of_war = enabled;
            }),
            "threat_multiplier" => value.as_f64().map(|multiplier| {
                self.active_rules.threat_multiplier = (multiplier as f32).max(0.0);
            }),
            "disabled_systems" => string_array(value).map(|systems| {
                self.active_rules.disabled_systems = systems;
            }),
            "starting_research" => string_array(value).map(|research| {
                self.active_rules.starting_research = research;
            }),
            _ => None,
        }
        .is_some()
    }
}

fn action_point_key(point_id: &str, owner: &OwnerId) -> String {
    format!("{}:{point_id}", owner.label())
}

fn string_array(value: &serde_json::Value) -> Option<Vec<String>> {
    let values = value.as_array()?;
    values
        .iter()
        .map(|item| item.as_str().map(ToOwned::to_owned))
        .collect()
}

fn collect_availability(
    rules: &HashMap<String, AvailabilityRule>,
) -> (HashSet<String>, HashSet<String>) {
    let mut available = HashSet::new();
    let mut unlocked = HashSet::new();

    for (id, rule) in rules {
        if rule.available {
            available.insert(id.clone());
            if rule.unlocked {
                unlocked.insert(id.clone());
            }
        }
    }

    (available, unlocked)
}

#[cfg(test)]
mod tests;
