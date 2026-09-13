pub mod campaign;
pub mod content_pack;
pub mod content_source;
pub(crate) mod embedded;
pub mod game_config;
pub mod hero_buildings;
pub mod heroes;
pub mod monsters;
pub mod rooms;
pub mod scenario;
pub mod spells;
pub mod technologies;
pub mod tiles;
pub mod traits;
pub mod traps;
pub mod tutorial;

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

pub use campaign::CampaignDefinition;
pub use game_config::GameConfig;
pub use hero_buildings::HeroBuildingData;
pub use heroes::HeroData;
pub use monsters::MonsterData;
pub use rooms::RoomData;
pub use scenario::ScenarioDefinition;
pub use spells::SpellData;
pub use technologies::TechData;
pub use tiles::TileData;
pub use traits::TraitData;
pub use traps::TrapData;
pub use tutorial::TutorialData;

#[derive(Default)]
pub struct GameData {
    pub tiles: HashMap<String, TileData>,
    pub rooms: HashMap<String, RoomData>,
    pub monsters: HashMap<String, MonsterData>,
    pub heroes: HashMap<String, HeroData>,
    pub spells: HashMap<String, SpellData>,
    pub traps: HashMap<String, TrapData>,
    pub technologies: HashMap<String, TechData>,
    pub traits: HashMap<String, TraitData>,
    pub hero_buildings: HashMap<String, HeroBuildingData>,
    pub scenarios: HashMap<String, ScenarioDefinition>,
    pub campaigns: HashMap<String, CampaignDefinition>,
    pub tutorial: TutorialData,
    pub config: GameConfig,
    pub content_pack_reports: Vec<content_pack::ContentPackReport>,
    pub asset_roots: Vec<PathBuf>,
    pub map_roots: Vec<PathBuf>,
}

impl GameData {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let tiles = tiles::load_tiles()?;
        let rooms = rooms::load_rooms()?;
        let monsters = monsters::load_monsters()?;
        let heroes = heroes::load_heroes()?;
        let spells = spells::load_spells()?;
        let traps = traps::load_traps()?;
        let technologies = technologies::load_technologies()?;
        let traits = traits::load_traits()?;
        let hero_buildings = hero_buildings::load_hero_buildings()?;
        let scenarios = scenario::load_scenarios()?;
        let campaigns = campaign::load_campaigns()?;
        let config = game_config::load_game_config()?;
        let tutorial = tutorial::load_tutorial()?;
        let tutorial_problems = tutorial.validate();
        if !tutorial_problems.is_empty() {
            return Err(tutorial_problems.join("; ").into());
        }

