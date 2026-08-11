use crate::data::monsters::MonsterData;
use crate::data::GameData;
use crate::state::entities::{CreatureState, Task};
use crate::state::room_manager::RoomManager;
use crate::state::tile_state::TilePos;

/// Try to satisfy a critical need by finding an appropriate room
pub fn try_satisfy_critical_need(
    creature: &CreatureState,
    creature_pos: TilePos,
    room_manager: &RoomManager,
    monster_data: &MonsterData,
    game_data: &GameData,
) -> Option<Task> {
    let (need_name, need_value) = creature.get_most_urgent_need()?;

    if need_value >= game_data.config.creature_ai.need_critical_threshold {
        return None;
    }

    let need_data = monster_data.needs.get(&need_name)?;

    if let Some(room_type) = need_data.satisfied_by.first() {
        use crate::engine::room_validator;
        let (room_id, _) =
            room_validator::find_nearest_room(&room_manager.rooms, room_type, creature_pos, 0.0)?;

        return match need_name.as_str() {
            "sleep" => Some(Task::Sleep(room_id)),
            "food" => Some(Task::Eat(room_id)),
            "gold" => Some(Task::CollectWages(room_id)),
            _ => None,
        };
    }

    None
}

/// Calculate desirability of a task for a creature
pub fn calculate_task_desirability(
    task: &Task,
    creature: &CreatureState,
    monster_data: &MonsterData,
    game_data: &GameData,
) -> f32 {
    let task_type = task.task_type();
    let task_config = &game_data.config.creature_ai.task_desirability;
    let mut desirability = task_config.base;

    // Apply task preference from monster data
    if let Some(&preference) = monster_data.ai.task_preferences.get(task_type) {
        desirability *= preference;
    }

    // Apply trait-driven task preference multipliers (data-driven; see traits.json)
    let trait_multiplier: f32 = monster_data
        .traits
        .iter()
        .filter_map(|trait_id| game_data.traits.get(trait_id))
        .filter_map(|t| t.task_preference_multipliers.get(task_type))
        .product();
    desirability *= trait_multiplier;

    // Boost desirability based on related needs
    match task {
        Task::Sleep(_) => {
            let sleep_need = 100.0 - creature.get_need("sleep");
            desirability *= 1.0 + (sleep_need / 100.0);
        }
        Task::Eat(_) => {
            let food_need = 100.0 - creature.get_need("food");
            desirability *= 1.0 + (food_need / 100.0);
        }
        Task::DepositGold(_) => {
            if creature.gold_carried > 0 {
                let gold_need = 100.0 - creature.get_need("gold");
                desirability *= task_config.gold_deposit + (gold_need / 100.0);
            } else {
                desirability *= task_config.skip_deposit; // Don't deposit if no gold
            }
        }
        Task::Train(_) => {
            // Training is more desirable when satisfied (not urgent need)
            let avg_satisfaction = if !creature.needs.is_empty() {
                creature.needs.values().sum::<f32>() / creature.needs.len() as f32
            } else {
                50.0
            };
            if avg_satisfaction > task_config.satisfaction_threshold {
                desirability *= task_config.training_high_satisfaction;
            } else {
                desirability *= task_config.training_low_satisfaction;
            }
        }
        Task::CollectWages(_) => {
            let gold_need = 100.0 - creature.get_need("gold");
            desirability *= task_config.wage_collection + (gold_need / 100.0);
        }
        _ => {}
    }

    desirability
}

/// Check if creature should desert the dungeon
pub fn should_desert(creature: &CreatureState, monster_data: &MonsterData) -> bool {
    let desertion_threshold = monster_data.ai.desertion_threshold;
    creature.mood < desertion_threshold
}

#[cfg(test)]
mod tests;
