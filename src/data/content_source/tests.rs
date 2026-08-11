use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Item {
    id: String,
    value: i32,
}

#[test]
fn from_embedded_merges_multiple_blobs_and_later_wins() {
    // Two "files", one overriding an id from the other — mirrors dropping a
    // second scenario/campaign file into the manifest.
    let blobs: &[&str] = &[
        r#"[{"id":"a","value":1},{"id":"b","value":2}]"#,
        r#"[{"id":"b","value":99},{"id":"c","value":3}]"#,
    ];
    let map = from_embedded(blobs, |item: &Item| item.id.clone()).unwrap();

    assert_eq!(map.len(), 3);
    assert_eq!(map["a"].value, 1);
    assert_eq!(map["b"].value, 99); // later blob overrode the earlier one
    assert_eq!(map["c"].value, 3);
}

#[test]
fn from_embedded_propagates_parse_errors() {
    let blobs: &[&str] = &["not json"];
    assert!(from_embedded(blobs, |item: &Item| item.id.clone()).is_err());
}
