use crate::data::scenario::ScenarioObjective;
use crate::data::GameData;
use crate::state::entities::EntityType;
use crate::state::faction::OwnerId;
use crate::state::game_state::GameState;

pub fn update_victory_and_defeat(state: &mut GameState, game_data: &GameData) {
    if state.game_over {
        return;
    }

    if state.dungeon_heart_health <= 0.0 {
        set_defeat(state, "DEFEAT! Your Dungeon Heart was destroyed!");
        return;
    }

    if update_scenario_objectives(state, game_data) {
        set_victory(state, "VICTORY! Scenario objectives completed!");
        complete_active_campaign_mission(state, game_data);
        return;
    }

    if state.hero_base.enabled && state.hero_base.is_defeated(&state.entities) {
        set_victory(state, "VICTORY! All hero buildings have been destroyed!");
        complete_active_campaign_mission(state, game_data);
    }
}

fn update_scenario_objectives(state: &mut GameState, game_data: &GameData) -> bool {
    let Some(scenario_id) = &state.active_scenario_id else {
        return false;
    };
    let Some(scenario) = game_data.scenarios.get(scenario_id) else {
        return false;
    };
    if scenario.objectives.is_empty() {
        return false;
    }

    let completed_ids: Vec<String> = scenario
        .objectives
        .iter()
        .filter(|objective| objective_is_complete(objective, state, game_data))
        .map(ScenarioObjective::objective_id)
        .collect();

    if let Some(runtime) = &mut state.scenario_runtime {
        for id in completed_ids {
            runtime.mark_objective_complete(&id);
        }

        scenario.objectives.iter().all(|objective| {
            runtime
                .completed_objectives
                .contains(&objective.objective_id())
        })
    } else {
        false
    }
}

fn objective_is_complete(
    objective: &ScenarioObjective,
    state: &GameState,
    game_data: &GameData,
) -> bool {
    match objective {
        ScenarioObjective::DestroyHeart { owner } => owner_heart_destroyed(owner, state),
        ScenarioObjective::SurviveTime { seconds } => state.time_elapsed >= *seconds,
        ScenarioObjective::DestroyAllHeroBuildings => hero_buildings_destroyed(state, game_data),
        ScenarioObjective::GatherResource { resource, amount } => {
            current_resource_amount(state, resource) >= *amount
        }
        ScenarioObjective::Custom { id, .. } => state
            .scenario_runtime
            .as_ref()
            .map(|runtime| runtime.completed_objectives.contains(id))
            .unwrap_or(false),
    }
}

fn owner_heart_destroyed(owner: &OwnerId, state: &GameState) -> bool {
    if owner.is_player_controlled() {
        return state.dungeon_heart_health <= 0.0;
    }

    let matching_hearts = state.entities.all().filter(|entity| {
        entity.owner == *owner
            && matches!(
                &entity.entity_type,
                EntityType::Structure(structure) if structure.building_id == "dungeon_heart"
            )
    });

    let mut saw_heart = false;
    for entity in matching_hearts {
        saw_heart = true;
        if entity.is_alive() {
            return false;
        }
    }

    saw_heart
}

fn hero_buildings_destroyed(state: &GameState, game_data: &GameData) -> bool {
    if state.hero_base.enabled {
        return state.hero_base.is_defeated(&state.entities);
    }

    let live_hero_buildings = state.entities.all().any(|entity| {
        entity.owner == OwnerId::Heroes
            && matches!(
                &entity.entity_type,
                EntityType::Structure(structure)
                    if game_data.hero_buildings.contains_key(&structure.building_id)
                        && structure.health > 0.0
            )
    });

    !live_hero_buildings && has_authored_hero_building_tiles(state, game_data)
}

fn has_authored_hero_building_tiles(state: &GameState, game_data: &GameData) -> bool {
    state.dungeon.grid.iter().flatten().any(|tile| {
        game_data.hero_buildings.contains_key(&tile.tile_type) && tile.owner == OwnerId::Heroes
    })
}

fn current_resource_amount(state: &GameState, resource: &str) -> i32 {
    match resource {
        "gold" => state.player.gold,
        "mana" => state.player.mana,
        "food" => state.player.food,
        "materials" => state.player.materials,
        _ => 0,
    }
}

fn set_victory(state: &mut GameState, message: &str) {
    state.game_over = true;
    state.victory = true;
    state.notifications.success(message);
}

fn set_defeat(state: &mut GameState, message: &str) {
    state.game_over = true;
    state.victory = false;
    state.notifications.danger(message);
}

fn complete_active_campaign_mission(state: &mut GameState, game_data: &GameData) {
    let Some(progress) = &mut state.campaign_progress else {
        return;
    };
    let Some(campaign) = game_data.campaigns.get(&progress.campaign_id) else {
        return;
    };

    let mission_id = progress.active_mission.clone();
    progress.complete_mission(campaign, &mission_id);
}

#[cfg(test)]
mod tests;
