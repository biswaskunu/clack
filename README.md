# clack

> Turn any keyboard into a mechanical one. A tiny, low-latency CLI that plays key-click sounds through your speakers as you type.

**Status:** v0.1.0. Linux is the primary platform. Windows is experimental: it builds, but has not yet been tested at runtime. See [Platform support](#platform-support).

## Why

Membrane and laptop keyboards are quiet. Existing click-sound tools tend to be heavy GUI or Electron apps. `clack` is one small native binary with one job and one flag.

## Install

Download the archive for your platform from the [Releases](https://github.com/biswaskunu/clack/releases) page, verify it, and extract it.

```bash
sha256sum -c SHA256SUMS
tar xzf clack-linux-x86_64.tar.gz
./clack --volume 60
```

### Linux permissions

`clack` reads keyboard events from `/dev/input`. Your user needs membership in the `input` group, one time only:

```bash
sudo usermod -aG input $USER
```

Log out and back in, then confirm with `groups`. Running as root also works but is not recommended.

**Privacy note:** membership in the `input` group lets any program you run read raw keystrokes. `clack` itself only uses key presses to trigger a sound. It never stores, logs, or sends key data.

### Windows

No setup is needed. Raw Input is used, so there is no group or permission step. Windows may show a SmartScreen warning because the binary is unsigned.

## Usage

```
clack [OPTIONS]

  -v, --volume <0-100>   Output volume (default: 50)
      --verbose          Print latency stats once per second (not yet implemented)
  -h, --help             Print help
  -V, --version          Print version
```

Stop with Ctrl+C.

## Features

- Plays a click on every key press, on every connected keyboard (Linux)
- Volume set at launch with `--volume 0-100`
- Fast typing layers clicks instead of cutting them off
- Works on X11 and Wayland (Linux reads the kernel, not the display server)
- No config file, no GUI, no hotkeys

## Platform support

| Platform | Status | Known gaps |
|----------|--------|------------|
| Linux x86_64 | Supported | `--verbose` not implemented yet |
| Windows x86_64 | Experimental. Builds in CI, not yet tested at runtime | Holding a key produces repeated clicks (auto-repeat not filtered) |

## Measured latency

Not yet measured. The `--verbose` latency report is planned for a later release. Measurements will be added here when available.

## Building from source

Linux needs the ALSA development headers (`libasound2-dev` on Debian/Ubuntu, `alsa-lib` on Arch).

```bash
cargo build --release
```

## Project docs

- [`docs/PRD.md`](docs/PRD.md): what and why
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md): how it's built
- [`docs/DESIGN.md`](docs/DESIGN.md): CLI experience and decisions
- [`docs/PHASES.md`](docs/PHASES.md): build plan

## License

MIT. See [`LICENSE`](LICENSE). The bundled click sample is CC0; see [`assets/LICENSE.md`](assets/LICENSE.md).
