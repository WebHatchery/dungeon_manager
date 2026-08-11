use crate::data::scenario::{HeroPartyBehavior, HeroPartyDefinition};
use crate::data::GameData;
use crate::state::entities::{EntityId, HeroGoal, HeroState};
use crate::state::game_state::GameState;
use crate::state::tile_state::TilePos;
use crate::state::OwnerId;

pub fn spawn_hero_party(
    state: &mut GameState,
    party: &HeroPartyDefinition,
    origin: TilePos,
    game_data: &GameData,
) -> Vec<EntityId> {
    let mut spawned = Vec::new();
    let mut offset_index = 0usize;

    for member in &party.members {
        let Some(hero_data) = game_data.heroes.get(&member.hero) else {
            continue;
        };

        for _ in 0..member.count {
            let pos = offset_position(origin, offset_index);
            offset_index += 1;

            let visual_seed = macroquad_toolkit::rng::random_u64();
            let mut hero_state = HeroState::new(
                member.hero.clone(),
                member.level,
                hero_data.stats.health,
                hero_data.stats.mana,
                pos,
                hero_data.stats.dig_time,
                visual_seed,
            );

            apply_party_behavior(&mut hero_state, &party.behavior, origin);
            let id = state
                .entities
                .spawn_hero_for_owner(pos, hero_state, OwnerId::Heroes);
            spawned.push(id);
        }
    }

    spawned
}

fn apply_party_behavior(hero_state: &mut HeroState, behavior: &HeroPartyBehavior, origin: TilePos) {
    match behavior {
        HeroPartyBehavior::AttackDungeonHeart => {
            hero_state.current_goal = HeroGoal::DestroyHeart;
            hero_state.target_pos = None;
        }
        HeroPartyBehavior::DefendLocation => {
            hero_state.is_defender = true;
            hero_state.current_goal = HeroGoal::RestAtSpawn(origin);
            hero_state.target_pos = Some(origin);
        }
        HeroPartyBehavior::StealGold => {
            hero_state.current_goal = HeroGoal::StealGold(500);
            hero_state.target_pos = None;
        }
        HeroPartyBehavior::Explore => {
            hero_state.current_goal = HeroGoal::Explore;
            hero_state.target_pos = None;
        }
    }
}

fn offset_position(origin: TilePos, index: usize) -> TilePos {
    const OFFSETS: [(i32, i32); 9] = [
        (0, 0),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ];

    let (dx, dy) = OFFSETS[index % OFFSETS.len()];
    TilePos::new(origin.x + dx, origin.y + dy)
}

#[cfg(test)]
mod tests;
