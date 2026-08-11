use super::*;

#[test]
fn cache_registers_configs_for_all_unit_types() {
    let cache = build_variation_cache();
    for unit in [
        "wizard", "warlock", "knight", "goblin", "orc", "skeleton", "imp", "troll", "archer",
        "vampire", "spider",
    ] {
        let config = cache.config_for(unit);
        assert!(
            config.color_regions.iter().all(|r| r.name != "primary"),
            "{unit} should use a tailored config, not the fallback"
        );
    }
    // Unregistered ids fall back to the generic two-region config
    assert_eq!(cache.config_for("unknown").color_regions[0].name, "primary");
}
