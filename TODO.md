# TODO — Deep Dominion

## Standards and architecture

- [ ] Expose testable game and graphics-generation logic through `src/lib.rs`; make the binaries use it and migrate all tests/helpers from `src/` and `graphics_gen/` into `tests/`. Preserve private internals behind intentional public APIs (§11.4).
- [ ] Review migrated suites by feature, consolidating related inputs toward five tests without losing regression coverage; explain justified exceptions. Start with map loading, lighting, combat, saves, and room placement (§11.3).
- [ ] Split cohesive responsibilities before expanding `engine/hero_ai.rs` (792 lines), `engine/creature_ai.rs` (781), and `graphics_gen/tiles/rooms.rs` (789). Migrate affected legacy `mod.rs` roots to named files and retain the existing source gate with an empty exception list (§2.2–2.3).
- [ ] Remove blanket dead-code/unused-import suppressions in `src/main.rs`, `graphics_gen/mod.rs`, and the balance calculator; remove unused arguments such as tutorial `_game_data` and sidebar `_current_mode`. Fix warnings and document narrowly justified Clippy allowances (§1.4, §10.2).
- [ ] Refactor `engine/input/playing.rs::handle_playing` into focused handlers below 100 lines; replace long argument lists with explicit interaction context. Give renderer entry points read-only game/mode references and route gameplay input through the existing action dispatcher (§4, §5.1, §7).
- [ ] Externalize tutorial steps, targets, UI/notification strings, imp claim delay, and hero ability thresholds into typed JSON under `assets/`, loaded through toolkit APIs. Add project-owned validation for IDs, references, finite balance values, and supported effects/triggers at startup; report invalid content clearly (§5.3, §6).
- [ ] Replace duplicated balance-test schemas/loaders with the public game-data API, and convert remaining calculator-only simulation assertions into behavior tests run by existing CI (§5.3, §11).
- [ ] Strengthen `tests/live_data_fields_tests.rs`: exclude test-only reads and distinguish struct fields with common names. Reconcile `UNCONSUMED` against actual consumers, wiring or removing inert room/tile/trap/hero/config fields rather than preserving misleading allowances (§1.4).
- [ ] Cache save availability on menu entry and refresh after save/load/delete; remove per-frame `any_save_exists()` calls from menus and sidebar rendering.
- [ ] Harden fallible startup/resource paths with visible retry or recovery actions; replace unchecked float comparisons in sidebar research sorting and hero targeting with validated values or safe ordering (§6).
- [ ] Isolate simulation randomness in state-owned RNG or small helpers; separate procedural graphics randomness and add seed-repeatability/save-resume regression coverage. Deduplicate AI distance/movement helpers where behavior matches.
- [ ] Profile large-map entity-position scans, pathfinding, room discovery, threat calculation, sidebar layout, and light-map rebuilding; add spatial indexing or invalidation-based caches where measurements justify them.
- [ ] Add missing module-purpose comments during these refactors; correct `docs/gdd.md`'s Bevy ECS claim and align README scope with campaign gameplay (§9).

## Touch controls and onboarding

- [ ] Add visible controls/gestures for camera pan, rotation, zoom, cancel/unmark/slap, and return-to-menu after defeat or final victory; these currently depend on keyboard, wheel, or right-click input in `engine/input/playing.rs` and `tile_actions.rs` (§7.5).
- [ ] Replace hand-rolled menu/sidebar press hit-tests with toolkit release buttons where immediate activation is unnecessary. Share layout between drawing and input, and verify controls/tutorial overlays fit narrow browser windows and supported text scales (§7.4–7.5).
- [ ] Give the intro an explicit visible Begin control; update tutorial, menu, README, and `game_page.json` shortcut instructions to name exact tap controls. Extend tutorial progression through combat, traps, wages/moods, research, and surviving the first hero wave; add later contextual guidance for spells, prison/torture, and temple use (§7.5).
- [ ] Verify one full mission from start through recovery using touch controls, plus desktop layouts; replace matching captures directly in `docs/verification/` and run parameterless `publish.ps1` after implementation (§8.3, §12).

