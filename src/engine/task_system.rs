//! Task execution system
//! Handles creature task completion logic

use crate::data::GameData;
use crate::engine::creature_ai;
use crate::state::entities::{EntityId, EntityManager, Task};
use crate::state::player_state::PlayerState;
use crate::state::room_manager::RoomManager;
use crate::state::tile_state::TilePos;

/// Result of task execution that may require state updates
pub struct TaskResult {
    pub gold_change: f32,
    pub food_change: f32,
    pub materials_change: f32,
    pub research_change: f32,
    pub manufactured_trap: Option<String>,
    pub claimed_tile: Option<TilePos>,
    pub task_complete: bool,
}

impl Default for TaskResult {
    fn default() -> Self {
        Self {
            gold_change: 0.0,
            food_change: 0.0,
            materials_change: 0.0,
            research_change: 0.0,
            manufactured_trap: None,
            claimed_tile: None,
            task_complete: false,
        }
    }
}

/// Execute a creature's current task
/// Returns changes that need to be applied to game state
pub fn execute_task(
    creature_id: EntityId,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    player: &PlayerState,
    game_data: &GameData,
    dt: f32,
) -> TaskResult {
    let mut result = TaskResult::default();

    // Get the current task
    let task = {
        let entity = match entities.get(creature_id) {
            Some(e) => e,
            None => return result,
        };
        let creature = match entity.as_creature() {
            Some(c) => c,
            None => return result,
        };
        creature.current_task.clone()
    };

    let Some(task) = task else { return result };

    match &task {
        Task::Sleep(room_id) => {
            execute_sleep(creature_id, *room_id, entities, room_manager, game_data, dt);
        }
        Task::Eat(room_id) => {
            result.food_change = execute_eat(
                creature_id,
                *room_id,
                entities,
                room_manager,
                player,
                game_data,
                dt,
            );
        }
        Task::DepositGold(room_id) => {
            result.gold_change =
                execute_deposit_gold(creature_id, *room_id, entities, room_manager, game_data, dt);
            if result.gold_change > 0.0 {
                result.task_complete = true;
            }
        }
        Task::Train(room_id) => {
            execute_train(creature_id, *room_id, entities, room_manager, game_data, dt);
        }
        Task::Dig(_) => {
            // Do nothing here.
            // Digging is handled by imp_ai system which manages the task timer.
            // If we complete it here, the imp will stop digging instantly every frame.
        }
        Task::Work(room_id, _) => {
            let work = execute_work(
                creature_id,
                *room_id,
                entities,
                room_manager,
                player,
                game_data,
                dt,
            );
            result.materials_change = work.materials_change;
            result.manufactured_trap = work.manufactured_trap;
        }
        Task::Research(room_id) => {
            result.research_change =
                execute_research(creature_id, *room_id, entities, room_manager, game_data, dt);
        }
        Task::CollectWages(room_id) => {
            result.gold_change = execute_collect_wages(
                creature_id,
                *room_id,
                entities,
                room_manager,
                player,
                game_data,
                dt,
            );
        }
        _ => {}
    }

    // Mark task complete if needed
    if result.task_complete {
        if let Some(entity) = entities.get_mut(creature_id) {
            if let Some(creature) = entity.as_creature_mut() {
                creature.current_task = None;
            }
        }
    }

    result
}

/// Handle Sleep task
fn execute_sleep(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
    dt: f32,
) {
    // Still keyed on the lair specifically: the `sleep` task family also covers
    // the kennel, and widening it collides with `count_available_lair_tiles`
    // setting the creature cap. See TODO.md. The room's own
    // `sleep_recovery_rate` is honoured either way.
    let room_rate = room_manager
        .rooms
        .iter()
        .find(|r| r.id == room_id && r.room_type == "lair")
        .and_then(|room| crate::engine::room_validator::room_data_for(room, game_data))
        .map(|data| data.effects.sleep_recovery_rate);

    let Some(room_rate) = room_rate else {
        return;
    };

    let creature = match entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        Some(c) => c,
        None => return,
    };
    let sleep_rate = game_data.config.task_execution.sleep_satisfaction_rate * room_rate;
    creature_ai::satisfy_need(creature, "sleep", sleep_rate, dt);
}

