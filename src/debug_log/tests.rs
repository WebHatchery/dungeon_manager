use super::*;

#[test]
fn an_unset_variable_traces_nothing() {
    let tags = parse_tags("");
    assert!(!tags_allow(&tags, "combat"));
    assert!(!tags_allow(&tags, "all"));
}

#[test]
fn named_tags_are_traced_and_others_are_not() {
    let tags = parse_tags("combat,imps");
    assert!(tags_allow(&tags, "combat"));
    assert!(tags_allow(&tags, "imps"));
    assert!(!tags_allow(&tags, "spells"));
}

#[test]
fn all_traces_everything() {
    let tags = parse_tags("all");
    assert!(tags_allow(&tags, "combat"));
    assert!(tags_allow(&tags, "anything_at_all"));
}

#[test]
fn whitespace_and_case_are_forgiven() {
    // Someone typing this at a shell prompt should not have to be careful.
    let tags = parse_tags(" Combat , IMPS ,, ");
    assert!(tags_allow(&tags, "combat"));
    assert!(tags_allow(&tags, "Imps"));
    assert!(!tags.contains(""), "empty entries should be dropped");
}
