//! Build-time-generated manifest of embedded campaign/scenario/map JSON.
//!
//! `build.rs` scans `assets/campaigns/`, `assets/scenarios/`, and
//! `assets/maps/` and writes `EMBEDDED_CAMPAIGNS` / `EMBEDDED_SCENARIOS` (each
//! a `&[&str]` of file contents) and `EMBEDDED_MAPS` (a `&[(&str, &str)]` of
//! filename/content pairs, since maps are looked up by path rather than
//! merged by id). This is the "WASM embedded manifest": on `wasm32` there is
//! no filesystem, so these arrays are the only content source. On native
//! builds the toolkit registry overlays a runtime directory scan on top for
//! campaigns/scenarios, and `state::map_loader` falls back to `EMBEDDED_MAPS`
//! when a requested map path isn't found on disk.

include!(concat!(env!("OUT_DIR"), "/embedded_content.rs"));
