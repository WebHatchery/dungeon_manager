# Deep Dominion — Campaign Arc Design

*Satisfies COMMERCIAL_ROADMAP.md §1 "Design a full campaign arc". This is the **design** deliverable
(mission list, difficulty curve, mechanic-introduction order, narrative framing/briefings, unlock
graph). Authoring each mission's map + scenario JSON + event scripting is the separate follow-on
item and is deliberately out of scope here.*

*Everything below is grounded in content that already exists in the codebase — no mechanic, creature,
room, spell, trap, hero, objective type, trigger, or event action is referenced unless it is already
implemented. Where the arc leans on a system the roadmap lists as shallow (multi-rival economy, boss
encounters, branch/hub unlock graph), that dependency is called out inline so mission authoring and
the engine work can be sequenced together.*

---

## 1. Design goals

1. **Minimum-viable-but-complete: 12 missions.** Above the genre's ~10 floor, below the 15–20 ideal.
   Twelve gives one clean teaching mission per major subsystem plus a three-mission climax, and fits
   the "2–3 polished missions → demo, full arc → 1.0" release plan in §8.
2. **One new idea per mission.** Each mission introduces exactly one headline mechanic (a room family,
   a creature archetype, a spell/trap category, or a hero threat type) and reinforces the previous
   two. Nothing is introduced in a mission where it is also load-bearing for victory.
3. **The curve is authored, not emergent.** Difficulty is expressed through five authored dials that
   already exist in the scenario schema — `start_gold`, `max_creatures`, `threat_multiplier`,
   hero-party composition/level, and rival-keeper aggression (`first_attack_delay`,
   `attack_cooldown`, `attack_cooldown_growth`, `raid_size`) — never through hidden fudge factors.
4. **Persistent unlocks are the meta-progression.** The campaign's `persistent_unlocks` block grows
   as missions complete, so the roster available at mission *N* is the union of everything unlocked in
   missions `1..N` plus that mission's own scenario grants. This exercises the `required_completed` /
   `unlocks_after` graph the roadmap flags as never-branched (§1) — see §6.
5. **Data-only.** With the manifest-driven loader (roadmap §1, `data/campaign.rs:128` /
   `data/scenario.rs:350`), the entire arc ships as JSON under `assets/campaigns/` +
   `assets/scenarios/` + `assets/maps/`. No Rust changes are required to add or reorder missions once
   the loader lands.

---

## 2. Content inventory the arc draws from

Authored and available today (the arc introduces these on the schedule in §4; it never invents new
content — new *content* is roadmap §1 roster work, tracked separately):

| Category | Available today |
|----------|-----------------|
| **Creatures** (12 + imp worker) | imp, goblin, orc, warlock, troll, skeleton, demon_spawn, spider, lizard, succubus, vampire, hellhound, bile_demon |
| **Rooms** (15) | lair, hatchery, treasury, library, workshop, training_hall, prison, guard_post, ritual_circle, torture_chamber, graveyard, kennel, barracks, temple, scavenger |
| **Traps** (7) | door, braced_door, magic_door, spike_trap, boulder_trap, alarm_trap, blowgun_trap |
| **Spells** (11) | summon_imps, lightning_strike, heal, possess, call_to_arms, corrupt_land, reveal_map, make_earth, speed_boost, iron_skin, chickenify |
| **Technologies** (4) | training_tech, prison_tech, ritual_tech, trap_tech |
| **Heroes** (17) | peasant_militia, scout, acolyte, knight, archer, battle_cleric, rogue, paladin, wizard, inquisitor, knight_commander, high_priest, archmage, champion_of_light, barbarian, alchemist, geomancer |
| **Hero buildings** (9) | town_hall, barracks, archery_range, church, mage_tower, stable, armory, hero_wall, hero_gate |

**Engine primitives the events lean on** (all present in `data/scenario.rs`):
- Objective types: `survive_time`, `destroy_heart`, `destroy_all_hero_buildings`, `gather_resource`, `convert_heroes`, `sacrifice_creatures`, `custom`.
- Triggers: `time_elapsed`, `objective_complete`, `room_claimed`, `action_point_reached`, `dungeon_breached`.
- Actions: `unlock_room`, `unlock_spell`, `unlock_trap`, `spawn_creature`, `spawn_hero_party`, `set_rule`, `complete_objective`.
- Hero-party behaviors: `attack_dungeon_heart`, `defend_location`, `steal_gold`, `explore`.

