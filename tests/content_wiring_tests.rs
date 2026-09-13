//! Content wiring tests
//!
//! Shipping a room is four separate edits — a `rooms.json` entry, art, a
//! `technologies.json` tech that unlocks it, and a slot in each mission's
//! `availability` — spread across files that never reference each other in
//! Rust. Nothing but these tests notices when one of the four is misspelled or
//! forgotten, and a room that no tech unlocks is a room the player can see in
//! the build bar and never build.
//!
//! Run with: cargo test --test content_wiring_tests

use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load(rel: &str) -> Vec<Value> {
    let path = project_root().join(rel);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse {} as a JSON array: {e}", path.display()))
}

fn ids(rel: &str) -> HashSet<String> {
    load(rel)
        .iter()
        .map(|entry| entry["id"].as_str().expect("id").to_string())
        .collect()
}

fn room_ids() -> HashSet<String> {
    let mut all = ids("assets/data/rooms.json");
    all.extend(ids("assets/data/special_rooms.json"));
    all
}

fn assert_empty(problems: Vec<String>, what: &str) {
    assert!(problems.is_empty(), "{what}:\n  {}", problems.join("\n  "));
}

#[test]
fn every_tech_unlock_names_real_content() {
    let rooms = room_ids();
    let spells = ids("assets/data/dungeon_spells.json");
    let creatures = ids("assets/data/monsters.json");
    let traps = ids("assets/data/traps.json");

    let mut problems = Vec::new();
    for tech in load("assets/data/technologies.json") {
        let tech_id = tech["id"].as_str().expect("tech id");
        for (kind, known) in [
            ("rooms", &rooms),
            ("spells", &spells),
            ("creatures", &creatures),
            ("traps", &traps),
        ] {
            let Some(list) = tech["unlocks"][kind].as_array() else {
                continue;
            };
            for unlocked in list {
                let id = unlocked.as_str().expect("unlock id");
                if !known.contains(id) {
                    problems.push(format!("tech `{tech_id}` unlocks unknown {kind} `{id}`"));
                }
            }
        }
    }
    assert_empty(
        problems,
        "technology unlocks reference content that does not exist",
    );
}

#[test]
fn every_tech_prerequisite_exists() {
    let techs = ids("assets/data/technologies.json");
    let mut problems = Vec::new();
    for tech in load("assets/data/technologies.json") {
        let tech_id = tech["id"].as_str().expect("tech id");
        for prereq in tech["prerequisites"].as_array().expect("prerequisites") {
            let id = prereq.as_str().expect("prerequisite id");
            if !techs.contains(id) {
                problems.push(format!("tech `{tech_id}` requires unknown tech `{id}`"));
            }
        }
    }
    assert_empty(
        problems,
        "technology prerequisites reference techs that do not exist",
    );
}

#[test]
fn every_room_not_unlocked_by_default_has_a_tech_that_grants_it() {
    // Rooms used to carry a `requirements.research` list beside the tech tree.
    // Nothing enforced it and the sidebar displayed it, so it drifted: six
    // rooms showed no requirement while being tech-locked, and the scavenger
    // room named `ritual_tech` when `logistics` was the real gate. The field
    // is gone; this is the check that replaces it — a room the player does not
    // start with must be reachable through research, or it can never be built.
    const UNLOCKED_AT_START: &[&str] = &[
        // Mirrors `PlayerState::new`.
        "lair",
        "hatchery",
        "treasury",
        "library",
        "dungeon_heart",
    ];

    let mut unlocked_by_tech: HashSet<String> = HashSet::new();
    for tech in load("assets/data/technologies.json") {
        for room in tech["unlocks"]["rooms"].as_array().expect("unlocks.rooms") {
            unlocked_by_tech.insert(room.as_str().expect("room id").to_string());
        }
    }

    let mut unreachable = Vec::new();
    for room in load("assets/data/rooms.json") {
        let id = room["id"].as_str().expect("room id");
        if UNLOCKED_AT_START.contains(&id) || unlocked_by_tech.contains(id) {
            continue;
        }
        unreachable.push(format!(
            "room `{id}` is neither unlocked at start nor granted by any technology"
        ));
    }

    assert_empty(unreachable, "rooms the player can never build");
}

