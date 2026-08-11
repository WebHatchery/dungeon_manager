use super::*;

#[derive(Debug, Clone, Deserialize)]
struct TestItem {
    id: String,
    value: i32,
}

impl Identified for TestItem {
    fn id(&self) -> &str {
        &self.id
    }
}

#[test]
fn merge_reports_added_and_replaced_ids() {
    let mut target = HashMap::new();
    target.insert(
        "room_a".to_string(),
        TestItem {
            id: "room_a".to_string(),
            value: 1,
        },
    );

    let report = merge_items(
        &mut target,
        vec![
            TestItem {
                id: "room_a".to_string(),
                value: 2,
            },
            TestItem {
                id: "room_b".to_string(),
                value: 3,
            },
        ],
    );

    assert_eq!(target["room_a"].value, 2);
    assert_eq!(target["room_b"].value, 3);
    assert_eq!(report.replaced_ids, vec!["room_a"]);
    assert_eq!(report.added_ids, vec!["room_b"]);
}

#[test]
fn load_order_resolves_pack_manifests() {
    let root =
        std::env::temp_dir().join(format!("dungeon_manager_mod_test_{}", std::process::id()));
    let pack_root = root.join("test_pack");
    std::fs::create_dir_all(&pack_root).unwrap();
    std::fs::write(
        pack_root.join("pack.json"),
        r#"{
          "id": "test_pack",
          "name": "Test Pack",
          "priority": 10
        }"#,
    )
    .unwrap();

    let order = ContentPackLoadOrder {
        packs: vec!["test_pack".to_string()],
    };

    let manifests = load_manifests_from_order(&root, &order).unwrap();
    assert_eq!(manifests[0].1.id, "test_pack");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn load_order_later_pack_overrides_earlier_data_and_assets() {
    let root = std::env::temp_dir().join(format!(
        "dungeon_manager_mod_override_test_{}",
        std::process::id()
    ));
    let first_root = root.join("first");
    let second_root = root.join("second");
    std::fs::create_dir_all(first_root.join("data")).unwrap();
    std::fs::create_dir_all(second_root.join("data")).unwrap();
    std::fs::create_dir_all(second_root.join("maps")).unwrap();
    std::fs::create_dir_all(second_root.join("assets/tiles")).unwrap();

    std::fs::write(
        first_root.join("pack.json"),
        r#"{
          "id": "first",
          "name": "First",
          "priority": 100,
          "data": { "campaigns": ["data/campaigns.json"] }
        }"#,
    )
    .unwrap();
    std::fs::write(
        first_root.join("data/campaigns.json"),
        r#"[{ "id": "override_campaign", "name": "First Campaign" }]"#,
    )
    .unwrap();

    std::fs::write(
        second_root.join("pack.json"),
        r#"{
          "id": "second",
          "name": "Second",
          "priority": 0,
          "assets": ["assets"],
          "maps": ["maps"],
          "data": { "campaigns": ["data/campaigns.json"] }
        }"#,
    )
    .unwrap();
    std::fs::write(
        second_root.join("data/campaigns.json"),
        r#"[{ "id": "override_campaign", "name": "Second Campaign" }]"#,
    )
    .unwrap();
    std::fs::write(second_root.join("maps/level_1.json"), "{}").unwrap();
    std::fs::write(second_root.join("assets/tiles/lair.png"), b"").unwrap();

    let order = ContentPackLoadOrder {
        packs: vec!["first".to_string(), "second".to_string()],
    };
    let manifests = load_manifests_from_order(&root, &order).unwrap();
    let mut data = GameData::load().unwrap();

    let reports = apply_ordered_content_packs(&mut data, &manifests).unwrap();

    assert_eq!(
        reports
            .iter()
            .map(|r| r.pack_id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert_eq!(data.campaigns["override_campaign"].name, "Second Campaign");
    assert_eq!(
        data.resolve_map_path("assets/maps/level_1.json"),
        second_root.join("maps/level_1.json")
    );
    assert!(data
        .resolve_asset_path("assets/tiles/lair.png")
        .ends_with("second/assets/tiles/lair.png"));

    let _ = std::fs::remove_dir_all(root);
}
