# DESIGN: clack

CLI experience, messages, and the reasoning behind key decisions. For structure see `ARCHITECTURE.md`; for requirements see `PRD.md`.

---

## 1. Design principles

1. **One command, zero config.** `clack` should work after install with no files to edit.
2. **Quiet by default.** Print almost nothing unless something is wrong or `--verbose` is set.
3. **Fail helpfully.** Every error says what happened *and* what to do next.
4. **Latency over features.** When a feature risks delay in the key-to-sound path, the feature loses.
5. **Small enough to finish.** If it doesn't fit the weekend-toy scope, it goes in the backlog.

## 2. Command-line interface

```
clack [OPTIONS]

Options:
  -v, --volume <0-100>   Output volume [default: 50]
      --verbose          Print latency stats once per second
  -h, --help             Print help
  -V, --version          Print version
```

### Examples

```
clack                    # default volume 50
clack --volume 80        # louder
clack -v 20              # quiet
clack --volume 60 --verbose
```

### Argument rules

| Input | Result |
|-------|--------|
| `--volume 0` | Valid, silent (useful for testing the plumbing). |
| `--volume 100` | Valid, maximum. |
| `--volume 101`, `--volume -5`, `--volume abc` | Rejected by clap with a usage error, exit code 2. |
| Unknown flag | Rejected by clap, exit code 2. |

Use a clap value parser with an integer range (`0..=100`) so validation and the error text come for free.

## 3. Output design

### Normal start (one line, then silence)

```
clack: listening on 2 keyboards, volume 60. Press Ctrl+C to stop.
```

### Clean exit

Nothing printed, or a single `clack: bye`. Exit code 0.

### `--verbose` output

One line per second while keys are being pressed, nothing while idle:

```
[clack] keys/s: 7  queue-wait avg 0.2ms  output 5.3ms  est avg 5.5ms  p95 6.1ms  max 7.4ms  dropped 0
```

At startup, `--verbose` also prints the facts that explain latency:

```
[clack] output device: <name>
[clack] sample rate: 48000 Hz, channels: 2
[clack] buffer: 256 frames requested, 256 granted (5.3 ms)
[clack] keyboards: /dev/input/event3 (AT Translated Set 2 keyboard), /dev/input/event7 (USB Keyboard)
```

If the buffer request was refused:

```
[clack] buffer: 256 frames requested, backend chose 1024 (21.3 ms) -- latency will be higher
```

Printing the device paths and names is for debugging only. Key codes are **never** printed.

## 4. Error messages

Each error: what failed, then the fix. Lowercase `clack:` prefix, written to stderr, exit code 1.

**No keyboards found / permission denied**
```
clack: can't read keyboard input (permission denied on /dev/input).
  Fix: add your user to the 'input' group, then log out and back in:
       sudo usermod -aG input $USER
  Note: this lets programs you run read raw keystrokes. See README "Permissions".
```

**No keyboards found (permissions fine)**
```
clack: no keyboard devices found under /dev/input.
  Is a keyboard connected? Try: ls -l /dev/input/by-id/
```

**No audio device**
```
clack: no audio output device found.
  Check your sound settings (PipeWire / PulseAudio / ALSA) and try again.
```

**Audio stream failed mid-run**
```
clack: audio stream error: <underlying error>
```

## 5. Key design decisions

Short ADR-style records. Revisit by ear and by measurement, not by opinion.

### D1: Read `/dev/input` with evdev instead of a global-hook crate

- **Chosen:** `evdev`, reading kernel input devices.
- **Why:** works on X11 *and* Wayland; hook libraries built on X11 fail on Wayland.
- **Cost:** needs `input` group membership; keystrokes become readable to that user's other programs.
- **Revisit if:** a better-supported portal-based approach appears for Wayland.

### D2: One click sample per press, ignore releases and auto-repeat

- **Why:** a real switch actuates once per press; holding a key does not repeat the mechanical sound.
- **Later:** add a release sample (FR-9).

### D3: Preload and pre-resample the sample at startup

- **Why:** zero decoding, zero file I/O, zero allocation on the key path.
- **Cost:** slightly slower startup (milliseconds).

### D4: Fixed-size voice pool with oldest-voice stealing

- **Why:** deterministic memory, no allocation in the callback; 32 voices is far beyond what anyone can type.
- **Behaviour at overflow:** the oldest click is cut off, which is almost always inaudible.

### D5: Squared volume taper

- **Gain:** `(volume/100)^2`.
- **Why:** perceived loudness is closer to logarithmic, so a linear slider feels top-heavy. Squaring is a cheap fix.
- **Revisit:** if `--volume 30` still sounds too loud or too quiet, switch to a real dB mapping.

### D6: No hotkeys, no config file, no tray

- **Why:** each adds platform-specific code and design surface. Volume is a launch argument; to change it, restart.

### D7: Drop events rather than block when the queue is full

- **Why:** blocking an input thread is harmless, but there's no scenario where the audio side should wait. A dropped click under extreme load is invisible; an audio glitch is not.

### D8: Linux-first, with platform code isolated to `input.rs`

- **Why:** the mixer and audio layers are already portable; porting means adding input backends.

## 6. Sound design notes

- **Format:** mono WAV, 16-bit or 24-bit, 44.1 or 48 kHz; trim leading silence so the click starts at sample 0.
- **Length:** short, around 80-200 ms. Long tails overlap and mud up fast typing.
- **License:** CC0 only; record origin in `assets/LICENSE.md`.
- **Normalisation:** peak around -3 dBFS so overlapping voices rarely clip.
- **Auditioning:** test at several typing speeds. A sample that sounds great at one key per second can sound like static at ten.

## 7. Accessibility and etiquette

- Volume defaults to a moderate 50, not 100.
- The tool is audible to others nearby; mention this lightly in the README.
- Exit is always Ctrl+C; no stuck background process.

## 8. Naming

`clack` is a placeholder. Candidates if it's taken: `thock`, `clickety`, `keyclack`. Check crates.io and the GitHub namespace before publishing.
