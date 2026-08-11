use super::*;

#[test]
fn parses_common_owner_labels() {
    assert_eq!("player".parse::<OwnerId>().unwrap(), OwnerId::Player);
    assert_eq!(
        "keeper2".parse::<OwnerId>().unwrap(),
        OwnerId::RivalKeeper(2)
    );
    assert_eq!("heroes".parse::<OwnerId>().unwrap(), OwnerId::Heroes);
    assert_eq!("surface".parse::<OwnerId>().unwrap(), OwnerId::AboveGround);
}

#[test]
fn hostility_uses_owner_not_creature_type() {
    assert!(OwnerId::Player.is_hostile_to(&OwnerId::Heroes));
    assert!(OwnerId::RivalKeeper(1).is_hostile_to(&OwnerId::Player));
    assert!(!OwnerId::RivalKeeper(1).is_hostile_to(&OwnerId::RivalKeeper(1)));
    assert!(!OwnerId::Neutral.is_hostile_to(&OwnerId::Player));
}