## Gameplay and authored content

- [ ] Implement remaining monster ability hooks for `charge`, `smash`, `berserk`, and `charm`; make combat speed-modifier application and expiry symmetric, including projectile impacts.
- [ ] Implement the `polymorph` effect used by `chickenify`, and the missing ritual/corruption/stealth/trap triggers behind dispel, purify, backstab, teleport, and mass_cleanse; validate unsupported authoring instead of silently accepting inert abilities.
- [ ] Connect `SpecialData::triggers_event` to scenario events and add a conversion-count objective trigger so authored conversion goals can complete.
- [ ] Add room efficiency rules for adjacency, shape, and doors; support creature contributions to ritual output. Resolve construction-time scale before adding per-tile build progress, or remove that unused field.
- [ ] Complete defensive interactions: imp trap rearming, magical door locking, alarm responses, player-directed wall reinforcement, and terrain damage for wall-breaking creatures. Reconcile Gatehouse behavior with `docs/ROOM_SET.md`.
- [ ] Implement authored mana upkeep for Ironbound/Balor and environmental terrain damage/movement effects; ensure their descriptions and balance values match actual behavior.
- [ ] Extend rival keepers with paid, timed digging/building/reinforcement and defensive/research actions; support multiple rivals. Specify formation behavior and ranged-combat gaps before extending combat AI.
- [ ] Resolve sleep/kennel population-cap rules, give eating and gold-deposit rooms distinct task semantics, and make authored recovery rates reachable. Add a comfort need so creatures deliberately seek amenities.
- [ ] Resolve missing Mentor's Den/Doctrine Chamber designs referenced through absent `docs/rooms.md`; define per-creature door/trap bypass before adding Assassin Wisp. Add species rivalry/brawl behavior if retained in the roster design.
- [ ] Make mutation progress and results inspectable in creature UI, including contributing rooms; add transformation feedback.
- [ ] Add map quality checks and regenerate poor layouts; stage large-map generation across frames. Verify scouting visibility across terrain, entities, and minimap, including the reported residual rival-lair leak.
- [ ] Extend hand interactions to gold/objects. Document concrete build-or-cut scope for possession, surface raids, meta-progression, trade/hiring, creature corpses, spell miscasts/counters/modifiers, and gamepad navigation.
- [ ] Reconcile campaign technology rewards, temple availability, undead acquisition, and routes for assaulting the hero base with authored progression. Document those rules in project design guidance.
- [ ] Define supported content-pack behavior and provide a validated example and authoring instructions, or remove the unused public-facing mod surface.

## Presentation, settings, and persistence

- [ ] Integrate the existing toolkit audio manager for dungeon SFX, music, raid/outcome transitions, and persisted volume controls; add shared mixer/panning/ducking capabilities only where missing. Produce an initial usable sound set with documented asset provenance and verify browser playback.
- [ ] Add sprite animation and event feedback for combat, death, spells, traps, digging, gold, and heart damage, using shared toolkit effects where suitable; provide reduced-motion controls.
- [ ] Add smooth authored light flicker and resolve room-wall sprite generation/rendering or remove unused wall-art fields. Review palette consistency, menu/sidebar art, and replace the minimap's fixed 20×20 viewport approximation with camera-derived bounds.
- [ ] Extend settings with key remapping, camera options, autosave enable/disable, display options, scalable text, colorblind-safe factions, and hold/toggle controls; persist values through existing toolkit settings.
- [ ] Add save deletion and quicksave/quickload with visible controls; use toolkit version inspection/migration hooks to handle incompatible and older save formats explicitly.

## Balance verification

- [ ] Run reproducible mission playtests covering first-wave preparation, starting gold, sustainable army size, and wave-10+ survival across difficulties. Tune wave pacing, level asymmetry, wages/needs, gem yield, sell refunds, and creature cost/mood outliers from recorded results.
- [ ] Evaluate stacked hero-building destruction penalties and widespread fight-to-death authoring during those runs; update data and regression expectations to match the intended difficulty curve.
