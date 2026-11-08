---
title: Narrator files
description: What each part of a narrator does, and what makes one write well.
---

The narrators you make in **Settings**, **Narrators** are the same kind of file: **Export** saves
one to share, **Import** adds one someone shared. The editor has a field for each entry in the
table below.

A narrator is who writes the journal: the murloc scribe by default, or a style a player picks. Each
narrator is a data file, one per narrator and prose language, in
`companion/crates/mrglglgl-core/narrators/<id>.<language>.toml`. No code changes are needed for a
new voice or a translation of an existing one, apart from listing the file in
`src/narrators.rs`.

The rules about facts, memory, length and language are the app's, the same for every narrator, and
are not part of a narrator file. A narrator decides only how the entry sounds.

## Fields

| Field | Meaning |
|---|---|
| `id` | Stable identifier shared by every language of the narrator, e.g. `grumpy_dwarf` |
| `language` | Prose language: `en` or `fr` |
| `name` | Shown to players in Settings |
| `opt_in` | `true` for a narrator only used when a player picks it (the full murloc-speak voice is one) |
| `point_of_view` | `adventurer` (the default): the entry says "I", as the adventurer, and the narrator is the voice it is written in. `observer`: the narrator tells what it watched, calling the adventurer by name |
| `persona` | Who is writing, in one or two sentences |
| `voice` | How it sounds: warmth, humour, register |
| `murloc` | How the murloc shows on a day without a margin note |
| `margin_note` | The instruction used on days the murloc adds a note in the margin |
| `[example]` | One worked example: `notes` (a day's facts, not from the test fixtures) and `entry` (the entry this narrator would write), plus `margin_note` |
| `[hints]` | One line per day pattern: `only_fished`, `many_deaths`, `big_level_jump`, `new_zone`, `rare_loot`, `murloc_heavy`, `murloc_death`, `nothing_happened` |

`murloc_scribe.en.toml` is the reference: copy it and change the voice.

## What makes a good narrator

- **Never mock the player.** A narrator can grumble about the day; it is always on the adventurer's
  side.
- **The murloc is seasoning.** Entries are readable prose; murloc-speak belongs in margin notes,
  except for the opt-in murloc voice.
- **The example matters most.** Small models copy the shape of the worked example closely: its
  length, its rhythm, and whether it invents things. Keep it about 120 words, built only from its
  own notes, with every number and name from those notes.
- **Hints steer, they do not add facts.** "Make the quiet charming" is a hint; "mention the rain"
  would invite an invented event.

## Testing a narrator

`cargo test -p mrglglgl-core --test prompt` checks every shipped narrator renders a complete prompt.
To read real entries, run the narrator samples with a downloaded model (`MRGLGLGL_TEST_MODEL`
pointing at a GGUF file), and read what comes out before shipping.
