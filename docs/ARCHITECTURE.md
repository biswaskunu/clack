# ARCHITECTURE: clack

**Scope:** v0.1, Linux only. Read `PRD.md` first.

---

## 1. Overview

Two kinds of threads connected by one lock-free queue:

- **Input threads** read raw key events from the kernel and push a tiny message per key press.
- **The audio callback** (driven by the sound card) pops those messages, starts voices, mixes, and writes samples.

The only rule that really matters: **the audio callback must never block.** Everything else in the design follows from that.

```
 /dev/input/eventN  (one per keyboard)
        │  evdev read (blocking)
        ▼
 ┌─────────────────┐      push KeyPress{ts}
 │  Input thread   │ ───────────────────────────┐
 │  (1 per device) │                            ▼
 └─────────────────┘                  ┌───────────────────┐
                                      │ Lock-free queue   │
                                      │ (bounded, MPSC)   │
                                      └─────────┬─────────┘
                                                │ pop (non-blocking)
                                                ▼
 ┌────────────────────────────────────────────────────────────┐
 │ Audio callback (cpal output stream, real-time thread)      │
 │   1. drain queue → start a Voice for each KeyPress         │
 │   2. Mixer: sum all active voices × gain                   │
 │   3. write to output buffer; update atomic stats           │
 └────────────────────────────────────────────────────────────┘
                                                │ atomics
                                                ▼
                                      ┌───────────────────┐
                                      │ Stats thread      │  (--verbose only)
                                      │ prints once / sec │
                                      └───────────────────┘
```

The main thread parses args, loads and prepares the sample, starts everything, then waits for Ctrl+C.

## 2. Components

| Module | Responsibility | Touches real-time path? |
|--------|----------------|-------------------------|
| `cli.rs` | Parse and validate args (`clap`). | No |
| `sample.rs` | Decode the WAV, convert to mono `f32`, resample to the device rate **once at startup**. | No |
| `input.rs` | Find keyboards, spawn one reader thread each, filter to key-press events, push to queue. | No (producer side) |
| `mixer.rs` | Voice pool and mixing logic. **Pure code, no I/O**, so it is unit-testable without a sound card. | **Yes** |
| `audio.rs` | Open the cpal stream, own the mixer and queue consumer, run the callback. | **Yes** |
| `stats.rs` | Atomic counters plus the reporter thread. | Atomics only |
| `main.rs` | Wiring, signal handling, error-to-exit-code mapping. | No |

## 3. Key data types

