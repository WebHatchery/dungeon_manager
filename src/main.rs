#![allow(dead_code, clippy::large_enum_variant, clippy::too_many_arguments)]

use macroquad::prelude::*;
use macroquad_toolkit::capture;

mod config;
mod data;
#[macro_use]
mod debug_log;
mod draw_utils;
mod engine;
mod sprite_variation;
mod state;
mod ui;

#[cfg(test)]
mod combat_tests;
#[cfg(test)]
mod command_aura_tests;
#[cfg(test)]
mod grafting_tests;
#[cfg(test)]
mod hero_abilities_tests;
#[cfg(test)]
mod hero_base_tests;
#[cfg(test)]
mod hero_behaviour_tests;
#[cfg(test)]
mod lighting_tests;
#[cfg(test)]
mod mining_tests;
#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod player_feedback_tests;
#[cfg(test)]
mod room_mood_tests;
#[cfg(test)]
mod room_placement_tests;
#[cfg(test)]
mod spell_targeting_tests;
#[cfg(test)]
mod task_system_tests;
#[cfg(test)]
mod threat_tests;
#[cfg(test)]
mod tile_aura_tests;
#[cfg(test)]
mod traits_tests;
#[cfg(test)]
mod wall_reinforcement_tests;

use data::GameData;
use engine::action_processor;
use engine::input::InputHandler;
use state::game_state::GameState;
use state::MapType;
use state::{DragSelection, GamePhase, InteractionMode};
use ui::actions::ActionQueue;
use ui::renderer::GameRenderer;

pub struct Game {
    phase: GamePhase,
    game_data: Option<GameData>,
    renderer: GameRenderer,
    interaction_mode: InteractionMode,
    hovered_tile: Option<state::tile_state::TilePos>,
    selected_map_type: MapType,
    selected_entity: Option<state::entities::EntityId>,
    selected_room: Option<usize>,
    held_entity: Option<state::entities::EntityId>,
    action_queue: ActionQueue,
    selected_spell: Option<String>,
    drag_selection: DragSelection,
    settings: state::settings::GameSettings,
    /// Autosave cadence. Lives on `Game` rather than on `GameState` for two
    /// reasons: the timer is a property of the session and has no business
    /// inside a save file, and the manager's `update` takes a closure that
    /// borrows the state it is saving — which it could not do from inside it.
    autosave: macroquad_toolkit::persistence::AutoSaveManager,
}

impl Game {
    fn new() -> Self {
        Self {
            phase: GamePhase::Loading,
            game_data: None,
            renderer: GameRenderer::new(),
            interaction_mode: InteractionMode::None,
            hovered_tile: None,
            selected_map_type: MapType::Standard, // Default to Standard Map
            selected_entity: None,
            selected_room: None,
            held_entity: None,
            action_queue: ActionQueue::new(),
            selected_spell: None,
            drag_selection: DragSelection::new(),
            settings: state::settings::GameSettings::load(),
            // Re-created from config once game data loads; the interval is
            // authored, not a constant.
            autosave: macroquad_toolkit::persistence::AutoSaveManager::default(),
        }
    }

    async fn load_resources(&mut self) {
        // Load game data
        if self.game_data.is_none() {
            match GameData::load_with_default_mod_order() {
                Ok(data) => {
                    eprintln!("Successfully loaded game data!");
                    self.autosave = macroquad_toolkit::persistence::AutoSaveManager::new(
                        data.config.timing.autosave_interval,
                    );
                    self.game_data = Some(data);
                }
                Err(e) => {
                    eprintln!("Failed to load game data: {}", e);
                    return;
                }
            }
        }

        // Load graphics
        if self.renderer.graphics_cache.is_none() {
            eprintln!("Loading graphics...");
            self.renderer.load_resources(self.game_data.as_ref()).await;
            if self.renderer.graphics_cache.is_some() {
                self.phase = GamePhase::MainMenu;
            }
        }
    }

    fn update(&mut self, dt: f32) {
        InputHandler::update(
            dt,
            &mut self.phase,
            &mut self.game_data, // Changed to mut to allow applying cheats
            &mut self.interaction_mode,
            &mut self.selected_map_type,
            &mut self.hovered_tile,
            &mut self.held_entity,
            &mut self.selected_entity,
            &mut self.selected_room,
            &mut self.renderer.sidebar,
            &mut self.action_queue,
            &mut self.drag_selection,
            &mut self.settings,
        );

        // Process queued actions
        if let GamePhase::Playing(ref mut state) = self.phase {
            if let Some(ref game_data) = self.game_data {
                // Update helpers (traps, etc)
                crate::engine::trap_system::process_trap_construction(
                    &mut state.dungeon,
                    &mut state.player,
                    &mut state.pending_trap_builds,
                    game_data,
                    dt,
                );

                action_processor::process_actions(
                    &mut self.action_queue,
                    state,
                    &mut self.interaction_mode,
                    &mut self.selected_spell,
                );
            }

            Self::tick_autosave(&mut self.autosave, state, dt);
        }
    }

