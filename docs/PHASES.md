# PHASES: clack

Build plan for v0.1. Each phase has a **goal**, a **task list**, and a **done-state**. Don't start the next phase until the current done-state is true.

**Budget:** 1-5 hours/week, so roughly 15-18 hours total, about 4-5 calendar weeks at an easy pace. Phases 1-3 are the actual "weekend toy"; everything after is polish that makes it good.

| Phase | Name | Est. hours | Cumulative |
|-------|------|-----------|-----------|
| 0 | Setup | 1 | 1 |
| 1 | Read keys | 2-3 | 4 |
| 2 | Make a sound | 3-4 | 8 |
| 3 | Volume flag | 1 | 9 |
| 4 | Polyphony | 3 | 12 |
| 5 | Latency stats | 2-3 | 15 |
| 6 | Ship v0.1 | 2 | 17 |

**Milestone "toy works" = end of Phase 3.** Celebrate it; you have a working product.

---

## Phase 0: Setup (about 1 h)

**Goal:** a repo that builds and a place to put things.

- [ ] `cargo new clack`, commit docs into `docs/`
- [ ] Add the dependencies from `ARCHITECTURE.md` section 10
- [ ] Add yourself to the `input` group, log out and back in, verify with `groups`
- [ ] Pick and download a CC0 click sample into `assets/press.wav`; note the source in `assets/LICENSE.md`
- [ ] `.gitignore`, first commit

**Done when:** `cargo build` passes, `groups` shows `input`, the sample plays in any audio player.

---

## Phase 1: Read keys (2-3 h)

**Goal:** prove you can see every key press, on every keyboard.

- [ ] List devices under `/dev/input` and print each one's name
- [ ] Filter to keyboards (supports key events and `KEY_A`)
- [ ] Read events from one keyboard; print "press" for `value == 1` only
- [ ] Spawn one thread per keyboard
- [ ] Friendly error when permission is denied (use text from `DESIGN.md`)

**Done when:** typing on any connected keyboard prints one "press" line per key, and holding a key prints exactly one.

**Learn:** how evdev events work (type, code, value), why `value == 2` is auto-repeat.

---

## Phase 2: Make a sound (3-4 h)

**Goal:** a click plays on a keypress. Latency not yet optimised.

- [ ] Decode the WAV with `hound` into `Vec<f32>` mono
- [ ] Open a cpal output stream on the default device; print its sample rate and channels
- [ ] Resample the sample to the device rate (linear interpolation) once at startup
- [ ] Hard-code a single "play the sample once" voice triggered by a key press
- [ ] Send key presses to the audio callback through the lock-free queue (no mutex)

**Done when:** pressing a key makes the click play, with no crackle at normal typing speed.

**Learn:** the callback model, why you can't allocate or lock in it, sample-rate conversion.

---

## Phase 3: Volume flag (1 h)

**Goal:** `--volume` works.

- [ ] Add `clap` with `--volume <0-100>` (default 50), range-validated
- [ ] Convert to gain with the squared taper from `DESIGN.md` D5
- [ ] Apply gain in the mixer
- [ ] Print the one-line startup message

**Done when:** `--volume 0` is silent, `--volume 100` is loud, `--volume 101` is rejected with a clear error.

> 🎉 **Milestone: the toy works.** Tag `v0.0.1` if you like.

---

## Phase 4: Polyphony (about 3 h)

**Goal:** fast typing sounds right.

- [ ] Replace the single voice with a fixed pool of 32 (`Mixer` in `mixer.rs`)
- [ ] Voice stealing: when the pool is full, take the oldest
- [ ] Sum voices, then clamp to `[-1, 1]`
- [ ] **Unit tests** for: allocation, stealing order, gain, clipping, voice finishing
- [ ] Move all mixing out of `audio.rs` into the pure `Mixer` so it's testable without a sound card

**Done when:** `cargo test` passes, and mashing keys for 30 seconds sounds clean with no dropped clicks.

**Learn:** real-time-safe data structures, designing for testability.

---

## Phase 5: Latency stats (2-3 h)

**Goal:** see the number, and make `--verbose` trustworthy.

- [ ] Timestamp each `KeyPress` with `Instant::now()` in the input thread
- [ ] In the callback, compute queue wait and read cpal's output latency estimate
- [ ] Store sum, count, max, and a small fixed histogram in atomics
- [ ] A stats thread prints one line per second while keys are active (format in `DESIGN.md`)
- [ ] Try requesting a smaller fixed buffer (128, 256, 512) and compare numbers
- [ ] Record your measurements in the README

**Done when:** `--verbose` prints sensible numbers, and you have picked a default buffer size based on data.

**Learn:** latency budgeting, atomics, measuring honestly. Remember it's a software estimate.

---

## Phase 6: Ship v0.1 (about 2 h)

**Goal:** a stranger can use it.

- [ ] Bundle the sample with `include_bytes!`
- [ ] Clean error messages for every failure mode in `ARCHITECTURE.md` section 12
- [ ] Ctrl+C handling with `ctrlc`, exit code 0
- [ ] Manual test pass: two keyboards, hold-key, volume 0 / 30 / 100, Ctrl+C
- [ ] Write the README (install, permissions and the privacy note, usage, measured latency)
- [ ] `cargo clippy` and `cargo fmt` clean
- [ ] Tag `v0.1.0`; optionally publish to crates.io or attach a release binary

**Done when:** every success metric in `PRD.md` section 9 is checked off.

---

## Backlog (not scheduled)

Pick from this only after v0.1.0 is tagged:

- [ ] Release sound on key up (FR-9)
- [ ] `--sound <file.wav>` custom sample (FR-10)
- [ ] Skip duplicate virtual devices from remappers
- [ ] Soft clipping instead of hard clamp
- [ ] Mechvibes pack compatibility
- [ ] Per-key sounds (space and enter differ)
- [ ] Typing-speed dynamics
- [ ] Hotkeys
- [ ] macOS and Windows input backends
- [ ] Native PipeWire host if ALSA latency disappoints

## Rules for scope

1. If it isn't in a phase, it isn't in v0.1.
2. If a phase runs over 2x its estimate, stop and cut scope, don't extend the schedule.
3. New ideas go to the backlog the moment you have them, then you go back to the current task.
