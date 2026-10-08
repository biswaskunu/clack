# clack

> Turn any keyboard into a mechanical one. A tiny, low-latency Linux CLI that plays key-click sounds through your speakers as you type.

**Status:** pre-v0.1, in development. Linux only for now.

## Why

Membrane and laptop keyboards are quiet. Existing click-sound tools tend to be heavy GUI or Electron apps. `clack` is one small native binary with one job and one flag.

## Project docs

- [`docs/PRD.md`](docs/PRD.md): what and why
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): how it's built
- [`docs/DESIGN.md`](docs/DESIGN.md): CLI experience and decisions
- [`docs/PHASES.md`](docs/PHASES.md): build plan
- [`CLAUDE.md`](CLAUDE.md): working agreement for AI help

## Features (v0.1)

- Plays a click on every key press, on every connected keyboard
- Volume set at launch: `--volume 0-100`
- Fast typing layers clicks instead of cutting them off
- Holding a key gives one click, not a machine gun
- Works on X11 and Wayland
- No config file, no GUI, no hotkeys

## License

TBD (suggest MIT or Apache-2.0). Bundled sample: see `assets/LICENSE.md`.