    /// Advance the autosave timer and write if it has come due.
    ///
    /// Gated on the game actually running: a paused game, a finished one, or an
    /// unread mission briefing all mean the player is not making progress worth
    /// preserving, and a timer that ran while the pause menu sat open would
    /// autosave the moment they walked back to the keyboard.
    ///
    /// Writes to [`SaveSlot::Auto`], never to the player's slot — an autosave
    /// that overwrote the save someone deliberately made would be worse than no
    /// autosave at all.
    fn tick_autosave(
        autosave: &mut macroquad_toolkit::persistence::AutoSaveManager,
        state: &mut GameState,
        dt: f32,
    ) {
        use crate::state::save_system::{save_game, should_autosave, SaveSlot};

        let running = should_autosave(state);
        let result = autosave.update(dt, running, || save_game(state, SaveSlot::Auto));

        match result {
            Ok(true) => state.notifications.info("Autosaved."),
            Ok(false) => {}
            Err(e) => {
                // Same rule as every other swallowed failure in this project:
                // the player is told, once, rather than it going to stderr.
                state
                    .player
                    .warn_once("autosave_failed", format!("Autosave failed: {e}"));
            }
        }
    }

    fn draw(&mut self) {
        clear_background(Color::new(0.05, 0.05, 0.05, 1.0)); // Ensure background is cleared every frame

        match self.phase {
            GamePhase::Playing(ref mut state) => {
                if let Some(ref data) = self.game_data {
                    self.renderer.draw_game(
                        state,
                        &self.interaction_mode,
                        self.hovered_tile,
                        data,
                        &self.drag_selection,
                    );
                }
                self.renderer.draw_gui(
                    state,
                    &mut self.interaction_mode,
                    self.hovered_tile,
                    self.held_entity,
                    self.selected_entity,
                    self.selected_room,
                    &self.game_data, // Helper takes Option
                    &self.drag_selection,
                );
            }
            _ => {
                self.renderer.draw(
                    &self.phase,
                    None,
                    &mut self.interaction_mode,
                    &self.selected_map_type,
                    self.hovered_tile,
                    self.held_entity,
                    self.selected_entity,
                    self.selected_room,
                    &self.game_data,
                    &self.drag_selection,
                    &self.settings,
                );
            }
        }
    }

    /// Seed a specific scene for the screenshot harness. Call after
    /// `load_resources` so `game_data` is populated.
    fn begin_capture_scene(&mut self, scene: &str) {
        match scene {
            "mainmenu" => {
                self.phase = GamePhase::MainMenu;
            }
            "skirmish" => {
                self.phase =
                    GamePhase::SkirmishSetup(crate::state::skirmish::SkirmishConfig::default());
            }
            "loadgame" => {
                // The save-slot browser. Whatever slots this machine happens to
                // have are what it shows — an all-empty capture is the honest
                // picture of a first run, and still proves the layout.
                self.phase = GamePhase::LoadGame(crate::state::interaction::SlotBrowser::open(
                    crate::state::interaction::SlotBrowserPurpose::Load,
                ));
            }
            "settings" => {
                self.phase = GamePhase::Settings;
            }
            "missionselect" => {
                self.phase = match self.game_data.as_ref() {
                    Some(data) if data.campaigns.contains_key("deep_dominion") => {
                        let progress = crate::data::campaign::CampaignProgress::new(
                            data.campaigns.get("deep_dominion").unwrap(),
                        );
                        GamePhase::MissionSelect(progress)
                    }
                    _ => GamePhase::MainMenu,
                };
            }
            _ => {
                // Default ("gameplay"): jump straight into a playable dungeon
                // so the capture photographs the main game view.
                //
                // "wave" is "simulation" with the first hero wave pulled
                // forward, so combat is reachable in a capture at all.
                //
                // "simulation" goes further and dismisses the mission intro,
                // because the intro overlay returns from input handling before
                // `state.update` — so a `gameplay` capture photographs a frozen
                // dungeon behind a modal and exercises no combat, digging or
                // spawning at all. Both scenes are kept: the modal is a real
                // screen worth a screenshot, it is just not the game running.
                if let Some(ref data) = self.game_data {
                    let mut game_state = if data.campaigns.contains_key("deep_dominion") {
                        GameState::new_campaign_start(data, "deep_dominion")
                    } else {
                        GameState::new_with_map_type(
                            data.config.map_size.width,
                            data.config.map_size.height,
                            data,
                            self.selected_map_type.clone(),
                        )
                    };

                    if scene == "simulation" || scene == "wave" || scene == "raid" {
                        game_state.tutorial.intro_dismissed = true;
                        seed_dig_orders(&mut game_state, data);
                    }

                    if scene == "wave" || scene == "raid" {
                        // The first wave is authored 600s out, which is 36,000
                        // frames — far past any practical capture. Pull it
                        // forward so combat, hero nerve and destruction
                        // effects can be observed at all.
                        game_state.hero_base.time_until_next_wave = 2.0;
                    }

                    if scene == "raid" {
                        seed_raiding_party(&mut game_state, data);
                    }

                    self.phase = GamePhase::Playing(game_state);
                } else {
                    self.phase = GamePhase::MainMenu;
                }
            }
        }
    }
}

