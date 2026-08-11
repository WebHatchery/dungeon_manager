use crate::data::GameData;

#[test]
fn new_battle_spells_are_wired_with_working_effects() {
    let game_data = GameData::load().expect("game data should load");
    // Effect types the spell dispatcher (`spell_effects::apply_spell_effect`)
    // actually executes — new spells must stick to these so they aren't inert.
    const SUPPORTED: [&str; 6] = [
        "damage",
        "heal",
        "stat_modifier",
        "status_apply",
        "spawn_entity",
        "reveal_map",
    ];
    for id in [
        "fireball",
        "frost_nova",
        "chain_lightning",
        "venom_spray",
        "raise_the_dead",
        "summon_skeletons",
    ] {
        let spell = game_data
            .spells
            .get(id)
            .unwrap_or_else(|| panic!("{id} should be in the spellbook"));
        assert!(!spell.effects.is_empty(), "{id} has no effects");
        for effect in &spell.effects {
            assert!(
                SUPPORTED.contains(&effect.effect_type.as_str()),
                "spell '{}' uses unsupported (inert) effect type '{}'",
                id,
                effect.effect_type
            );
        }
    }
    // The summon spells reference real creatures.
    for (spell_id, creature) in [
        ("raise_the_dead", "zombie"),
        ("summon_skeletons", "skeleton"),
    ] {
        assert!(game_data.monsters.contains_key(creature));
        let spell = &game_data.spells[spell_id];
        assert!(spell
            .effects
            .iter()
            .any(|e| e.entity.as_deref() == Some(creature)));
    }
}
