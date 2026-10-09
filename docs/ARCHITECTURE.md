# ARCHITECTURE: clack

**Scope:** v0.1, Linux primary, Windows experimental. Read `PRD.md` first.

> **Status note:** items marked *(planned)* are designed but not yet in the code. Items without that marker are implemented. The Windows backend compiles but has not been tested at runtime.

---

## 1. Overview

Two kinds of threads connected by one bounded channel:

- **Input threads** read raw key events from the OS and send a tiny message per key press.
- **The audio callback** (driven by the sound card) receives those messages, starts voices, mixes, and writes samples.

The only rule that really matters: **the audio callback must never block.** Everything else follows from that.

```
 Linux: /dev/input/eventN (one per keyboard)       Windows: WM_INPUT on a hidden window
        │  evdev read (blocking)                          │  GetMessageW loop
        ▼                                                 ▼
 ┌─────────────────┐      try_send () per press   ┌───────────────────┐
 │  Input thread   │ ────────────────────────────►│ Bounded channel   │
 │  (1 per device  │                              │ (crossbeam, 64)   │
 │   on Linux;     │                              └─────────┬─────────┘
 │   1 on Windows) │                                        │ try_recv
 └─────────────────┘                                        ▼
 ┌────────────────────────────────────────────────────────────┐
 │ Audio callback (cpal output stream, real-time thread)      │
 │   1. drain channel → start a Voice for each press          │
 │   2. Mixer: sum all active voices, apply gain, clamp       │
 │   3. write to output buffer (mono duplicated to channels)  │
 └────────────────────────────────────────────────────────────┘
```

The main thread parses args, loads and prepares the sample, starts the audio stream, starts the input threads, then waits for Ctrl+C.

## 2. Components

| Module | Responsibility | Touches real-time path? | Status |
|--------|----------------|-------------------------|--------|
| `main.rs` | Wiring, Ctrl+C handling, error-to-exit-code mapping, clap argument struct | No | Implemented |
| `sample.rs` | Decode the WAV, convert to mono `f32`, resample to the device rate once at startup | No | Implemented |
| `input/mod.rs` | Platform dispatch: selects the Linux or Windows `InputHandler` at compile time | No | Implemented |
| `input/linux.rs` | Enumerate `/dev/input` keyboards, one reader thread each, send presses | No (producer) | Implemented |
| `input/windows.rs` | Hidden message-only window, Raw Input registration, `WM_INPUT` handling, send presses | No (producer) | Implemented, untested at runtime |
| `mixer.rs` | Voice pool and mixing. Pure code, no I/O, unit-testable without a sound card | **Yes** | Implemented |
| `audio.rs` | Open the cpal stream, own the mixer and channel consumer, run the callback | **Yes** | Implemented |
| `cli.rs` | Split argument parsing out of `main.rs` | No | *(planned, optional)* |
| `stats.rs` | Atomic counters plus the reporter thread | Atomics only | *(planned)* |

## 3. Key data types

```rust
/// One playing click.
struct Voice {
    pos: usize,   // next sample index to play
    active: bool,
    started: u64, // sequence number, used to steal the oldest voice
}

/// Fixed-size, preallocated. No Vec growth in the callback.
struct Mixer {
    sample: Vec<f32>,     // decoded click, already at device sample rate
    voices: [Voice; 32],  // MAX_VOICES
    gain: f32,            // from --volume
    seq: u64,
}
```

**Current message type:** a press is sent as `()`.

**Planned:** when latency stats arrive, the message carries a timestamp:

```rust
struct KeyPress {
    at: std::time::Instant, // when the input thread saw it
}
```

## 4. Input backends

### Linux (`input/linux.rs`)

- `evdev::enumerate()` yields opened devices and silently skips ones it cannot read, so a permission problem appears as "no keyboards found".
- A device counts as a keyboard if it supports key events and `KEY_A`. This filters out mice, power buttons, and lid switches.
- One thread per device blocks on `fetch_events()`. Only `value == 1` (press) is sent. Release (0) and auto-repeat (2) are ignored.
- A read error ends that thread quietly (device unplugged). There is no hotplug in v0.1.
- Permissions: `/dev/input/event*` is typically `root:input` with mode `660`. The user must be in the `input` group, or run as root.