/// Handle Eat task - returns food consumed (negative)
fn execute_eat(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    player: &PlayerState,
    game_data: &GameData,
    dt: f32,
) -> f32 {
    // Verify room is a hatchery
    if !room_manager
        .rooms
        .iter()
        .any(|r| r.id == room_id && r.room_type == "hatchery")
    {
        return 0.0;
    }

    if player.food <= 0 {
        return 0.0;
    }

    let food_rate = game_data.config.task_execution.food_consumption_rate;
    let food_multiplier = game_data.config.task_execution.food_satisfaction_multiplier;
    let food_consumed = (dt * food_rate).min(player.food as f32); // Keep precise float
    if let Some(creature) = entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        creature_ai::satisfy_need(creature, "food", food_consumed * food_multiplier, dt);
    }

    -food_consumed
}

/// Handle DepositGold task - returns gold deposited
fn execute_deposit_gold(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
    dt: f32,
) -> f32 {
    let is_treasury = room_manager
        .rooms
        .iter()
        .any(|r| r.id == room_id && r.room_type == "treasury");
    if !is_treasury {
        return 0.0;
    }

    let creature = match entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        Some(c) => c,
        None => return 0.0,
    };

    let gold = creature.gold_carried;
    creature.gold_carried = 0;
    let gold_satisfaction_rate = game_data
        .config
        .task_execution
        .gold_deposit_satisfaction_rate;
    creature_ai::satisfy_need(creature, "gold", gold_satisfaction_rate, dt);
    gold as f32
}

/// Handle Train task
fn execute_train(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
    dt: f32,
) {
    // Any room in the `train` task family, scaled by its own `training_rate` —
    // the same shape as `execute_research`, so a second training room is a
    // data edit rather than another branch here.
    let room_rate = room_manager
        .rooms
        .iter()
        .find(|r| r.id == room_id)
        .and_then(|room| crate::engine::room_validator::room_data_for(room, game_data))
        .filter(|data| data.ai.task_type == "train")
        .map(|data| data.effects.training_rate);

    let Some(room_rate) = room_rate else {
        return;
    };

    let creature = match entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        Some(c) => c,
        None => return,
    };

    let task_config = &game_data.config.task_execution;
    let combat_config = &game_data.config.combat;

    creature.training_timer += dt;
    if creature.training_timer < task_config.training_timer_threshold {
        return;
    }

    creature.training_timer = 0.0;
    creature.experience += task_config.xp_per_training * room_rate;

    if creature.level >= combat_config.max_creature_level {
        creature.experience = creature.max_experience;
        return;
    }

    if creature.experience >= creature.max_experience {
        creature.level += 1;
        creature.experience = 0.0;
        creature.max_experience *= task_config.level_up_exp_multiplier;
        creature.max_health *= task_config.level_up_health_multiplier;
        creature.health = creature.max_health;
        trace_log!(
            "tasks",
            "Creature {} leveled up to {}",
            creature.creature_id,
            creature.level
        );
    }
}

/// Handle Work task - returns materials produced
struct WorkResult {
    materials_change: f32,
    manufactured_trap: Option<String>,
}

fn execute_work(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    player: &PlayerState,
    game_data: &GameData,
    dt: f32,
) -> WorkResult {
    let room = match room_manager.rooms.iter().find(|r| {
        r.id == room_id && (r.room_type == "workshop" || r.room_type == "torture_chamber")
    }) {
        Some(r) => r,
        None => {
            return WorkResult {
                materials_change: 0.0,
                manufactured_trap: None,
            }
        }
    };

    let creature = match entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        Some(c) => c,
        None => {
            return WorkResult {
                materials_change: 0.0,
                manufactured_trap: None,
            }
        }
    };

    let efficiency = game_data
        .monsters
        .get(&creature.creature_id)
        .map(|m| creature_ai::calculate_work_efficiency(creature, m, game_data))
        .unwrap_or(1.0);

    let productivity = crate::engine::room_validator::room_data_for(room, game_data)
        .map(|data| crate::engine::room_validator::room_productivity_multiplier(room, data))
        .unwrap_or(1.0);
    creature.work_timer += dt * room.efficiency * productivity * efficiency;

    let work_threshold = game_data.config.task_execution.work_timer_threshold;
    if creature.work_timer >= work_threshold {
        creature.work_timer = 0.0;
        if room.room_type == "workshop" {
            let manufactured_trap = select_manufactured_trap(player, game_data);
            if let Some(trap_id) = &manufactured_trap {
                trace_log!(
                    "tasks",
                    "Creature {} manufactured {} crate.",
                    creature.creature_id,
                    trap_id
                );
            }
            return WorkResult {
                materials_change: 0.0,
                manufactured_trap,
            };
        }
    }

    WorkResult {
        materials_change: 0.0,
        manufactured_trap: None,
    }
}

