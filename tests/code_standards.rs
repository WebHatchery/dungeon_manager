// The shared file-size gate from CODE_STANDARDS §2.2 — the 800-line hard
// limit on non-test lines — enforced under plain `cargo test`.

#[test]
fn source_files_stay_under_the_limit() {
    // Over the limit when the gate arrived; the gate fails the moment an
    // entry drops back under 800 so this list can only shrink.
    macroquad_toolkit::source_gate::assert_source_files_within_limit(
        env!("CARGO_MANIFEST_DIR"),
        &[
            "src/state/game_state.rs",
            "src/engine/combat.rs",
            "src/engine/imp_ai.rs",
        ],
    );
}
