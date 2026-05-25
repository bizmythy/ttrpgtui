# ttrpgtui

A keyboard-driven terminal UI for tracking tabletop RPG creatures during an encounter.

Built with [Ratatui] and [`ratatui-textarea`].

## Usage

Run the app with:

```sh
cargo run
```

Encounter/session data is stored in the directory where the app is launched. Use `--dir DIR` or `-d DIR` to store it somewhere else.

## Campaign creature presets

Each data directory has a campaign-specific `campaign.ron` file. Creatures listed there are automatically added to every newly-created encounter. Manual presets include name, health, AC, and description; initiative is intentionally not configurable because it should be rolled per encounter.

You can also reference a D&D Beyond character ID. When a new encounter is created, the app fetches that character and uses D&D Beyond's reported name, max HP, AC if present in the API response, and character summary. The app does not manually calculate AC.

D&D Beyond only allows this for character sheets your request can access. Public/campaign-visible sheets should work anonymously; private sheets require authentication. To authenticate, set `DND_BEYOND_AUTHORIZATION` to the full `Authorization` header value, `DND_BEYOND_BEARER_TOKEN` to just the token, or `DND_BEYOND_COOKIE` to a valid D&D Beyond `Cookie` header value before launching the app.

Example:

```ron
(
    creatures: [
        (
            name: "Mira",
            health: 24,
            ac: Some(16),
            description: "party cleric",
        ),
        (
            dnd_beyond_character_id: 48690485,
        ),
    ],
)
```

## Keybindings

| Key | Action |
| --- | --- |
| `j` / `Down` | Move to the next creature |
| `k` / `Up` | Move to the previous creature |
| `g` | Move to the first creature |
| `G` | Move to the last creature |
| `Space` | Toggle multi-select for the hovered creature |
| `+` / `=` | Add health to the selected creatures, or hovered creature if none are selected |
| `-` / `_` | Subtract health from the selected creatures, or hovered creature if none are selected |
| `n` | Open the new-creature form |
| `r` | Rename the hovered creature; disabled while any creature is selected |
| `u` | Undo the last creature mutation |
| `Ctrl+R` | Redo the last undone creature mutation |
| `Esc` | Cancel an open form; quit from normal mode |
| `q` | Quit from normal mode |
| `Ctrl+C` | Quit |

## Creature display

- The hovered creature row is shown with a focused border.
- Multi-selected creature rows are shown with a distinct selected border.
- Creatures at `0` or lower health render in red.
- Creatures sort by descending initiative.
- Creatures without initiative sort after all creatures with numeric initiative.
- Blank/unknown AC is displayed as `—`.

## Health changes

Press `+`/`=` or `-`/`_` to open a small health input popup. Enter a positive number and press `Enter` to apply it.

If any creatures are selected, the change applies to all selected creatures. If none are selected, it applies only to the hovered creature.

Healing is capped at max health. Damage can reduce health below zero.

## New creatures

Press `n` to open the new-creature form. Fields:

- `name` — required
- `initiative` — optional number; blank initiatives sort last
- `AC` — optional positive number; blank means unknown/not tracked
- `health` — required positive number; used as both current and max health
- `count` — optional positive number; blank means `1`

When `count` is greater than `1`, the app creates multiple creatures with letter suffixes, for example `Goblin A`, `Goblin B`, `Goblin C`.

## Undo/redo

Undo and redo track creature mutations, including health changes, creature renames, and new-creature creation. Navigation, selection toggles, and text typed into an unsubmitted form are not part of undo history.

## License

Copyright (c) bizmythy <andrew.p.council@gmail.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[Ratatui]: https://ratatui.rs
[`ratatui-textarea`]: https://crates.io/crates/ratatui-textarea
[LICENSE]: ./LICENSE
