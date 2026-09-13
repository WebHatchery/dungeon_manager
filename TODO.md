# TODO — Deep Dominion

Only the following work still requires an AI agent. Completed implementation
notes and obsolete design promises are intentionally not retained here.

## Architecture and verification

- [ ] Expose testable game and graphics-generation APIs through `src/lib.rs`, migrate source-local tests/helpers into `tests/`, and preserve private internals behind intentional APIs.
- [ ] Split the near-limit `hero_ai.rs`, `creature_ai.rs`, and `graphics_gen/tiles/rooms.rs` modules before adding more behavior; remove broad warning suppressions and replace remaining unchecked float ordering.
- [ ] Refactor `engine/input/playing.rs::handle_playing` into focused handlers and use toolkit release-button controls for menus/sidebar actions that do not need immediate activation.
- [ ] Replace duplicated balance-calculator schemas with the public game-data API, strengthen live-data coverage, and add reproducible behavior tests for remaining balance assumptions.
- [ ] Harden startup/resource recovery with visible retry paths; profile large-map scans, pathfinding, lighting, room discovery, and sidebar work before adding caches or spatial indexes.
- [ ] Isolate simulation randomness behind state-owned helpers and add seed-repeatability plus save/resume regression coverage.

## Touch and gameplay verification

- [ ] Verify one complete mission from briefing through recovery on touch and desktop layouts, capture the required replacement screenshots in `docs/verification/`, and keep the parameterless publish check green.
- [ ] Add player-directed wall reinforcement and reconcile Gatehouse behavior with `docs/ROOM_SET.md`.
- [ ] Resolve sleep/kennel population caps, distinct eating and gold-deposit task semantics, authored recovery rates, and a creature comfort need.
- [ ] Decide and implement the missing Mentor's Den/Doctrine Chamber designs, Assassin Wisp door/trap bypass, and species-rivalry behavior, or remove those roster promises from design docs and data.
- [ ] Make mutation progress/results inspectable in creature UI, including contributing rooms and transformation feedback.
- [ ] Add map-quality checks and staged large-map generation; verify scouting visibility across terrain, entities, and minimap, including the rival-lair leak.
- [ ] Extend hand interactions to gold/objects and document concrete build-or-cut scope for possession, surface raids, meta-progression, trade/hiring, corpses, spell modifiers, and gamepad navigation.
- [ ] Reconcile campaign technology rewards, temple/undead availability, and hero-base assault routes with authored progression and document the final rules.

## Presentation and balance

- [ ] Integrate toolkit audio with persisted volume groups, dungeon/raid/outcome feedback, browser playback, and documented sound provenance.
- [ ] Add sprite animation and event feedback for combat, death, spells, traps, digging, gold, and heart damage, with reduced-motion behavior.
- [ ] Add authored light flicker, resolve room-wall sprite generation/rendering, review palette/menu/sidebar art, and derive minimap bounds from the camera.
- [ ] Complete settings coverage for camera/display preferences, remapping, colorblind-safe factions, and hold/toggle controls using the existing toolkit settings.
- [ ] Run reproducible mission playtests across difficulties, then tune wave pacing, army sustainability, wages/needs, yields, refunds, stacked destruction penalties, and fight-to-death behavior from recorded results.