### Windows (`input/windows.rs`)

- Raw Input does not enumerate devices in this backend. `list_keyboards()` returns a placeholder so `main.rs` keeps the same shape.
- A single thread creates a message-only window (`HWND_MESSAGE`, never shown), registers for keyboard raw input with `RIDEV_INPUTSINK` (events arrive even when the window is unfocused), and runs a `GetMessageW` loop.
- `WM_INPUT` is decoded with `GetRawInputData`. Key-down events (`RI_KEY_BREAK` clear) are sent with `try_send`.
- The sender is stored in a `static`. The window procedure has no user-data pointer in this design. *(planned: replace `static mut` with `OnceLock<Sender<()>>`.)*
- **Known gap:** auto-repeat is not filtered. Holding a key sends repeated key-down messages, producing repeated clicks. The fix is to track key state per virtual key code and send only on the first down event.
- No permissions or group membership are required.

### Privacy and security note

On Linux, membership in the `input` group lets any program running as that user read raw keystrokes from every keyboard. On Windows, Raw Input sees keystrokes for the session, which is the same trust level as any user-space program. In both cases `clack` itself:

- uses key events only to start a sound
- never stores, logs, or transmits key codes
- keeps no key history beyond the channel

## 5. Threading and data flow

1. **Startup (main thread):** parse args → find keyboards (exit 1 if none) → build mixer and sample → open audio stream → start it → spawn input threads → install Ctrl+C handler → wait.
2. **Input threads:** send `()` with `try_send` per press. If the channel is full, drop the event rather than block.
3. **Audio callback (per buffer):**
   1. Drain pending presses with `try_recv` and call `mixer.trigger()` for each.
   2. `mixer.render(out, channels)`: sums voices, applies gain, clamps, and writes the same mono value to every channel of each frame.
4. **Shutdown:** Ctrl+C sends on a channel; main returns, drops the stream, and the process exits 0.

### Why one thread per device on Linux?

Each `fetch_events()` blocks, and there are usually only 1-3 keyboards. An `epoll`-based loop is a possible later optimisation.

### Why a channel instead of a mutex?

A mutex can make the audio thread wait for a preempted input thread, which causes an audible glitch. A bounded channel lets the input side use `try_send` and the audio side use `try_recv`, so neither blocks.

## 6. Real-time rules for the audio callback

These are hard rules. Break them and you get clicks and pops:

- ❌ No heap allocation (`Vec::push`, `String`, `Box::new`, `format!`)
- ❌ No locks (`Mutex`, `RwLock`), no blocking channel operations
- ❌ No I/O (`println!`, `eprintln!`, file reads)
- ❌ No `unwrap()` or `panic!` on the hot path
- ✅ Preallocated buffers, fixed-size arrays, atomics, plain arithmetic

The stream **error** callback still prints to stderr, which breaks this rule. It only runs on errors, so it is accepted for v0.1 as a known exception.

## 7. Mixing

- `trigger()`: find an inactive voice; if none, steal the voice with the lowest `started`. Does nothing if no sample is loaded.
- `render(out, channels)`: for each frame, add `sample[pos] * gain` for every active voice, advance `pos`, deactivate at the end of the sample. Clamp the sum to `[-1.0, 1.0]` and write it to all channels of that frame.
- **Clipping:** hard clamp in v0.1. A soft clipper (`tanh`) is a later nicety.
- **Gain:** `(volume / 100.0)^2`.

## 8. Audio output configuration

- cpal's default host and default output device on both platforms (ALSA on Linux, WASAPI on Windows).
- **Sample format:** only `f32` is accepted; other formats return an error naming the format.
  - *(planned)* Convert `i16` and other formats to `f32` at the edge, outside the mixer.
- **Buffer size:** the backend default is used.
  - *(planned)* Request a small fixed buffer (256 frames) and fall back to default if refused, reporting the result in `--verbose`.
- The click is resampled once at startup to the device rate (linear interpolation).
- Linux: cpal's ALSA backend works on plain ALSA and PipeWire (through the PipeWire ALSA plugin).

## 9. Latency model

```
finger down ─► kernel / OS ─► input read ─► channel ─► callback ─► device buffer ─► speaker
   (not measured)   (small)    (~µs)      (<1 ms)   (≤ 1 buffer)   (1-2 buffers)
```

