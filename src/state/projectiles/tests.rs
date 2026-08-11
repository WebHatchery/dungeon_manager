use super::*;

#[test]
fn current_format_round_trips() {
    let mut manager = ProjectileManager::new();
    manager.spawn((0.0, 0.0), (5.0, 5.0), "ranged", 1, 2, 12.5);

    let json = serde_json::to_string(&manager).expect("should serialize");
    let mut restored: ProjectileManager = serde_json::from_str(&json).expect("should deserialize");

    let impacts = restored.update(1.0);
    assert_eq!(impacts.len(), 1);
    assert_eq!(impacts[0].attacker_id, 1);
    assert_eq!(impacts[0].defender_id, 2);
    assert!((impacts[0].damage - 12.5).abs() < f32::EPSILON);
}

#[test]
fn legacy_save_format_loads_as_empty() {
    // Pre-toolkit shape: per-projectile start_pos/end_pos/etc. fields.
    let legacy = r#"{"projectiles":[{
        "start_pos":[0.0,0.0],"end_pos":[3.0,4.0],
        "projectile_type":"Arrow","attacker_id":7,"defender_id":9,
        "damage":5.0,"progress":0.5,"elapsed":0.15,"duration":0.3
    }]}"#;
    let manager: ProjectileManager =
        serde_json::from_str(legacy).expect("legacy saves must still load");
    assert!(
        manager.active_projectiles().next().is_none(),
        "legacy in-flight projectiles are dropped, not migrated"
    );
}

#[test]
fn melee_projectile_stays_near_attacker() {
    let mut manager = ProjectileManager::new();
    manager.spawn((0.0, 0.0), (10.0, 0.0), "melee", 1, 2, 1.0);

    // Advance almost to the end of the melee duration (0.15s)
    manager.update(0.14);
    let projectile = manager
        .active_projectiles()
        .next()
        .expect("projectile still in flight");
    assert!(
        projectile.position().x < 3.5,
        "melee travel should be capped at 30% of the distance"
    );
}
