use crate::data::GameData;

#[test]
fn tech_tree_is_a_real_graph_of_valid_unlocks() {
    let game_data = GameData::load().expect("game data should load");
    let techs = &game_data.technologies;
    // A "real tree", not a 4-tech stub.
    assert!(techs.len() >= 10, "expected an expanded tech tree");

    for tech in techs.values() {
        // Prerequisites must reference real techs...
        for req in &tech.prerequisites {
            assert!(
                techs.contains_key(req),
                "tech '{}' requires unknown tech '{}'",
                tech.id,
                req
            );
        }
        // ...and every unlock must reference real content.
        for room in &tech.unlocks.rooms {
            assert!(
                game_data.rooms.contains_key(room),
                "tech '{}' unlocks unknown room '{}'",
                tech.id,
                room
            );
        }
        for spell in &tech.unlocks.spells {
            assert!(
                game_data.spells.contains_key(spell),
                "tech '{}' unlocks unknown spell '{}'",
                tech.id,
                spell
            );
        }
        for creature in &tech.unlocks.creatures {
            assert!(
                game_data.monsters.contains_key(creature),
                "tech '{}' unlocks unknown creature '{}'",
                tech.id,
                creature
            );
        }
        for trap in &tech.unlocks.traps {
            assert!(
                game_data.traps.contains_key(trap),
                "tech '{}' unlocks unknown trap '{}'",
                tech.id,
                trap
            );
        }
    }

    // The prerequisite graph is acyclic and rooted: every tech's prereq
    // chain terminates at a prerequisite-free root (so the tree is
    // researchable from scratch, no dependency cycles).
    fn reaches_root<'a>(
        id: &'a str,
        techs: &'a std::collections::HashMap<String, crate::data::TechData>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> bool {
        let Some(tech) = techs.get(id) else {
            return false;
        };
        if tech.prerequisites.is_empty() {
            return true;
        }
        if !visiting.insert(id.to_string()) {
            return false; // cycle
        }
        let ok = tech
            .prerequisites
            .iter()
            .all(|req| reaches_root(req, techs, visiting));
        visiting.remove(id);
        ok
    }
    for id in techs.keys() {
        assert!(
            reaches_root(id, techs, &mut std::collections::HashSet::new()),
            "tech '{id}' has no acyclic path to a root tech"
        );
    }
}
