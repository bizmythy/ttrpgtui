# ttrpgtui

A keyboard-driven terminal UI for tracking tabletop RPG creatures during an encounter.

Built with [Ratatui] and [`ratatui-textarea`].

## Usage

Run the app with:

```sh
cargo run
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
