use super::*;

#[test]
fn settings_roundtrip_serialization() {
    let settings = GameSettings {
        base: ToolkitSettings {
            fullscreen: true,
            ui_text_scale: 1.25,
            ..Default::default()
        },
        difficulty: Difficulty::Hard,
    };
    let json = serde_json::to_string(&settings).expect("settings should serialize");
    let loaded: GameSettings = serde_json::from_str(&json).expect("settings should parse");
    assert!(loaded.base.fullscreen);
    assert_eq!(loaded.base.ui_text_scale, 1.25);
    assert_eq!(loaded.difficulty, Difficulty::Hard);

    // Missing fields fall back to defaults for forward compatibility
    let empty: GameSettings = serde_json::from_str("{}").expect("empty settings should parse");
    assert!(!empty.base.fullscreen);
    assert_eq!(empty.base.ui_text_scale, 1.0);
    assert_eq!(empty.difficulty, Difficulty::Normal);
}

#[test]
fn difficulty_cycles_and_scales_threat() {
    assert_eq!(Difficulty::default(), Difficulty::Normal);
    assert_eq!(Difficulty::Normal.threat_scale(), 1.0);
    // Easy is gentler, Hard is harsher.
    assert!(Difficulty::Easy.threat_scale() < 1.0);
    assert!(Difficulty::Hard.threat_scale() > 1.0);
    // Cycles Easy → Normal → Hard → Easy.
    assert_eq!(Difficulty::Easy.next(), Difficulty::Normal);
    assert_eq!(Difficulty::Normal.next(), Difficulty::Hard);
    assert_eq!(Difficulty::Hard.next(), Difficulty::Easy);
}

#[test]
fn ui_scale_cycles_through_steps() {
    let mut settings = GameSettings::default();
    // Note: cycle_ui_text_scale() also applies + saves settings, so exercise
    // only the pure step selection here.
    let current = UI_SCALE_STEPS
        .iter()
        .position(|scale| (scale - settings.base.ui_text_scale).abs() < 0.01)
        .unwrap_or(1);
    settings.base.ui_text_scale = UI_SCALE_STEPS[(current + 1) % UI_SCALE_STEPS.len()];
    assert_eq!(settings.base.ui_text_scale, 1.25);
}
