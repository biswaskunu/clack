# ARCHITECTURE: clack

**Scope:** v0.1, Linux only. Read `PRD.md` first.

> **Status note:** items marked *(planned)* are designed but not yet in the code. Items without that marker are implemented.

---

## 1. Overview

Two kinds of threads connected by one lock-free channel:

- **Input threads** read raw key events from the kernel and send a tiny message per key press.
- **The audio callback** (driven by the sound card) receives those messages, starts voices, mixes, and writes samples.

The only rule that really matters: **the audio callback must never block.** Everything else in the design follows from that.

```
 /dev/input/eventN  (one per keyboard)
        │  evdev read (blocking)
        ▼
 ┌─────────────────┐      send () per press
 │  Input thread   │ ───────────────────────────┐
 │  (1 per device) │                            ▼
 └─────────────────┘                  ┌───────────────────┐
                                      │ Bounded channel   │
                                      │ (crossbeam, 64)   │
                                      └─────────┬─────────┘
                                                │ try_recv (non-blocking)
                                                ▼
 ┌────────────────────────────────────────────────────────────┐
 │ Audio callback (cpal output stream, real-time thread)      │
 │   1. drain channel → start a Voice for each press          │
 │   2. Mixer: sum all active voices, apply gain, clamp       │
 │   3. write to output buffer (mono duplicated to channels)  │
 └────────────────────────────────────────────────────────────┘
                                                │ atomics *(planned, Phase 5)*
                                                ▼
                                      ┌───────────────────┐
                                      │ Stats thread      │  (--verbose only)
                                      │ prints once / sec │
                                      └───────────────────┘
```

The main thread parses args, loads and prepares the sample, starts the audio stream, starts the input threads, then waits for Ctrl+C.

## 2. Components

| Module | Responsibility | Touches real-time path? | Status |
|--------|----------------|-------------------------|--------|
| `main.rs` | Wiring, Ctrl+C handling, error-to-exit-code mapping. Owns the `clap` argument struct. | No | Implemented |
| `sample.rs` | Decode the WAV, convert to mono `f32`, resample to the device rate **once at startup**. | No | Implemented |
| `input.rs` | Find keyboards, spawn one reader thread each, filter to key-press events, send to the channel. | No (producer side) | Implemented |
| `mixer.rs` | Voice pool and mixing logic. **Pure code, no I/O**, so it is unit-testable without a sound card. | **Yes** | Implemented |
| `audio.rs` | Open the cpal stream, own the mixer and channel consumer, run the callback. | **Yes** | Implemented |
| `cli.rs` | Split argument parsing out of `main.rs`. | No | *(planned, optional)* |
| `stats.rs` | Atomic counters plus the reporter thread. | Atomics only | *(planned, Phase 5)* |

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

**Current message type:** a press is sent as `()`, since nothing else is needed yet.

**Planned (Phase 5):** when latency stats arrive, the message becomes a struct that carries a timestamp:

```rust
/// Sent from an input thread to the audio callback. (planned, Phase 5)
struct KeyPress {
    at: std::time::Instant, // when the input thread saw it
}
```

`Instant` is `Copy` and cheap, so this stays safe to send through the channel.

## 4. Threading and data flow

1. **Startup (main thread):** parse args → find keyboards (exit 1 if none) → build the mixer and sample → open audio stream → start it → spawn input threads → install Ctrl+C handler → wait.
2. **Input thread loop:** block on `fetch_events()`. For each event with `value == 1` (press), send `()` with `try_send`. Ignore `value == 0` (release) and `value == 2` (auto-repeat) in v0.1. If the channel is full, **drop the event** rather than block. A read error ends the thread quietly (device unplugged).
3. **Audio callback (per buffer, every few ms):**
   1. Drain pending presses with `try_recv` and call `mixer.trigger()` for each.
   2. `mixer.render(out, channels)`: sums voices, applies gain, clamps, and writes the same mono value to every channel of each frame.
   3. Record stats in atomics *(planned, Phase 5)*.
4. **Shutdown:** Ctrl+C sends a signal on a channel; main returns, drops the stream, and the process exits 0.

### Why one thread per device?

It is the simplest correct model: each `Device::fetch_events()` blocks, and there are usually only 1-3 keyboards. An `epoll`-based single loop is a possible later optimisation, not a v0.1 need.

### Why a channel instead of a mutex?

A mutex can make the audio thread wait for an input thread that was preempted, which causes an audible glitch. A bounded `crossbeam-channel` lets the input side use `try_send` and the audio side use `try_recv`, so neither ever blocks.

## 5. Real-time rules for the audio callback

These are hard rules. Break them and you get clicks and pops:

- ❌ No heap allocation (`Vec::push`, `String`, `Box::new`, `format!`)
- ❌ No locks (`Mutex`, `RwLock`), no channel operations that can block
- ❌ No I/O (`println!`, `eprintln!`, file reads, syscalls you control)
- ❌ No `unwrap()` or `panic!` on the hot path
- ✅ Preallocated buffers, fixed-size arrays, atomics, plain arithmetic

Printing stats from inside the callback is therefore forbidden; a separate thread reads atomics and prints. The stream **error** callback also prints, which breaks this rule. It only runs on errors, so it is accepted for v0.1, but it is a known exception.

## 6. Mixing

