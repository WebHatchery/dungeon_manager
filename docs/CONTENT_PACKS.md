# Content packs

Deep Dominion loads optional packs from `assets/mods/`. Add a directory name to
`assets/mods/load_order.json`; each directory must contain a `pack.json` manifest.
Packs are applied in listed order, so a later item with the same id replaces an
earlier item. `assets/` and `maps/` entries are search roots for the normal
asset and map paths.

```json
{
  "id": "my_pack",
  "name": "My Pack",
  "version": "1.0.0",
  "priority": 0,
  "data": {
    "monsters": ["data/monsters.json"],
    "rooms": ["data/rooms.json"]
  },
  "assets": ["assets"],
  "maps": ["maps"]
}
```

Data files are arrays matching the built-in schemas. IDs are the replacement
key; use an existing ID to override content or a new ID to add content. The
loader validates scenario references, hero trigger names, spell/hero effect
types, supported polymorph forms, and finite numeric values after all packs are
merged. Invalid content stops startup with the complete validation message.

The empty `assets/mods/example_pack/pack.json` is a safe manifest template. It
is not enabled by default; copy it and add real data files before placing it in
`load_order.json`.
