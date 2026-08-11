//! Entity management system
//! Handles creatures, heroes, and their runtime state

use crate::state::tile_state::TilePos;
use crate::state::OwnerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an entity
pub type EntityId = usize;

/// Main entity container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub entity_type: EntityType,
    pub pos: TilePos,
    #[serde(default)]
    pub owner: OwnerId,
    #[serde(skip, default = "default_visual_pos")]
    pub visual_pos: (f32, f32),
    #[serde(skip, default = "default_damage_time")]
    pub last_damage_time: f32,
}

fn default_damage_time() -> f32 {
    -100.0
}

fn default_visual_pos() -> (f32, f32) {
    (0.0, 0.0)
}

impl Entity {
    /// Create a new creature entity
    pub fn new_creature(id: EntityId, pos: TilePos, creature_state: CreatureState) -> Self {
        Self::new_creature_for_owner(id, pos, creature_state, OwnerId::Player)
    }

    pub fn new_creature_for_owner(
        id: EntityId,
        pos: TilePos,
        creature_state: CreatureState,
        owner: OwnerId,
    ) -> Self {
        Self {
            id,
            entity_type: EntityType::Creature(creature_state),
            pos,
            owner,
            visual_pos: (pos.x as f32, pos.y as f32),
            last_damage_time: -100.0,
        }
    }

    /// Create a new hero entity
    pub fn new_hero(id: EntityId, pos: TilePos, hero_state: HeroState) -> Self {
        Self::new_hero_for_owner(id, pos, hero_state, OwnerId::Heroes)
    }

    pub fn new_hero_for_owner(
        id: EntityId,
        pos: TilePos,
        hero_state: HeroState,
        owner: OwnerId,
    ) -> Self {
        Self {
            id,
            entity_type: EntityType::Hero(hero_state),
            pos,
            owner,
            visual_pos: (pos.x as f32, pos.y as f32),
            last_damage_time: -100.0,
        }
    }

    /// Create a new structure entity
    pub fn new_structure(id: EntityId, pos: TilePos, structure_state: StructureState) -> Self {
        Self::new_structure_for_owner(id, pos, structure_state, OwnerId::Heroes)
    }

    pub fn new_structure_for_owner(
        id: EntityId,
        pos: TilePos,
        structure_state: StructureState,
        owner: OwnerId,
    ) -> Self {
        Self {
            id,
            entity_type: EntityType::Structure(structure_state),
            visual_pos: (pos.x as f32, pos.y as f32),
            pos,
            owner,
            last_damage_time: -100.0,
        }
    }

    /// Get creature state if this is a creature
    pub fn as_creature(&self) -> Option<&CreatureState> {
        match &self.entity_type {
            EntityType::Creature(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable creature state if this is a creature
    pub fn as_creature_mut(&mut self) -> Option<&mut CreatureState> {
        match &mut self.entity_type {
            EntityType::Creature(state) => Some(state),
            _ => None,
        }
    }

    /// Get hero state if this is a hero
    pub fn as_hero(&self) -> Option<&HeroState> {
        match &self.entity_type {
            EntityType::Hero(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable hero state if this is a hero
    pub fn as_hero_mut(&mut self) -> Option<&mut HeroState> {
        match &mut self.entity_type {
            EntityType::Hero(state) => Some(state),
            _ => None,
        }
    }

    /// Check if entity is alive
    pub fn is_alive(&self) -> bool {
        match &self.entity_type {
            EntityType::Creature(state) => state.health > 0.0,
            EntityType::Hero(state) => state.health > 0.0,
            EntityType::Structure(state) => state.health > 0.0,
            EntityType::ResourcePile(_) => true,
        }
    }
}

/// Type of entity (creature or hero)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Creature(CreatureState),
    Hero(HeroState),
    Structure(StructureState),
    ResourcePile(ResourcePileState),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityCategory {
    Monster,
    Hero,
}

/// Runtime state for a static structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureState {
    pub building_id: String, // "town_hall", "barracks"
    pub health: f32,
    pub max_health: f32,
}

impl StructureState {
    pub fn new(building_id: String, max_health: f32) -> Self {
        Self {
            building_id,
            health: max_health,
            max_health,
        }
    }

    pub fn take_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
    }
}

/// Runtime state for a resource pile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePileState {
    pub resource_type: String, // "gold"
    pub amount: i32,
}

impl ResourcePileState {
    pub fn new(resource_type: String, amount: i32) -> Self {
        Self {
            resource_type,
            amount,
        }
    }
}

mod creature;
mod hero;

pub use creature::CreatureState;
pub use hero::HeroState;

/// Creature task types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Task {
    /// Idle/wandering
    Idle,

    /// Digging a specific tile
    Dig(TilePos),

    /// Working in a room at a specific slot
    Work(usize, TilePos), // room_id, target_pos

    /// Sleeping in lair
    Sleep(usize), // room_id

    /// Eating in hatchery
    Eat(usize), // room_id

    /// Training in training room
    Train(usize), // room_id

    /// Researching in library
    Research(usize), // room_id

    /// Depositing gold in treasury
    DepositGold(usize), // room_id

    /// Moving to a position
    MoveTo(TilePos),

    /// Attacking an entity
    Attack(EntityId),

    /// Fleeing from combat
    Flee,

    /// Collecting wages from treasury
    CollectWages(usize), // room_id

    /// Claiming a tile
    ClaimTile(TilePos),

    /// Picking up a resource pile
    PickupResource(EntityId),
}

impl Task {
    /// Get the task type as a string for matching against AI preferences
    pub fn task_type(&self) -> &str {
        match self {
            Task::Idle => "idle",
            Task::Dig(_) => "dig",
            Task::Work(_, _) => "work",
            Task::Sleep(_) => "sleep",
            Task::Eat(_) => "eat",
            Task::Train(_) => "train",
            Task::Research(_) => "research",
            Task::DepositGold(_) => "deposit_gold",
            Task::CollectWages(_) => "collect_wages",
            Task::MoveTo(_) => "move",
            Task::Attack(_) => "attack",
            Task::Flee => "flee",
            Task::ClaimTile(_) => "claim_tile",
            Task::PickupResource(_) => "pickup_resource",
        }
    }
}

/// Hero goal types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroGoal {
    /// Primary goal: destroy the dungeon heart
    DestroyHeart,

    /// Steal a specific amount of gold
    StealGold(i32),

    /// Kill a specific number of creatures
    KillCreatures(u32),

    /// Sabotage a specific room
    SabotageRoom(usize),

    /// Explore the dungeon (reveal fog)
    Explore,

    /// Rest at spawn location
    RestAtSpawn(TilePos),

    /// Retreat to entrance
    Retreat,
}

/// Status effect on an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    pub effect_type: String,
    pub duration: f32,
    pub strength: f32,
}