/// Put an army in the field and point it at the hero base.
///
/// The keeper's own assault is unreachable in a normal capture for two
/// reasons: play starts with two minions against 200 HP buildings, and the
/// hero base sits behind solid rock that creatures cannot dig — only imps
/// dig, and only where the keeper has marked. So the scene also launches a
/// wave, and the raiders walk back out along the corridor the heroes tunnel
/// in through. That is the same route a player would use, which is why it is
/// worth seeding rather than teleporting an army to the gates.
fn seed_raiding_party(state: &mut GameState, game_data: &GameData) {
    use state::entities::CreatureState;

    if !state.hero_base.enabled {
        return;
    }

    // Enough to overcome the garrison; the point is to reach a demolition
    // inside a few thousand frames, not to model a fair fight.
    const RAIDERS: [(&str, usize); 3] = [("orc", 6), ("troll", 3), ("demon_spawn", 3)];

    let heart = state
        .find_dungeon_heart_position()
        .unwrap_or(state.hero_base.position);

    for (creature_id, count) in RAIDERS {
        let Some(monster) = game_data.monsters.get(creature_id) else {
            continue;
        };
        for _ in 0..count {
            let creature = CreatureState::new(
                creature_id.to_string(),
                5,
                monster.stats.health,
                monster.stats.mana,
                macroquad_toolkit::rng::random_u64(),
            );
            state.entities.spawn_creature(heart, creature);
        }
    }

    // The order that sends them: creature AI reads this marker as "go and
    // fight there".
    state.attack_marker = Some(state.hero_base.position);
}

/// Mark diggable tiles beside the player's territory so a captured
/// simulation has work to do.
///
/// Without this the imps wander: nothing is marked at mission start, so a
/// capture would exercise pathfinding and little else.
fn seed_dig_orders(state: &mut GameState, game_data: &GameData) {
    use state::tile_state::{Ownership, TilePos};

    let (width, height) = (state.dungeon.width, state.dungeon.height);
    let mut to_mark = Vec::new();

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let pos = TilePos::new(x, y);
            let Some(tile) = state.dungeon.get_tile(pos) else {
                continue;
            };
            if tile.ownership != Ownership::Unclaimed
                || !engine::tile_types::is_diggable(&tile.tile_type, game_data)
            {
                continue;
            }
            // Only tiles the imps can actually reach: adjacent to owned floor.
            let beside_player_ground = [
                TilePos::new(x + 1, y),
                TilePos::new(x - 1, y),
                TilePos::new(x, y + 1),
                TilePos::new(x, y - 1),
            ]
            .iter()
            .any(|neighbour| {
                state
                    .dungeon
                    .get_tile(*neighbour)
                    .map(|t| t.ownership == Ownership::Player)
                    .unwrap_or(false)
            });

            if beside_player_ground {
                to_mark.push(pos);
            }
        }
    }

    for pos in to_mark {
        if let Some(tile) = state.dungeon.get_tile_mut(pos) {
            tile.marked_for_dig = true;
        }
    }
}

/// Decode a generated icon PNG into the flat RGBA array miniquad wants.
///
/// The three sizes are drawn separately by `graphics_gen::icon` rather than
/// downscaled from one, because a resampled dungeon heart is mud at 16px.
fn icon_rgba<const N: usize>(png: &[u8], size: u32) -> [u8; N] {
    let decoded = image::load_from_memory(png)
        .unwrap_or_else(|e| panic!("window icon {size}px failed to decode: {e}"))
        .to_rgba8();
    assert_eq!(
        (decoded.width(), decoded.height()),
        (size, size),
        "window icon should be {size}x{size}"
    );

    let mut out = [0u8; N];
    out.copy_from_slice(decoded.as_raw());
    out
}

fn window_conf() -> Conf {
    let mut conf = capture::capture_window_conf("DUNGEON_MANAGER", "Deep Dominion", 1280, 720);
    conf.icon = Some(macroquad::miniquad::conf::Icon {
        small: icon_rgba(include_bytes!("../assets/ui/icon_16.png"), 16),
        medium: icon_rgba(include_bytes!("../assets/ui/icon_32.png"), 32),
        big: icon_rgba(include_bytes!("../assets/ui/icon_64.png"), 64),
    });
    conf
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    game.settings.apply();

    // Screenshot harness: when DUNGEON_MANAGER_CAPTURE_PATH is set, load
    // resources synchronously, seed a scene, simulate deterministic frames,
    // write a PNG, and exit.
    if let Some(configs) = capture::CaptureConfig::all_from_env("DUNGEON_MANAGER") {
        game.load_resources().await;
        for config in configs {
            game.begin_capture_scene(&config.scene);
            capture::run_capture_once(&config, |dt| {
                game.update(dt);
                game.draw();
            })
            .await;
        }
        return;
    }

    let mut loading_started = false;

    loop {
        // Handle loading phase
        if matches!(game.phase, GamePhase::Loading) && !loading_started {
            loading_started = true;
            game.load_resources().await;
        }

        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();

        next_frame().await;
    }
}