What `--verbose` will report *(planned; the flag is parsed but prints nothing yet)*:

- **queue wait** = time from `KeyPress.at` to the start of the callback that consumed it
- **output latency** = cpal's playback − callback timestamp gap
- **estimate** = queue wait + output latency

These are software estimates only.

## 10. Dependencies

| Crate | Purpose | Platform |
|-------|---------|----------|
| `clap` | Argument parsing | all |
| `cpal` | Audio output | all |
| `hound` | WAV decoding | all |
| `crossbeam-channel` | Channel between input threads and audio callback | all |
| `ctrlc` | Ctrl+C handling | all |
| `anyhow` | Error handling | all |
| `evdev` | Reading `/dev/input` | Linux |
| `windows-sys` | Raw Input and window APIs | Windows |

Feature flags for `windows-sys` must include `Win32_Graphics_Gdi` (the `WNDCLASSW` struct depends on GDI types), along with `Win32_Foundation`, `Win32_UI_Input`, `Win32_UI_WindowsAndMessaging`, and `Win32_System_LibraryLoader`.

## 11. Build and release

- `.github/workflows/release.yml` builds `x86_64-unknown-linux-gnu` on `ubuntu-latest` and `x86_64-pc-windows-msvc` on `windows-latest`.
- Each build is packaged as a `.tar.gz` with the binary, README, and LICENSE.
- A tag push matching `v*` runs the `release` job, which creates a GitHub release with both archives and `SHA256SUMS`.
- `workflow_dispatch` runs the builds without publishing.

## 12. Repository layout

```
clack/
├── Cargo.toml
├── README.md
├── LICENSE                 # MIT
├── CLAUDE.md
├── assets/
│   ├── press.wav           # bundled via include_bytes!
│   └── LICENSE.md          # source + license of the sample
├── docs/
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   ├── DESIGN.md
│   └── PHASES.md
├── .github/workflows/
│   └── release.yml
└── src/
    ├── main.rs
    ├── sample.rs
    ├── mixer.rs
    ├── audio.rs
    └── input/
        ├── mod.rs
        ├── linux.rs
        └── windows.rs
```

## 13. Failure modes

| Failure | Behaviour |
|---------|-----------|
| No keyboards found, including permission denied (Linux) | Print error with setup hint; exit 1. |
| No keyboard input available (Windows) | Print error; exit 1. *(Not expected in practice; Raw Input does not enumerate.)* |
| No audio output device | Print error; exit 1. |
| Device offers no `f32` output config | Exit 1, naming the sample format. *(planned: convert)* |
| Click sample decodes to zero samples | *(planned)* Exit 1 at startup. Currently plays silence. |
| Keyboard unplugged mid-run (Linux) | That reader thread exits; others keep running. |
| Channel full | Drop the key event silently. *(planned: count in verbose stats)* |
| Audio stream error | Print error to stderr; the stream stops. |
| Windows Raw Input registration fails | *(not yet handled)* Currently ignored. Should print an error and exit 1. |

## 14. Testing strategy

- **Unit tests** (no hardware) in `mixer.rs`: voice allocation, stealing order, gain, clipping, voice finishing, mono-to-multichannel output.
- **Unit tests still to write** in `sample.rs`: resampler output length (for example, 100 samples from 44100 Hz to 48000 Hz should produce 109), and the empty-sample case once the check in section 13 exists.
- **Compile checks:** `cargo check --target x86_64-pc-windows-msvc` verifies the Windows backend type-checks from Linux.
- **CI:** both platforms build on every release run.
- **Manual tests (Linux):** press-and-hold, fast typing, two keyboards, volume 0 / 30 / 100, Ctrl+C.
- **Manual tests (Windows, not yet done):** typing produces clicks, holding a key (expected to fail until auto-repeat is filtered), volume 0 / 30 / 100, Ctrl+C.

## 15. Extension points

- Release samples: also emit `KeyRelease` messages.
- Per-key sounds: add a key code to the message type.
- Sound packs: replace the single `sample` with a lookup table.
- macOS: add `input/macos.rs` behind the same `InputHandler` shape; the mixer and audio layers are already platform-neutral.