Only **4 technologies** exist, so the campaign's "research gates" reuse those four repeatedly rather
than implying a deep tech tree. Growing the tech tree is roadmap §1 (`player_state.rs:240`) and would
let missions 7–12 gate more content behind research; the arc is authored so that upgrade slots in
without reshaping the mission list.

---

## 3. The narrative spine

**Premise (from the GDD / existing `dark_beginnings` intro):** a newly-woken Dungeon Heart beneath
the Sunlit Kingdoms grows from a single hidden chamber into a realm-spanning dark empire, ending by
toppling the Kingdoms' capital and its Champion of Light.

Three acts:

- **Act I — Roots (M1–M4): "Carve a foothold."** Survive, learn the economy, take the first prisoners.
  Tone: cramped, scarce, reactive. Antagonist: a local hero outpost and one weak rival keeper.
- **Act II — Reach (M5–M8): "Turn the depths against the surface."** Rituals, the dead, and demons.
  The player stops merely surviving raids and starts fielding a real army. Antagonist: an organized
  hero order (paladins/clerics) and a competent rival keeper; the branch point lives here.
- **Act III — Dominion (M9–M12): "Break the Sunlit Kingdoms."** Offense: raze hero towns, out-expand
  a rival, and defeat the named boss heroes at the capital. Tone: overwhelming force, then a knife-edge
  final defense.

Each mission ships a `briefing` (pre-mission, in the campaign JSON) and an `intro[]` block (the
`meta.intro` flavor already used by `dark_beginnings`). A short debrief line per mission is authored
for the between-mission screen (roadmap §1 "between-mission screens").

---

## 4. Mission-by-mission design

Legend: **New** = the single headline mechanic taught. **Adds** = roster/tools unlocked (persist
forward). **Win/Lose** = objective mapping to real `ScenarioObjective` variants. **Pressure** = the
authored hero/rival difficulty for this slot.

### Act I — Roots

**M1 · Dark Beginnings** *(exists — `dark_beginnings.json`, use as-is / lightly tuned)*
- **New:** dig → claim → build the core loop (lair, hatchery, treasury); attract & recruit (goblin); the slap.
- **Adds:** training_hall, spike_trap (mid-mission event grant), orc (as a preview unlock).
- **Win:** `survive_time` 600s **then** `destroy_all_hero_buildings`. **Lose:** `destroy_heart` (player).
- **Pressure:** one 4-hero raid at t+300s (`first_raid`); one `builder_defender` rival, first attack t+240s. Baseline `threat_multiplier` 1.0.
- **Briefing:** *"Your heart beats for the first time. Carve your first chambers, raise a fighting force, and break the surface outpost before the Kingdoms notice the dark stirring beneath them."*

**M2 · Blood and Iron**
- **New:** training & melee depth — the training_hall economy, guard_post chokepoints, `door`/`braced_door` placement. Combat becomes something you *prepare for*, not just survive.
- **Adds:** orc (usable), blowgun_trap, `training_tech` + `trap_tech` researched from the start.
- **Win:** `survive_time` (longer) + `destroy_all_hero_buildings`. **Lose:** heart destroyed.
- **Pressure:** two escalating raids (peasant_militia + acolyte, then + archer). Rival raids sooner and larger (`raid_size` up, `attack_cooldown` down). `threat_multiplier` ~1.1.
- **Briefing:** *"An armed force is worth ten hungry mouths. Drill your creatures, wall your approaches, and let the next patrol break itself on your doors."*