        let data = Self {
            tiles,
            rooms,
            monsters,
            heroes,
            spells,
            traps,
            technologies,
            traits,
            hero_buildings,
            scenarios,
            campaigns,
            tutorial,
            config,
            content_pack_reports: Vec::new(),
            asset_roots: Vec::new(),
            map_roots: Vec::new(),
        };
        data.validate_content()?;
        Ok(data)
    }

    pub fn load_with_default_mod_order() -> Result<Self, Box<dyn Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mods_root = Path::new("assets/mods");
            let load_order_path = mods_root.join("load_order.json");
            if load_order_path.exists() {
                return Self::load_with_mod_order(mods_root, &load_order_path);
            }
        }

        Self::load()
    }

    pub fn validate_scenarios(&self) -> Vec<String> {
        self.scenarios
            .values()
            .flat_map(|scenario| scenario.validate_references(self))
            .collect()
    }

    /// Validate all cross-file references and the fixed vocabularies that the
    /// runtime deliberately supports. This runs after base data and after
    /// content packs are merged, so a mod cannot silently author an inert
    /// trigger or effect.
    pub fn validate_content(&self) -> Result<(), Box<dyn Error>> {
        let mut problems = self.validate_scenarios();

        for (hero_id, hero) in &self.heroes {
            for ability in &hero.abilities {
                if !is_supported_hero_trigger(&ability.trigger) {
                    problems.push(format!(
                        "hero '{hero_id}' ability '{}' uses unsupported trigger '{}'",
                        ability.id, ability.trigger
                    ));
                }
                validate_effects(
                    &mut problems,
                    &format!("hero '{hero_id}' ability '{}'", ability.id),
                    &ability.effects,
                );
            }
        }
        for (spell_id, spell) in &self.spells {
            validate_effects(
                &mut problems,
                &format!("spell '{spell_id}'"),
                &spell.effects,
            );
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("; ").into())
        }
    }

    pub fn load_with_content_packs(
        manifests: &[(PathBuf, content_pack::ContentPackManifest)],
    ) -> Result<Self, Box<dyn Error>> {
        let mut data = Self::load()?;
        content_pack::apply_ordered_content_packs(&mut data, manifests)?;
        data.validate_content()?;
        Ok(data)
    }

    pub fn load_with_mod_order(
        mods_root: &Path,
        load_order_path: &Path,
    ) -> Result<Self, Box<dyn Error>> {
        let load_order = content_pack::load_order(load_order_path)?;
        let manifests = content_pack::load_manifests_from_order(mods_root, &load_order)?;
        Self::load_with_content_packs(&manifests)
    }

    pub fn register_content_pack_roots(
        &mut self,
        manifest: &content_pack::ContentPackManifest,
        root: &Path,
    ) {
        for asset_root in &manifest.assets {
            self.asset_roots.push(root.join(asset_root));
        }
        for map_root in &manifest.maps {
            self.map_roots.push(root.join(map_root));
        }
    }

    pub fn resolve_map_path(&self, requested: &str) -> PathBuf {
        self.resolve_pack_path(&self.map_roots, requested)
            .unwrap_or_else(|| PathBuf::from(requested))
    }

    pub fn resolve_asset_path(&self, requested: &str) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            requested.to_string()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.resolve_pack_path(&self.asset_roots, requested)
                .unwrap_or_else(|| PathBuf::from(requested))
                .to_string_lossy()
                .replace('\\', "/")
        }
    }

    fn resolve_pack_path(&self, roots: &[PathBuf], requested: &str) -> Option<PathBuf> {
        for root in roots.iter().rev() {
            for candidate in pack_path_candidates(root, requested) {
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        None
    }
}

fn is_supported_hero_trigger(trigger: &str) -> bool {
    matches!(
        trigger,
        "passive"
            | "on_low_health"
            | "on_self_low_health"
            | "defensive"
            | "on_hit"
            | "on_damaged"
            | "on_ally_low_health"
            | "on_party_damaged"
            | "on_target"
            | "on_armored_target"
            | "on_target_isolated"
            | "on_multiple_targets"
            | "on_group"
            | "on_creature_nearby"
            | "on_open_area"
            | "on_large_group"
            | "on_undead_nearby"
            | "on_ritual_detected"
            | "on_corruption"
            | "on_corruption_detected"
            | "on_sneak_attack"
            | "on_trapped"
    ) || trigger == "in_room"
        || trigger
            .strip_prefix("in_room:")
            .is_some_and(|room| !room.is_empty())
}

fn validate_effects(
    problems: &mut Vec<String>,
    source: &str,
    effects: &[spells::SpellEffect],
) {
    for (index, effect) in effects.iter().enumerate() {
        let supported = matches!(
            effect.effect_type.as_str(),
            "damage"
                | "heal"
                | "stat_modifier"
                | "status_apply"
                | "polymorph"
                | "tile_transform"
                | "spawn_entity"
                | "reveal_map"
        );
        if !supported {
            problems.push(format!(
                "{source} effect {index} uses unsupported effect type '{}'",
                effect.effect_type
            ));
        }
        for (label, value) in [
            ("amount", effect.amount),
            ("multiplier", effect.multiplier.unwrap_or(0.0)),
            ("duration", effect.duration.unwrap_or(0.0)),
            ("delay", effect.delay.unwrap_or(0.0)),
        ] {
            if !value.is_finite() {
                problems.push(format!(
                    "{source} effect {index} has non-finite {label}"
                ));
            }
        }
        if effect.effect_type == "polymorph"
            && effect.target_form.as_deref() != Some("chicken")
        {
            problems.push(format!(
                "{source} effect {index} has unsupported polymorph target {:?}",
                effect.target_form
            ));
        }
    }
}

fn pack_path_candidates(root: &Path, requested: &str) -> Vec<PathBuf> {
    let mut candidates = vec![root.join(requested)];

    for prefix in ["assets/maps/", "assets/", "maps/"] {
        if let Some(stripped) = requested.strip_prefix(prefix) {
            candidates.push(root.join(stripped));
        }
    }

    candidates
}
