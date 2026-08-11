//! Manifest-driven loading for base-game campaigns and scenarios.
//!
//! Replaces the old one-file-per-loader `include_str!`. Content is sourced from
//! the build-time embedded manifest (`data::embedded`), and on native builds a
//! runtime directory scan is overlaid so a mission JSON dropped into
//! `assets/campaigns/` or `assets/scenarios/` loads without recompiling. Adding
//! a mission is therefore pure content work — no code changes.

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::error::Error;

/// Parse every embedded JSON blob (each a JSON array of `T`) and index by id.
/// Later blobs override earlier ones on id collision (deterministic: `build.rs`
/// emits files in sorted order).
pub fn from_embedded<T, F>(blobs: &[&str], id_of: F) -> Result<HashMap<String, T>, Box<dyn Error>>
where
    T: DeserializeOwned,
    F: Fn(&T) -> String,
{
    let mut map = HashMap::new();
    for blob in blobs {
        let items: Vec<T> = serde_json::from_str(blob)?;
        for item in items {
            map.insert(id_of(&item), item);
        }
    }
    Ok(map)
}

/// Native only: overlay a runtime scan of `<rel>` (e.g. `"assets/scenarios"`)
/// on top of the embedded set, so freshly-authored files are picked up without
/// a rebuild. The first candidate directory that exists wins; a malformed file
/// is skipped rather than aborting the load. No-op on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub fn overlay_from_disk<T, F>(map: &mut HashMap<String, T>, rel: &str, id_of: F)
where
    T: DeserializeOwned,
    F: Fn(&T) -> String,
{
    use std::path::PathBuf;

    for dir in candidate_dirs(rel) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            match serde_json::from_str::<Vec<T>>(&text) {
                Ok(items) => {
                    for item in items {
                        map.insert(id_of(&item), item);
                    }
                }
                Err(err) => {
                    eprintln!("[content] skipping malformed {}: {err}", path.display());
                }
            }
        }
        // Only the first existing directory is authoritative.
        return;
    }
}

/// Candidate directories to scan at runtime, most-preferred first: the source
/// tree (dev/test, baked in at compile time) then a directory beside the
/// executable (a shipped native build with assets alongside the binary).
#[cfg(not(target_arch = "wasm32"))]
fn candidate_dirs(rel: &str) -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;

    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join(rel));
        }
    }
    dirs
}

#[cfg(test)]
mod tests;