- `trigger()`: find an inactive voice; if none, **steal the voice with the lowest `started`** (the oldest). Set `pos = 0`. Does nothing if no sample is loaded.
- `render(out, channels)`: for each frame, add `sample[pos] * gain` for every active voice, advance `pos`, deactivate when `pos >= sample.len()`. Clamp the sum to `[-1.0, 1.0]` and write it to all channels of that frame.
- **Clipping:** hard clamp in v0.1. A soft clipper (`tanh`) is a later nicety.
- **Gain:** `gain = (volume / 100.0)^2`, a cheap taper that feels more even to the ear than linear.

## 7. Audio output configuration

- Use cpal's default host and default output device.
- **Sample format:** the current code accepts only `f32` and returns an error for anything else.
  - *(planned)* Search the device's supported output configs for an `f32` range that covers the default sample rate. If none exists, convert `i16` (and other formats) to `f32` at the edge, outside the mixer.
- **Buffer size:** the current code leaves the buffer size at the backend default.
  - *(planned, Phase 5)* Request a **small fixed buffer** (start with 256 frames, about 5 ms at 48 kHz). If the backend rejects it, fall back to the default and say so in `--verbose`.
- Resample the click **once at startup** to the device sample rate (linear interpolation is fine for a short click).
- Linux backends: cpal's ALSA backend works on both plain ALSA and PipeWire (through the PipeWire ALSA plugin). A native PipeWire or JACK host is an optional later experiment if latency disappoints.

## 8. Input handling (Linux)

- Enumerate devices with the `evdev` crate. `enumerate()` opens each device and silently skips any it cannot read, so a permission problem shows up as "no keyboards found".
- **Keyboard heuristic:** a device counts as a keyboard if it supports key events *and* the `KEY_A` code. This filters out mice, power buttons, and lid switches.
- Works on **X11 and Wayland**, because it reads from the kernel, not the display server. (Global-hook libraries built on X11 do not work on Wayland.)
- **Permissions:** `/dev/input/event*` is typically `root:input` with mode `660`. The user must be in the `input` group (log out and in afterwards) or run as root.

### Privacy and security note

Membership in the `input` group lets *any* program running as that user read raw keystrokes from every keyboard. This is the cost of the approach and is why the README explains it. `clack` itself:

- uses key events only to start a sound
- never stores, logs, or transmits key codes
- keeps no key history in memory beyond the channel

## 9. Latency model

```
finger down ─► kernel ─► evdev read ─► channel ─► callback ─► device buffer ─► speaker
   (not measured)  (small)   (~µs)    (<1 ms)   (≤ 1 buffer)   (1-2 buffers)
```

What `--verbose` will report *(planned, Phase 5; the flag is parsed but prints nothing yet)*. These are software estimates only:

- **queue wait** = time from `KeyPress.at` to the start of the callback that consumed it
- **output latency** = cpal's `playback − callback` timestamp gap for that buffer
- **estimate** = queue wait + output latency

Stats are written to atomics by the callback (sum, count, max, plus a small fixed histogram for p95) and read once per second by the stats thread.

## 10. Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | Argument parsing |
| `evdev` | Reading `/dev/input` |
| `cpal` | Audio output |
| `hound` | WAV decoding |
| `crossbeam-channel` | Lock-free bounded channel between input threads and the audio callback |
| `ctrlc` | Clean Ctrl+C handling |
| `anyhow` | Error handling in `main` and setup code |

Keep the list this short. Check each crate's current docs for exact API names, since `evdev` and `cpal` have changed their types between major versions.

## 11. Repository layout

```
clack/
├── Cargo.toml
├── README.md
├── CLAUDE.md
├── assets/
│   ├── press.wav          # bundled via include_bytes!
│   └── LICENSE.md         # source + license of the sample
├── docs/
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   ├── DESIGN.md
│   └── PHASES.md
└── src/
    ├── main.rs
    ├── sample.rs
    ├── input.rs
    ├── mixer.rs
    ├── audio.rs
    ├── cli.rs             # planned (optional split from main.rs)
    └── stats.rs           # planned, Phase 5
```

## 12. Failure modes

| Failure | Behaviour |
|---------|-----------|
| No keyboards found (including permission denied) | Print an error with setup hint; exit 1. |
| No audio output device | Print error; exit 1. |
| Device offers no `f32` output config | Currently: exit 1 with the sample format named. *(planned: convert, see section 7)* |
| Click sample decodes to zero samples | *(planned)* Exit 1 at startup with a clear message. Currently plays silence without an error. |
| Keyboard unplugged mid-run | That reader thread exits quietly; others keep running (v0.1: no hotplug). |
| Channel full | Drop the key event silently. *(planned: count it in verbose stats)* |
| Buffer size rejected | *(planned, Phase 5)* Fall back to default buffer; note it in verbose output. |
| Audio stream error | Print error to stderr; the stream stops. |

## 13. Testing strategy

- **Unit tests** (no hardware), implemented in `mixer.rs`: voice allocation, voice stealing order, gain math, clipping, voice finishing, mono-to-multichannel output.
- **Unit tests** still to write in `sample.rs`: resampler output length (for example, 100 samples from 44100 Hz to 48000 Hz should produce 109), and the empty-sample case once the check in section 12 exists.
- **Manual tests:** press-and-hold, fast typing, two keyboards, volume 0 / 30 / 100, Ctrl+C.
- **Latency:** the `--verbose` estimate, compared against a rough sanity check (record speaker plus keyboard with a phone and look at the waveform gap) if you're curious.

## 14. Extension points (later)

- Release samples: also emit `KeyRelease` messages.
- Per-key or per-region sounds: add a key code to the message type.
- Sound packs: replace the single `sample` with a lookup table.
- Other OSes: hide input capture behind a trait; the mixer and audio layers are already platform-neutral.