**M3 · The Long Dark** *(economy mission)*
- **New:** the gold/research economy — library (research), workshop (trap manufacture / imp rearming), treasury scaling, and gold scarcity as the real constraint (ties to the "starting gold is too scarce" feedback in §6-balance; this mission is where scarcity is *intended*).
- **Adds:** library, workshop, boulder_trap, `trap_tech`.
- **Win:** `gather_resource` (gold, a target amount) + `survive_time`. **Lose:** heart destroyed.
- **Pressure:** raids are modest; the challenge is throughput, not bodies. Rival competes for the same gold seams (design hook for the "gems as infinite-but-slower gold source" fix, roadmap §2 line 166).
- **Briefing:** *"Steel rusts; gold endures. Sink shafts to the deep seams, turn your workshops to war, and out-earn the rival festering to the south — starve him and he falls without a fight."*

**M4 · No Prisoners** *(prison / conversion mission)*
- **New:** the prison → skeleton pipeline and torture_chamber → defection; capturing routed heroes instead of killing them. Introduces the "don't kill, *harvest*" strategic layer.
- **Adds:** prison, torture_chamber, skeleton (via conversion), `prison_tech`.
- **Win:** `custom` "convert N heroes" (a `custom` objective completed by an `objective_complete`/event that counts prison→skeleton conversions) + `destroy_all_hero_buildings`. **Lose:** heart destroyed.
- **Pressure:** heroes now arrive in named parties worth capturing (knight, battle_cleric). First mission where letting a hero *live* (in your prison) is correct.
- **Briefing:** *"Death wastes a good soldier. Take them alive, break them in the dark, and send them back against their own as skeletons and turncoats."*

### Act II — Reach

**M5 · Whispers in the Circle** *(ritual / spell / mana mission)*
- **New:** the ritual_circle + mana economy + offensive spellcasting (lightning_strike), warlock casters. The Hand of Evil stops being just pick-up-and-slap and becomes a weapon.
- **Adds:** ritual_circle, warlock, lightning_strike, `ritual_tech`; spells reveal_map & speed_boost as utility.
- **Win:** `destroy_all_hero_buildings` under a `survive_time` floor. **Lose:** heart destroyed.
- **Pressure:** a wizard-led party (ranged magic) that punishes clumped defense — teaches spell counterplay. `threat_multiplier` ~1.2.
- **Briefing:** *"Power sleeps in the old circles. Bind it, and call down lightning on any hero fool enough to march in the open."*

**M6 · The Kennels** *(army-scaling / beasts mission)*
- **New:** large-army logistics — kennel, barracks, scavenger; fast shock units (hellhound, spider, lizard) and the mood/wages strain of a big roster (the "wage-collection unrest" system, roadmap §2 line 125).
- **Adds:** kennel, barracks, scavenger, hellhound, spider, lizard.
- **Win:** `destroy_all_hero_buildings` (a fortified outpost). **Lose:** heart destroyed.
- **Pressure:** sustained pressure rather than spikes; the mission is a stress test of feeding, paying, and keeping a 15–20 creature army loyal. First mission to raise `max_creatures` meaningfully.
- **Briefing:** *"An empire needs teeth. Fill the kennels, man the barracks, and keep the pay flowing — a hungry horde turns on the hand that starves it."*

> **⑂ Branch point (see §6).** M6 completion unlocks **both** M7a and M7b. The player picks one to
> play; the other's signature unlock is granted at reduced strength so no build is permanently locked
> out. This is the mission that exercises the `unlocks_after` graph's never-tested branching.

**M7a · The Restless Dead** *(graveyard path)*
- **New:** graveyard → vampire generation; playing the long, attrition-based undead game.
- **Adds:** graveyard, vampire, troll (heavy line).
- **Win:** `survive_time` (a long siege) + `destroy_heart` (rival). **Lose:** heart destroyed.
- **Pressure:** a paladin/turn_undead-heavy hero order that specifically counters undead — teaches the roster's rock-paper-scissors. Undead traits (roadmap §2 traits system) are load-bearing here.
- **Briefing:** *"Let no corpse go to waste. Raise the fallen as vampires and drown the surface in a tide that does not tire and does not fear."*