/// Entity manager for tracking all entities in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: EntityId,
}

impl EntityManager {
    /// Create a new entity manager
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
        }
    }

    /// Spawn a new creature
    pub fn spawn_creature(&mut self, pos: TilePos, creature_state: CreatureState) -> EntityId {
        self.spawn_creature_for_owner(pos, creature_state, OwnerId::Player)
    }

    pub fn spawn_creature_for_owner(
        &mut self,
        pos: TilePos,
        creature_state: CreatureState,
        owner: OwnerId,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;

        let entity = Entity::new_creature_for_owner(id, pos, creature_state, owner);
        self.entities.insert(id, entity);
        id
    }

    /// Spawn a new hero
    pub fn spawn_hero(&mut self, pos: TilePos, hero_state: HeroState) -> EntityId {
        self.spawn_hero_for_owner(pos, hero_state, OwnerId::Heroes)
    }

    pub fn spawn_hero_for_owner(
        &mut self,
        pos: TilePos,
        hero_state: HeroState,
        owner: OwnerId,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;

        let entity = Entity::new_hero_for_owner(id, pos, hero_state, owner);
        self.entities.insert(id, entity);
        id
    }

    /// Get an entity by ID
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Spawn a new structure
    pub fn spawn_structure(&mut self, pos: TilePos, structure_state: StructureState) -> EntityId {
        self.spawn_structure_for_owner(pos, structure_state, OwnerId::Heroes)
    }

    pub fn spawn_structure_for_owner(
        &mut self,
        pos: TilePos,
        structure_state: StructureState,
        owner: OwnerId,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;

        let entity = Entity::new_structure_for_owner(id, pos, structure_state, owner);
        self.entities.insert(id, entity);
        id
    }

    /// Spawn a new resource pile
    pub fn spawn_resource_pile(&mut self, pos: TilePos, state: ResourcePileState) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;

        let entity = Entity {
            id,
            entity_type: EntityType::ResourcePile(state),
            pos,
            owner: OwnerId::Neutral,
            visual_pos: (pos.x as f32, pos.y as f32),
            last_damage_time: -100.0,
        };
        self.entities.insert(id, entity);
        id
    }

    /// Get a mutable entity by ID
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// Remove an entity (death, despawn, etc.)
    pub fn remove(&mut self, id: EntityId) -> Option<Entity> {
        self.entities.remove(&id)
    }

    /// Count all tracked entities.
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// Count all creature entities.
    pub fn count_creatures(&self) -> usize {
        self.entities
            .values()
            .filter(|entity| matches!(entity.entity_type, EntityType::Creature(_)))
            .count()
    }

    /// Remove all entities whose health has reached zero.
    pub fn remove_dead(&mut self) -> usize {
        let before = self.entities.len();
        self.entities.retain(|_, entity| entity.is_alive());
        before - self.entities.len()
    }

    /// Get all entities
    pub fn all(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    /// Get all mutable entities
    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.values_mut()
    }

    /// Get the entities map mutably (for combat resolution)
    pub fn entities_mut(&mut self) -> &mut HashMap<EntityId, Entity> {
        &mut self.entities
    }

    /// Get the entities map immutably
    pub fn entities(&self) -> &HashMap<EntityId, Entity> {
        &self.entities
    }

    /// Get all creatures
    pub fn creatures(&self) -> impl Iterator<Item = (EntityId, &CreatureState)> {
        self.entities
            .iter()
            .filter_map(|(id, entity)| entity.as_creature().map(|creature| (*id, creature)))
    }

    /// Get all heroes
    pub fn heroes(&self) -> impl Iterator<Item = (EntityId, &HeroState)> {
        self.entities
            .iter()
            .filter_map(|(id, entity)| entity.as_hero().map(|hero| (*id, hero)))
    }

    /// Get entities at a specific position
    pub fn at_position(&self, pos: TilePos) -> impl Iterator<Item = &Entity> {
        self.entities.values().filter(move |e| e.pos == pos)
    }

    /// Get mutable entities at a specific position
    pub fn at_position_mut(&mut self, pos: TilePos) -> impl Iterator<Item = &mut Entity> {
        self.entities.values_mut().filter(move |e| e.pos == pos)
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