```rust
/// Sent from an input thread to the audio callback.
struct KeyPress {
    at: std::time::Instant, // when the input thread saw it (for latency stats)
}

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

`Instant` is `Copy` and cheap, so `KeyPress` is safe to send through the queue.

## 4. Threading and data flow

1. **Startup (main thread):** parse args → build the sample → open audio stream (paused) → spawn input threads → start stream.
2. **Input thread loop:** block on `fetch_events()`. For each event with `value == 1` (press), push `KeyPress { at: Instant::now() }`. Ignore `value == 0` (release) and `value == 2` (auto-repeat) in v0.1. If the queue is full, **drop the event** rather than block.
3. **Audio callback (per buffer, every few ms):**
   1. Pop all pending `KeyPress` items and call `mixer.trigger()` for each.
   2. Zero the output buffer, then `mixer.render(&mut out)`.
   3. Duplicate mono samples across the device's channel count.
   4. Record stats in atomics.
4. **Shutdown:** Ctrl+C sets an atomic flag; main drops the stream; process exits 0.

### Why one thread per device?

It is the simplest correct model: each `Device::fetch_events()` blocks, and there are usually only 1-3 keyboards. An `epoll`-based single loop is a possible later optimisation, not a v0.1 need.

### Why a queue instead of a mutex?

A mutex can make the audio thread wait for an input thread that was preempted, which causes an audible glitch. A bounded lock-free queue (`crossbeam_queue::ArrayQueue`, multi-producer) never blocks either side.

## 5. Real-time rules for the audio callback

These are hard rules. Break them and you get clicks and pops:

- ❌ No heap allocation (`Vec::push`, `String`, `Box::new`, `format!`)
- ❌ No locks (`Mutex`, `RwLock`), no channels that can block
- ❌ No I/O (`println!`, file reads, syscalls you control)
- ❌ No `unwrap()` or `panic!` on the hot path
- ✅ Preallocated buffers, fixed-size arrays, atomics, plain arithmetic

Printing stats from inside the callback is therefore forbidden; a separate thread reads atomics and prints.

## 6. Mixing

- `trigger()`: find an inactive voice; if none, **steal the voice with the lowest `started`** (the oldest). Set `pos = 0`.
- `render(out)`: for each active voice, add `sample[pos] * gain` into `out`, advance `pos`, deactivate when `pos >= sample.len()`.
- **Clipping:** after summing, clamp to `[-1.0, 1.0]` (v0.1). A soft clipper (`tanh`) is a later nicety.
- **Gain:** `gain = (volume / 100.0)^2`, a cheap taper that feels more even to the ear than linear.

## 7. Audio output configuration

- Use cpal's default host and default output device.
- Request `f32` samples. If the device offers only another format, convert at the edge, outside the mixer.
- Request a **small fixed buffer** (start with 256 frames, about 5 ms at 48 kHz). If the backend rejects it, fall back to the default and say so in `--verbose`.
- Resample the click **once at startup** to the device sample rate (linear interpolation is fine for a short click).
- Linux backends: cpal's ALSA backend works on both plain ALSA and PipeWire (through the PipeWire ALSA plugin). A native PipeWire or JACK host is an optional later experiment if latency disappoints.

## 8. Input handling (Linux)

- Enumerate devices with the `evdev` crate.
- **Keyboard heuristic:** a device counts as a keyboard if it supports key events *and* the `KEY_A` code. This filters out mice, power buttons, and lid switches.
- Works on **X11 and Wayland** because it reads from the kernel, not the display server. (Global-hook libraries built on X11 do not work on Wayland.)
- **Permissions:** `/dev/input/event*` is typically `root:input` with mode `660`. The user must be in the `input` group (log out and in afterwards) or run as root.

### Privacy and security note

Membership in the `input` group lets *any* program running as that user read raw keystrokes from every keyboard. This is the cost of the approach and is why the README explains it. `clack` itself:

- uses key events only to start a sound
- never stores, logs, or transmits key codes
- keeps no key history in memory beyond the queue

## 9. Latency model

```
finger down ─► kernel ─► evdev read ─► queue ─► callback ─► device buffer ─► speaker
   (not measured)  (small)   (~µs)    (<1 ms)   (≤ 1 buffer)   (1-2 buffers)
```

What `--verbose` reports (software estimate only):

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
| `crossbeam-queue` | Lock-free bounded queue |
| `ctrlc` | Clean Ctrl+C handling |
| `anyhow` | Error handling in `main` |

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
    ├── cli.rs
    ├── sample.rs
    ├── input.rs
    ├── mixer.rs
    ├── audio.rs
    └── stats.rs
```

## 12. Failure modes

| Failure | Behaviour |
|---------|-----------|
| No keyboards found | Print an error with permission hint; exit 1. |
| Permission denied on `/dev/input` | Print setup hint; exit 1. |
| No audio output device | Print error; exit 1. |
| Keyboard unplugged mid-run | That reader thread exits quietly; others keep running (v0.1: no hotplug). |
| Queue full | Drop the key event silently (count it in verbose stats). |
| Buffer size rejected | Fall back to default buffer; note it in verbose output. |
| Audio stream error | Print error; exit 1. |

## 13. Testing strategy

- **Unit tests** (no hardware): mixer voice allocation, voice stealing, gain math, clipping, resampler output length.
- **Manual tests:** press-and-hold, fast typing, two keyboards, volume 0 / 30 / 100, Ctrl+C.
- **Latency:** the `--verbose` estimate, compared against a rough sanity check (record speaker plus keyboard with a phone and look at the waveform gap) if you're curious.

## 14. Extension points (later)

- Release samples: also emit `KeyRelease` messages.
- Per-key or per-region sounds: add a key code to `KeyPress`.
- Sound packs: replace the single `sample` with a lookup table.
- Other OSes: hide input capture behind a trait; the mixer and audio layers are already platform-neutral.