**M7b · Pacts and Sacrifice** *(temple path)*
- **New:** temple → sacrifice mechanics; succubus mood loop; demonic summons.
- **Adds:** temple, succubus, demon_spawn.
- **Win:** `custom` "complete N sacrifices" + `destroy_all_hero_buildings`. **Lose:** heart destroyed.
- **Pressure:** an inquisitor/high_priest order (anti-demon, `banish_ritual`) — the mirror counter to 7a's anti-undead order.
- **Briefing:** *"The old powers trade in blood. Feed the temple, bind the succubi and the demon-spawn, and buy an army no mortal treasury could afford."*

**M8 · The Iron Siege** *(defense mission — Act II climax)*
- **New:** layered fortress defense — magic_door locking, alarm_trap (summon defenders, roadmap §2 line 182), boulder + blowgun kill-boxes, guard_post rotation; the mission is a defensive set-piece against hero *waves*.
- **Adds:** magic_door, alarm_trap, bile_demon (anchor tank), iron_skin spell.
- **Win:** `survive_time` (a long, escalating wave assault) then `destroy_all_hero_buildings`. **Lose:** heart destroyed.
- **Pressure:** the roadmap's "hero waves" tuned as a real gauntlet — knight_commander + rally, group heals, ranged volleys. `threat_multiplier` ~1.35. This is the difficulty knee before the offensive act.
- **Briefing:** *"They come in their hundreds now, banners high, sure the dark can be dug out like a splinter. Turn every corridor into a grave and teach the Kingdoms the cost of digging too deep."*

### Act III — Dominion

**M9 · Corruption Rising** *(offense mission — taking the fight up)*
- **New:** offensive campaigning — corrupt_land, reveal_map, `possess`; assaulting a full hero **town** (town_hall, church, mage_tower, armory) instead of a lone outpost. The player is now the aggressor.
- **Adds:** archmage-tier spell access (chickenify as a panic button); no new rooms — this is a mastery mission.
- **Win:** `destroy_all_hero_buildings` (a large town) within a time pressure. **Lose:** heart destroyed **or** time expires.
- **Pressure:** the town actively counter-attacks (`defend_location` + sortie parties). First mission where the player's heart is relatively safe and the risk is overextension.
- **Briefing:** *"No more waiting in the dark. March on the border town, corrupt its fields, pull down its church and its mage-spire, and let the surface feel the ground rot beneath them."*

**M10 · Two Kings Under the Mountain** *(rival-keeper duel)*
- **New:** direct keeper-vs-keeper war against **two simultaneous** competent rivals (roadmap §2 "support multiple simultaneous rivals" — this mission is its showcase and forcing function). A race to expand, out-tech, and destroy an enemy heart.
- **Adds:** no new content — full roster is available; this is the sandbox-skirmish skills exam inside the campaign (and validates the skirmish generator, roadmap §1 line 51).
- **Win:** `destroy_heart` × both rivals. **Lose:** own heart destroyed.
- **Pressure:** two rivals with reduced `first_attack_delay` and higher `attack_cooldown_growth` resistance; the difficulty is economic tempo, not a scripted wave.
- **Briefing:** *"The deep is not big enough for three hearts. Two other keepers scrabble for the same veins — expand faster, hit harder, and leave only your heart beating in the dark."*

