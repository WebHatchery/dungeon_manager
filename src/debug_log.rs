//! Subsystem-tagged trace logging, silent unless asked for.
//!
//! The engine used to call `eprintln!` unconditionally on every combat hit,
//! every imp dig, every trap trigger and every spell effect, so a minute of
//! play buried anything actually worth reading — including, for a long while,
//! genuine warnings and messages the player should have been shown.
//!
//! Set `DUNGEON_MANAGER_LOG` to a comma-separated list of tags to bring a
//! subsystem back, or `all` for everything:
//!
//! ```text
//! DUNGEON_MANAGER_LOG=combat,traps cargo run
//! ```
//!
//! This is for *tracing*, not for problems. A genuine warning — a data error,
//! a lost resource, a failed load — should stay an unconditional `eprintln!`
//! so it is visible without knowing to ask. Anything the *player* needs is
//! neither: use `PlayerState::notify` / `warn_once`.

use std::collections::HashSet;
use std::sync::OnceLock;

const ENV_VAR: &str = "DUNGEON_MANAGER_LOG";

/// Split a `DUNGEON_MANAGER_LOG` value into tags.
///
/// Separate from the environment so it can be tested without one — the cached
/// `OnceLock` below reads the variable exactly once per process, which makes
/// env-var-based tests unreliable.
pub fn parse_tags(value: &str) -> HashSet<String> {
    value
        .split(',')
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect()
}

/// Whether `tag` should be traced, given a parsed tag set.
pub fn tags_allow(tags: &HashSet<String>, tag: &str) -> bool {
    tags.contains("all") || tags.contains(&tag.to_ascii_lowercase())
}

fn enabled_tags() -> &'static HashSet<String> {
    static TAGS: OnceLock<HashSet<String>> = OnceLock::new();
    // `env::var` fails on wasm, which leaves tracing off in the browser —
    // where nobody could set it anyway.
    TAGS.get_or_init(|| parse_tags(&std::env::var(ENV_VAR).unwrap_or_default()))
}

/// Whether `tag` is currently being traced.
pub fn tag_enabled(tag: &str) -> bool {
    tags_allow(enabled_tags(), tag)
}

/// Trace a message for one subsystem. Compiles to a cheap check when the tag
/// is off, and formats nothing.
#[macro_export]
macro_rules! trace_log {
    ($tag:literal, $($arg:tt)*) => {
        if $crate::debug_log::tag_enabled($tag) {
            eprintln!("[{}] {}", $tag, format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests;
