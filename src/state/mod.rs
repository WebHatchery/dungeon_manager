pub mod camera_state;
pub mod campaign_store;
mod campaign_flow;
mod combat_tick;
pub mod drag_selection;
pub mod dungeon;
pub mod entities;
pub mod faction;
mod fog_of_war;
pub mod game_state;
mod heart_attack;
pub mod hero_base;
pub mod interaction;
pub mod map_loader;
pub mod notifications;
pub mod player_state;
pub mod projectiles;
pub mod rival_keeper;
pub mod room_manager;
mod rooms;
pub mod save_system;
mod scenario_setup;
pub mod scenario_state;
pub mod settings;
pub mod skirmish;
mod spawning;
pub mod tile_state;
pub mod tutorial;

#[cfg(target_arch = "wasm32")]
pub mod wasm_storage;

pub use drag_selection::DragSelection;
pub use faction::OwnerId;
pub use game_state::MapType;
pub use interaction::{GamePhase, InteractionMode};
pub use tile_state::{Ownership, TilePos};