**M11 · The Gates of Heaven's Reach** *(assault on the capital's outer wall)*
- **New:** siege of a fortified hero capital — hero_wall / hero_gate breaching, the first **boss hero** encounter (knight_commander or high_priest as a named, buffed unit with a real party). Introduces the boss-encounter framing designed in `hero_notes/`.
- **Adds:** full arsenal; scenario grants elite-level spawns of the player's best creatures as a "war host" reward.
- **Win:** `destroy_all_hero_buildings` (the outer capital) + defeat the named boss (a `custom` objective completed on the boss party's destruction). **Lose:** heart destroyed.
- **Pressure:** `threat_multiplier` ~1.5; multiple simultaneous named parties (paladin, archmage, inquisitor) defending. Hardest offensive mission.
- **Briefing:** *"Heaven's Reach has never fallen. Its walls are old and its champions proud. Break the gate, scatter its captains, and carve a road to the heart of the Sunlit Kingdoms."*

**M12 · Deep Dominion** *(finale — the Champion of Light)*
- **New:** the climax — a two-phase mission: (1) a desperate `survive_time` defense as the Kingdoms throw everything at your heart, then (2) `destroy_heart`/`destroy_all_hero_buildings` on the capital with the **Champion of Light** (champion_of_light + divine_rage + light_blast) as the final boss, backed by high_priest and knight_commander.
- **Adds:** nothing — this is the mastery capstone; every system taught across M1–M11 is required.
- **Win:** survive the assault (phase 1, `survive_time`) → `custom` "defeat the Champion of Light" + `destroy_all_hero_buildings` (phase 2). **Lose:** heart destroyed.
- **Pressure:** highest in the campaign (`threat_multiplier` ~1.6, boss party + support). Two-phase pacing via a `time_elapsed`/`objective_complete` trigger flipping the active objective.
- **Briefing:** *"They send their brightest against you — the Champion of Light, and all the host of the Sunlit Kingdoms. Endure their fury, then walk into their capital and put out the last light. The dominion is yours to keep. Deep and forever."*

---

## 5. Difficulty curve

Difficulty is the product of the five authored dials, tuned per slot. Values below are **design
targets** to be validated by the balance/playtest program (roadmap §6), not final numbers.

| # | Mission | `threat_mult` | Rival first-attack | Hero pressure | Player `start_gold` | `max_creatures` | Net difficulty |
|---|---------|:-------------:|:------------------:|---------------|:-------------------:|:---------------:|:--------------:|
| 1 | Dark Beginnings   | 1.00 | t+240s | 1 raid, lvl 1        | 2500 | 15 | ▁ tutorial |
| 2 | Blood and Iron    | 1.10 | t+180s | 2 raids, lvl 1–2     | 2200 | 15 | ▂ |
| 3 | The Long Dark     | 1.05 | t+300s | light; econ-gated    | 1800 | 15 | ▂ (econ) |
| 4 | No Prisoners      | 1.15 | t+180s | capture targets      | 2000 | 18 | ▃ |
| 5 | Whispers/Circle   | 1.20 | t+150s | ranged casters       | 2200 | 18 | ▃ |
| 6 | The Kennels       | 1.20 | t+150s | sustained            | 2400 | 22 | ▄ (logistics) |
| 7a/b | Dead / Sacrifice| 1.25 | t+150s | hard-counter order   | 2000 | 22 | ▅ |
| 8 | The Iron Siege    | 1.35 | t+120s | **wave gauntlet**    | 2600 | 25 | ▆ (knee) |
| 9 | Corruption Rising | 1.30 | n/a (offense) | town sorties  | 3000 | 25 | ▅ (offense) |
| 10 | Two Kings        | 1.30 | t+90s ×2 | 2 rivals            | 2800 | 25 | ▆ (tempo) |
| 11 | Heaven's Reach   | 1.50 | n/a | boss + defenders       | 3200 | 28 | ▇ |
| 12 | Deep Dominion    | 1.60 | n/a | **final boss, 2-phase**| 3000 | 30 | █ climax |

Shape: a gentle ramp through Act I, an economy dip at M3 and a logistics spike at M6, a clear
**difficulty knee at M8** (the Act II defensive climax), a brief relief at M9 as the player goes on
the offensive, then a steady climb to the M12 capstone. Difficulty *levels* (easy/normal/hard, absent
today per roadmap §1) scale these columns by a global multiplier plus per-difficulty hero-party
level offsets — authored once, applied to every mission.

---

## 6. Unlock graph (exercising `unlocks_after` / `required_completed`)

```
M1 ─ M2 ─ M3 ─ M4 ─ M5 ─ M6 ─┬─ M7a ─┐
                              └─ M7b ─┴─ M8 ─ M9 ─ M10 ─ M11 ─ M12
```

- **Linear spine M1→M6 and M8→M12**: each `unlocks_after: [previous]`.
- **Branch at M6**: M7a and M7b **both** list `unlocks_after: ["the_kennels"]`; the player plays one.
- **Re-merge at M8**: M8 lists `required_completed: ["restless_dead"]` **OR** `["pacts_and_sacrifice"]`
  — i.e. either branch satisfies the gate. This is the exact never-tested `required_completed`
  path the roadmap flags (§1 line 44); authoring M7a/M7b forces it to work and to be tested.
- **Persistent-unlock consequence of the branch:** the unplayed branch's signature building
  (graveyard *or* temple) is granted at mission start in later missions but its capstone creature
  (vampire *or* demon_spawn) is *not* auto-unlocked — the player who took the graveyard road gets
  vampires as a campaign staple and demons only as scenario-local grants, and vice-versa. This gives
  the branch a lasting identity without hard-locking any content out of the finale.
- **Extensibility to a hub:** if playtests want more player agency, M9–M10 can be converted from a
  linear pair into a two-node hub (play in either order, both `required_completed` for M11) using the
  same OR-gate mechanism proven at M8. Designed-in, not built, until the linear arc is validated.

The `persistent_unlocks` block in `deep_dominion.json` grows monotonically along the spine so that the
roster at mission *N* is deterministic regardless of which M7 branch was taken (branch-specific deltas
handled as above).

---

## 7. Mechanic-introduction order (at a glance)

| Mission | Rooms | Creatures | Spells | Traps | Tech | Hero threat introduced |
|---------|-------|-----------|--------|-------|------|------------------------|
| M1 | lair, hatchery, treasury, training_hall | imp, goblin | summon_imps | spike_trap, door | — | peasant_militia, acolyte |
| M2 | guard_post | orc | — | braced_door, blowgun_trap | training_tech, trap_tech | archer |
| M3 | library, workshop | — | — | boulder_trap | trap_tech | (econ; light) |
| M4 | prison, torture_chamber | skeleton | — | — | prison_tech | knight, battle_cleric |
| M5 | ritual_circle | warlock | lightning_strike, reveal_map, speed_boost | — | ritual_tech | wizard |
| M6 | kennel, barracks, scavenger | hellhound, spider, lizard | call_to_arms | — | — | rogue, sustained mixed |
| M7a | graveyard | vampire, troll | — | — | — | paladin, high_priest (turn_undead) |
| M7b | temple | succubus, demon_spawn | corrupt_land | — | — | inquisitor, high_priest (banish) |
| M8 | (mastery) | bile_demon | iron_skin | magic_door, alarm_trap | — | knight_commander waves |
| M9 | (mastery) | — | possess, chickenify | — | — | town sorties |
| M10 | (mastery) | — | make_earth | — | — | 2 rival keepers |
| M11 | (mastery) | — | — | — | — | **boss**: knight_commander/high_priest |
| M12 | (mastery) | — | — | — | — | **boss**: champion_of_light |

By M8 all 15 rooms, all 7 traps, all 4 techs, all 11 spells, and all 13 creatures (imp + 12 combat,
counting both M7 branches) have been introduced; Act III is pure mastery and offense with no new
tools — the correct shape for a finale.

---

## 8. Dependencies & sequencing notes

This design is authorable today for the **linear** experience; three items sharpen it and are already
tracked elsewhere in the roadmap:

1. **Manifest-driven loader** (§1, `data/campaign.rs:128` / `data/scenario.rs:350`) — *required before
   authoring* so 12 missions don't each need an `include_str!` and a code edit.
2. **Multi-rival support** (§2) — *required for M10*; M1–M9 use the single-rival path that already works.
3. **Boss-hero encounters** (§1, `hero_notes/`) — *required for M11–M12*; can be prototyped as an
   elite-statted named party via existing `spawn_hero_party` + a `custom` objective before any bespoke
   boss AI exists, so it does not block starting Act III.

Non-blocking but curve-relevant: difficulty levels (§1), gem-seam economy fix (§2 line 166, felt most
in M3/M10), wage-unrest consequences (§2 line 125, load-bearing in M6), and the trait/ability systems
(§2, load-bearing for the hard-counter design of M7a/M7b) — all already implemented or tracked.

**Recommended authoring order** (maps to the demo → 1.0 plan in §8 of the roadmap): M1 (done) → M2 →
M3 as the **demo slice** (3 missions covering dig/combat/economy), then M4–M8 for the Act II vertical,
then M9–M12 once multi-rival and boss framing land.
