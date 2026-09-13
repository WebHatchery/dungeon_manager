//! Guards against content-data fields that are parsed and then never read.
//!
//! Ten such fields were found one at a time while building other things, each
//! by accident: `happiness_modifier` (twelve rooms carried tuned values that
//! did nothing), `research_rate` (declared under a name no JSON used, so serde
//! silently dropped the authored value), `xp_per_minute`,
//! `creature_defense_modifier`, `sleep_recovery_rate`, `grouping_point`,
//! `lockable`, `trigger_type`, `durability`, `damage_per_second`. The failure
//! mode is always silence: the data looks authored, the entity looks
//! configured, and nothing reports that the number goes nowhere.
//!
//! So this sweeps **every** `pub struct` under `src/data/` rather than a
//! hand-kept list — a new struct cannot quietly arrive with dead fields. A
//! field counts as read if its name appears anywhere in `src/` beyond its own
//! declaration. That is a coarse signal: it proves a field is referenced, not
//! that the reference is correct. It is still exactly the signal that was
//! missing, and it costs nothing to keep.
//!
//! **Comments are stripped before counting.** They were not, and it mattered:
//! writing "the aura is for the workers" in an unrelated doc comment was
//! enough to make `SpecialData::aura` — genuinely dead — look alive and fail
//! the stale-allowance check. Prose about a feature is not an implementation
//! of it.
//!
//! Run with: cargo test --test live_data_fields_tests

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Fields parsed but not yet consumed, as `Struct::field`.
///
/// Every entry is a real gap rather than an oversight — the subsystem that
/// would read it does not exist yet — and each maps to a named item in
/// TODO.md. **Shrink this list; do not grow it.** Adding a name here is a
/// decision to ship data that does nothing, and should be made deliberately.
const UNCONSUMED: &[&str] = &[
    // Per-room wall art is authored but never generated or drawn.
    "RoomVisualData::wall_sprite",
    // Lighting and atmosphere pass.
    "LightEffect::flicker",
    // Tiles: durability and fog art are still presentation/authoring hooks.
    "TileData::durability",
    "VisualData::fogged_sprite",
    "VisualData::animated",
    // Traps: every shipped trigger is pressure, so the trigger vocabulary is
    // reserved until a second trigger semantics exists.
    "TrapEffects::trigger_type",
    // Hero behaviour model, the part still unreachable. `trap_awareness`,
    // `fear_resistance`, `will_fight_to_death`, `bravery` and now
    // `light_preference` are wired. `door_break_chance` needs heroes to attack
    // doors at all, and `call_for_aid` needs a hero rally mechanic.
    "BehaviorData::door_break_chance",
    "ThreatResponse::call_for_aid",
    "HeroProgressionData::level_range",
    "HeroProgressionData::elite_variants",
    // Config knobs with no consumer.
    "CombatConfig::creature_health_per_level",
    "CombatConfig::hero_health_per_level",
    "CombatConfig::counterattack_death_chance",
    "CombatConfig::counterattack_level_threshold",
    "CombatConfig::xp_requirement_base",
    "CreatureAIConfig::mood_attention_threshold",
    "CreatureAIConfig::need_attention_threshold",
    "CreatureAIConfig::slap_cooldown",
    "CreatureAIConfig::base_mood_efficiency",
    "SpawningConfig::monster_spawner_interval",
    "TimingConfig::initial_creature_spawn_delay",
];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `(struct name, field name)` for every `pub struct` under `src/data/`.
fn declared_fields() -> Vec<(String, String)> {
    let data_dir = project_root().join("src").join("data");
    let mut files: Vec<PathBuf> = Vec::new();
    collect_rs(&data_dir, &mut files);
    files.sort();

    let mut out = Vec::new();
    for path in files {
        let source = std::fs::read_to_string(&path).expect("read data source");
        let mut rest = source.as_str();
        while let Some(start) = rest.find("pub struct ") {
            let after = &rest[start + "pub struct ".len()..];
            let Some(brace) = after.find('{') else { break };
            let name = after[..brace].trim().to_string();
            let body = &after[brace..];
            // Structs are formatted with the closing brace in column 0.
            let end = body.find("\n}").unwrap_or(body.len());
            for line in body[..end].lines() {
                let line = line.trim();
                let Some(decl) = line.strip_prefix("pub ") else {
                    continue;
                };
                let Some(field) = decl.split(':').next() else {
                    continue;
                };
                let field = field.trim();
                if !field.is_empty()
                    && field
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                {
                    out.push((name.clone(), field.to_string()));
                }
            }
            rest = &body[end.min(body.len())..];
        }
    }
    out
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// The whole of `src/`, concatenated.
fn all_source() -> String {
    let mut files = Vec::new();
    collect_rs(&project_root().join("src"), &mut files);
    files.sort();
    files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("read source"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Blank out `//` line comments and `/* */` block comments so prose cannot
/// keep a dead field alive. Replaces them with spaces rather than deleting, so
/// nothing on either side is accidentally joined into one token.
fn strip_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(' ');
                i += 1;
            }
        } else if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            out.push_str("  ");
            i += 2;
            while i < bytes.len()
                && !(bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/')
            {
                out.push(if bytes[i] == b'\n' { '\n' } else { ' ' });
                i += 1;
            }
            if i < bytes.len() {
                out.push_str("  ");
                i += 2;
            }
        } else {
            out.push(source[i..].chars().next().unwrap_or(' '));
            i += source[i..].chars().next().map_or(1, |c| c.len_utf8());
        }
    }
    out
}

fn occurrences(source: &str, field: &str) -> usize {
    let bytes = source.as_bytes();
    let mut count = 0;
    let mut from = 0;
    while let Some(idx) = source[from..].find(field) {
        let at = from + idx;
        let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
        let after = at + field.len();
        let after_ok = after >= bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            count += 1;
        }
        from = at + field.len();
    }
    count
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[test]
fn every_data_field_is_read_somewhere() {
    let fields = declared_fields();
    assert!(
        fields.len() > 100,
        "expected to sweep the whole of src/data, parsed only {} fields",
        fields.len()
    );

    let source = strip_comments(&all_source());
    let allowed: BTreeSet<&str> = UNCONSUMED.iter().copied().collect();

    let mut dead = Vec::new();
    let mut stale_allowance = Vec::new();

    for (struct_name, field) in &fields {
        let qualified = format!("{struct_name}::{field}");
        let read = occurrences(&source, field) > 1;
        match (read, allowed.contains(qualified.as_str())) {
            (false, false) => dead.push(qualified),
            (true, true) => stale_allowance.push(qualified),
            _ => {}
        }
    }

    let mut problems = Vec::new();
    for name in dead {
        problems.push(format!(
            "{name} is parsed but read nowhere. Wire it up, delete it (CODE_STANDARDS.md \
on unused code), or add it to UNCONSUMED with a reason."
        ));
    }
    for name in stale_allowance {
        problems.push(format!(
            "{name} is listed in UNCONSUMED but is now read. Drop it from the list."
        ));
    }

    assert!(problems.is_empty(), "\n  {}", problems.join("\n  "));
}

#[test]
fn unconsumed_entries_name_fields_that_exist() {
    // Stops the allowlist outliving the fields it excuses.
    let declared: BTreeSet<String> = declared_fields()
        .into_iter()
        .map(|(s, f)| format!("{s}::{f}"))
        .collect();

    let stale: Vec<&&str> = UNCONSUMED
        .iter()
        .filter(|name| !declared.contains(**name))
        .collect();

    assert!(
        stale.is_empty(),
        "UNCONSUMED names fields that no longer exist: {stale:?}"
    );
}