#[test]
fn every_scenario_availability_entry_names_real_content() {
    let rooms = room_ids();
    let spells = ids("assets/data/dungeon_spells.json");
    let creatures = ids("assets/data/monsters.json");
    let traps = ids("assets/data/traps.json");

    let scenarios = project_root().join("assets").join("scenarios");
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(&scenarios)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", scenarios.display()))
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).expect("read scenario");
        let parsed: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
        // Scenario files are written both as a bare object and as a one-element
        // array; accept either rather than making authors care.
        let scenario = parsed.get(0).unwrap_or(&parsed);
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        for (kind, known) in [
            ("rooms", &rooms),
            ("spells", &spells),
            ("creatures", &creatures),
            ("traps", &traps),
        ] {
            let Some(entries) = scenario["availability"][kind].as_object() else {
                continue;
            };
            for id in entries.keys() {
                if !known.contains(id.as_str()) {
                    problems.push(format!("scenario `{name}` lists unknown {kind} `{id}`"));
                }
            }
        }
    }
    assert_empty(
        problems,
        "scenario availability references content that does not exist",
    );
}

/// Status types the engine actually acts on. `poison`/`burn` tick damage in
/// `combat::dot_damage`, `stun` blocks an attack in `resolve_combat_tick`,
/// `freeze` scales `movement_speed` on application and divides it back out on
/// expiry, and `fear` raises a hero's breaking point in
/// `hero_ai::current_retreat_threshold`.
///
/// `speed_modifier` is applied on status arrival and reverted on expiry just
/// like `freeze`; `charm` blocks attacks like a tactical stun. `none` is the
/// explicit marker for a deterministic stat multiplier with no proc status.
const ENGINE_CONSUMED_STATUS: &[&str] = &[
    "poison",
    "burn",
    "stun",
    "freeze",
    "fear",
    "speed_modifier",
    "charm",
    "none",
];

#[test]
fn every_creature_ability_is_wired_or_declared_inert() {
    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(project_root().join("assets/data/game_config.json"))
            .expect("read game_config.json"),
    )
    .expect("parse game_config.json");

    let effects = config["status_effects"]["ability_effects"]
        .as_object()
        .expect("status_effects.ability_effects");

    let mut problems = Vec::new();

    for creature in load("assets/data/monsters.json") {
        let id = creature["id"].as_str().expect("id");
        let abilities = creature["combat"]["abilities"]
            .as_array()
            .expect("combat.abilities");

        for ability in abilities {
            let name = ability.as_str().expect("ability name");
            match effects.get(name) {
                None => problems.push(format!(
                    "creature `{id}` has ability `{name}`, which is neither in \
                     game_config's ability_effects nor declared inert"
                )),
                Some(effect) => {
                    let status = effect["status_type"].as_str().unwrap_or("");
                    if !ENGINE_CONSUMED_STATUS.contains(&status) {
                        problems.push(format!(
                            "creature `{id}` ability `{name}` applies status \
                             `{status}`, which no engine code acts on"
                        ));
                    }
                }
            }
        }
    }

    assert_empty(problems, "creature abilities that go nowhere");
}

#[test]
fn every_creature_trait_exists_in_traits_json() {
    // Traits are tag strings. An unknown tag is not an error anywhere — the
    // creature simply gets no modifier — so a typo turns a design intent into
    // silence. This is the same failure mode as the tech and room references
    // above, for the one content vocabulary that had no guard.
    let known = ids("assets/data/traits.json");
    let mut problems = Vec::new();

    for creature in load("assets/data/monsters.json") {
        let id = creature["id"].as_str().expect("id");
        for tag in creature["traits"].as_array().expect("traits") {
            let tag = tag.as_str().expect("trait name");
            if !known.contains(tag) {
                problems.push(format!(
                    "creature `{id}` has trait `{tag}`, which traits.json does not define"
                ));
            }
        }
    }

    assert_empty(problems, "creature traits that modify nothing");
}

#[test]
fn every_mutation_target_exists() {
    // The goblin declared a `hobgoblin` mutation for as long as this repo has
    // existed, and no such creature was in the roster. Nothing noticed, because
    // nothing read `mutations` at all — the reference and its consumer were
    // missing together, which is the quietest version of this failure.
    let creatures = ids("assets/data/monsters.json");
    let mut problems = Vec::new();

    for creature in load("assets/data/monsters.json") {
        let id = creature["id"].as_str().expect("id");
        let Some(mutations) = creature["progression"]["mutations"].as_array() else {
            continue;
        };
        for mutation in mutations {
            let target = mutation["id"].as_str().expect("mutation id");
            if !creatures.contains(target) {
                problems.push(format!(
                    "creature `{id}` mutates into `{target}`, which is not in the roster"
                ));
            }
        }
    }

    assert_empty(problems, "mutations that lead nowhere");
}