fn select_manufactured_trap(player: &PlayerState, game_data: &GameData) -> Option<String> {
    let mut candidates: Vec<&crate::data::traps::TrapData> = player
        .unlocked_traps
        .iter()
        .filter_map(|id| game_data.traps.get(id))
        .collect();

    candidates.sort_by(|a, b| {
        player
            .trap_inventory_count(&a.id)
            .cmp(&player.trap_inventory_count(&b.id))
            .then_with(|| a.cost.cmp(&b.cost))
            .then_with(|| a.name.cmp(&b.name))
    });

    candidates.first().map(|trap| trap.id.clone())
}

/// Handle CollectWages task - returns gold consumed from player (negative)
fn execute_collect_wages(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    player: &PlayerState,
    game_data: &GameData,
    dt: f32,
) -> f32 {
    let is_treasury = room_manager
        .rooms
        .iter()
        .any(|r| r.id == room_id && r.room_type == "treasury");
    if !is_treasury {
        return 0.0;
    }

    let creature = match entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        Some(c) => c,
        None => return 0.0,
    };

    let monster_data = match game_data.monsters.get(&creature.creature_id) {
        Some(data) => data,
        None => return 0.0,
    };

    if player.gold <= 0 {
        // Treasury is empty: this creature goes unpaid this tick. There's nothing physical
        // to steal from an empty coffer, but a creature prone to theft when unpaid
        // (`economy.steals_if_unpaid`) resents it harder than a docile one, escalating its
        // "gold" need faster toward the desertion threshold instead of only decaying at the
        // same passive rate as a well-behaved creature would.
        let unpaid_rate = game_data.config.task_execution.wage_satisfaction_rate;
        let unrest_multiplier = if monster_data.economy.steals_if_unpaid {
            2.0
        } else {
            1.0
        };
        creature_ai::satisfy_need(creature, "gold", -unpaid_rate * unrest_multiplier, dt);
        return 0.0;
    }

    let wage_rate = game_data.config.task_execution.wage_satisfaction_rate;
    // Precise consumption
    let gold_consumed = (wage_rate * dt).min(player.gold as f32);

    creature_ai::satisfy_need(creature, "gold", gold_consumed, dt);
    -gold_consumed
}

/// Handle Research task - returns research points generated
fn execute_research(
    creature_id: EntityId,
    room_id: usize,
    entities: &mut EntityManager,
    room_manager: &RoomManager,
    game_data: &GameData,
    dt: f32,
) -> f32 {
    // Any room in the `research` task family, not just the library — see
    // `room_validator::find_nearest_room_for_task`. The room's own
    // `research_rate` scales the global rate, so a dedicated research room is
    // a data edit rather than a new branch here.
    let room_rate = room_manager
        .rooms
        .iter()
        .find(|r| r.id == room_id)
        .and_then(|room| crate::engine::room_validator::room_data_for(room, game_data))
        .filter(|data| data.ai.task_type == "research")
        .map(|data| data.effects.research_rate);

    let Some(room_rate) = room_rate else {
        return 0.0;
    };

    let creature = match entities
        .get_mut(creature_id)
        .and_then(|e| e.as_creature_mut())
    {
        Some(c) => c,
        None => return 0.0,
    };

    let efficiency = game_data
        .monsters
        .get(&creature.creature_id)
        .map(|m| creature_ai::calculate_work_efficiency(creature, m, game_data))
        .unwrap_or(1.0);

    let research_rate = game_data.config.task_execution.research_production_rate;
    let productivity = room_manager
        .rooms
        .iter()
        .find(|room| room.id == room_id)
        .and_then(|room| crate::engine::room_validator::room_data_for(room, game_data))
        .map(|data| {
            let room = room_manager
                .rooms
                .iter()
                .find(|room| room.id == room_id)
                .unwrap();
            crate::engine::room_validator::room_productivity_multiplier(room, data)
        })
        .unwrap_or(1.0);
    research_rate * room_rate * productivity * dt * efficiency
}