/// Condition keys the mutation engine understands. Anything else fails closed,
/// which means the mutation silently never happens — so a typo here is a
/// design intent that quietly evaporates.
#[test]
fn every_mutation_condition_key_is_understood() {
    let mut problems = Vec::new();
    let rooms = room_ids();

    for creature in load("assets/data/monsters.json") {
        let id = creature["id"].as_str().expect("id");
        let Some(mutations) = creature["progression"]["mutations"].as_array() else {
            continue;
        };
        for mutation in mutations {
            let Some(conditions) = mutation["conditions"].as_object() else {
                continue;
            };
            for key in conditions.keys() {
                if key == "level_at_least" {
                    continue;
                }
                match key.strip_suffix("_tiles") {
                    Some(room) if rooms.contains(room) => {}
                    Some(room) => problems.push(format!(
                        "creature `{id}` gates a mutation on `{key}`, but `{room}` is not a room"
                    )),
                    None => problems.push(format!(
                        "creature `{id}` gates a mutation on `{key}`, which the engine \
                         does not understand (see engine/mutation.rs)"
                    )),
                }
            }
        }
    }

    assert_empty(problems, "mutation conditions that can never be satisfied");
}

#[test]
fn every_creature_can_actually_be_obtained() {
    // Fifteen creatures were added to this roster in quick succession, each
    // wired through a different route — a tech unlock, a mutation target, or a
    // scenario's starting availability. A creature with art, stats and no route
    // is content the player can never see, and nothing else checks for it.
    const STARTING_CREATURES: &[&str] = &[
        // Spawned by the dungeon heart itself rather than unlocked.
        "imp",
    ];

    let mut reachable: HashSet<String> = STARTING_CREATURES.iter().map(|s| s.to_string()).collect();

    for tech in load("assets/data/technologies.json") {
        for id in tech["unlocks"]["creatures"].as_array().expect("creatures") {
            reachable.insert(id.as_str().expect("creature id").to_string());
        }
    }

    for creature in load("assets/data/monsters.json") {
        let Some(mutations) = creature["progression"]["mutations"].as_array() else {
            continue;
        };
        for mutation in mutations {
            reachable.insert(mutation["id"].as_str().expect("mutation id").to_string());
        }
    }

    let scenarios = project_root().join("assets").join("scenarios");
    for entry in std::fs::read_dir(&scenarios)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", scenarios.display()))
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).expect("read scenario");
        let parsed: Value = serde_json::from_str(&raw).expect("parse scenario");
        let scenario = parsed.get(0).unwrap_or(&parsed);
        if let Some(entries) = scenario["availability"]["creatures"].as_object() {
            reachable.extend(entries.keys().cloned());
        }
    }

    let mut stranded = Vec::new();
    for creature in load("assets/data/monsters.json") {
        let id = creature["id"].as_str().expect("id");
        if !reachable.contains(id) {
            stranded.push(format!(
                "creature `{id}` is unlocked by no tech, is no mutation's target, and \
                 appears in no scenario's availability"
            ));
        }
    }

    assert_empty(stranded, "creatures the player can never obtain");
}

/// Aura effect keys the engine acts on. `tile_aura::hero_fear_at` reads
/// `hero_fear`; anything else is silently ignored, so an authored effect that
/// is not on this list is a design intent that evaporates.
const ENGINE_CONSUMED_AURA_EFFECTS: &[&str] = &["hero_fear"];

#[test]
fn every_tile_aura_effect_is_one_the_engine_acts_on() {
    let mut problems = Vec::new();

    for tile in load("assets/data/tiles.json") {
        let id = tile["id"].as_str().expect("id");
        let Some(effects) = tile["special"]["aura"]["effects"].as_object() else {
            continue;
        };
        for key in effects.keys() {
            if !ENGINE_CONSUMED_AURA_EFFECTS.contains(&key.as_str()) {
                problems.push(format!(
                    "tile `{id}` declares aura effect `{key}`, which no engine code reads"
                ));
            }
        }
    }

    assert_empty(problems, "tile aura effects that go nowhere");
}

/// Light preferences the engine acts on. Anything else falls through to "no
/// effect", so a typo turns an authored personality trait into silence — the
/// same fail-quiet shape as mutation conditions, aura effects and abilities.
const ENGINE_CONSUMED_LIGHT_PREFERENCES: &[&str] = &["bright", "dark", "any"];

#[test]
fn every_hero_light_preference_is_one_the_engine_understands() {
    let mut problems = Vec::new();

    for hero in load("assets/data/heroes.json") {
        let id = hero["id"].as_str().expect("id");
        let Some(preference) = hero["behavior"]["light_preference"].as_str() else {
            continue;
        };
        if !ENGINE_CONSUMED_LIGHT_PREFERENCES.contains(&preference) {
            problems.push(format!(
                "hero `{id}` prefers light `{preference}`, which lighting::discomfort_for \
                 does not recognise"
            ));
        }
    }

    assert_empty(problems, "hero light preferences that mean nothing");
}
