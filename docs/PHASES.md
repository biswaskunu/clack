# PHASES: clack

Build plan for v0.1. Each phase has a **goal**, a **task list**, and a **done-state**. Don't start the next phase until the current done-state is true, except where noted.

**Budget:** 1-5 hours/week, so roughly 15-18 hours total, about 4-5 calendar weeks at an easy pace. Phases 1-3 are the actual "weekend toy"; everything after is polish that makes it good.

| Phase | Name | Est. hours | Status |
|-------|------|-----------|--------|
| 0 | Setup | 1 | Done |
| 1 | Read keys | 2-3 | Done (Linux) |
| 2 | Make a sound | 3-4 | Done |
| 3 | Volume flag | 1 | Done |
| 4 | Polyphony | 3 | Done |
| 5 | Latency stats | 2-3 | **Not started** |
| 6 | Ship v0.1 | 2 | **Partly done** (see below) |
| W | Windows backend | 3-4 (added) | **Compiles, untested at runtime** |

**Milestone "toy works" = end of Phase 3.** Reached.

---

## Phase 0: Setup (about 1 h) ✅

**Goal:** a repo that builds and a place to put things.

- [x] `cargo new clack`, commit docs into `docs/`
- [x] Add the dependencies from `ARCHITECTURE.md` section 10
- [x] Add yourself to the `input` group, log out and back in, verify with `groups`
- [x] Pick and download a CC0 click sample into `assets/press.wav`
- [ ] Note the sample's source in `assets/LICENSE.md` (still says "please add source if known")
- [x] `.gitignore`, first commit

**Done when:** `cargo build` passes, `groups` shows `input`, the sample plays in any audio player.

---

## Phase 1: Read keys (2-3 h) ✅ Linux

**Goal:** prove you can see every key press, on every keyboard.

- [x] List devices under `/dev/input` and print each one's name
- [x] Filter to keyboards (supports key events and `KEY_A`)
- [x] Read events from one keyboard; print "press" for `value == 1` only
- [x] Spawn one thread per keyboard
- [x] Friendly error when permission is denied

**Done when:** typing on any connected keyboard prints one "press" line per key, and holding a key prints exactly one.

**Learn:** how evdev events work (type, code, value), why `value == 2` is auto-repeat.

---

## Phase 2: Make a sound (3-4 h) ✅

**Goal:** a click plays on a keypress. Latency not yet optimised.

- [x] Decode the WAV with `hound` into `Vec<f32>` mono
- [x] Open a cpal output stream on the default device
- [x] Resample the sample to the device rate (linear interpolation) once at startup
- [x] Hard-code a single "play the sample once" voice triggered by a key press
- [x] Send key presses to the audio callback through the lock-free queue (no mutex)

**Done when:** pressing a key makes the click play, with no crackle at normal typing speed.

**Learn:** the callback model, why you can't allocate or lock in it, sample-rate conversion.

---

## Phase 3: Volume flag (1 h) ✅

**Goal:** `--volume` works.

- [x] Add `clap` with `--volume <0-100>` (default 50), range-validated
- [x] Convert to gain with the squared taper from `DESIGN.md` D5
- [x] Apply gain in the mixer
- [x] Print the one-line startup message

**Done when:** `--volume 0` is silent, `--volume 100` is loud, `--volume 101` is rejected with a clear error.

> 🎉 **Milestone: the toy works.**

---

## Phase 4: Polyphony (about 3 h) ✅

**Goal:** fast typing sounds right.

- [x] Replace the single voice with a fixed pool of 32 (`Mixer` in `mixer.rs`)
- [x] Voice stealing: when the pool is full, take the oldest
- [x] Sum voices, then clamp to `[-1, 1]`
- [x] **Unit tests** for: allocation, stealing order, gain, clipping, voice finishing
- [x] Move all mixing out of `audio.rs` into the pure `Mixer`

**Done when:** `cargo test` passes, and mashing keys for 30 seconds sounds clean with no dropped clicks.

*(Mashing test: confirm by ear on Linux. Not yet recorded.)*

---

## Phase 5: Latency stats (2-3 h) ⏳ Not started

**Goal:** see the number, and make `--verbose` trustworthy.

- [ ] Timestamp each `KeyPress` with `Instant::now()` in the input thread
- [ ] In the callback, compute queue wait and read cpal's output latency estimate
- [ ] Store sum, count, max, and a small fixed histogram in atomics
- [ ] A stats thread prints one line per second while keys are active (format in `DESIGN.md`)
- [ ] Try requesting a smaller fixed buffer (128, 256, 512) and compare numbers
- [ ] Record your measurements in the README

**Done when:** `--verbose` prints sensible numbers, and you have picked a default buffer size based on data.

**Learn:** latency budgeting, atomics, measuring honestly. Remember it's a software estimate.

> **Note:** v0.1.0 was tagged before this phase. The `--verbose` flag is parsed but prints nothing. Either finish Phase 5 for v0.1.1, or document the flag as unimplemented until then.

---

## Phase 6: Ship v0.1 (about 2 h) ⏳ Partly done

**Goal:** a stranger can use it.

- [ ] Bundle the sample with `include_bytes!` ✅ (already done)
- [ ] Clean error messages for every failure mode in `ARCHITECTURE.md` section 13 (Windows Raw Input registration failure is missing)
- [x] Ctrl+C handling with `ctrlc`, exit code 0
- [ ] Manual test pass: two keyboards, hold-key, volume 0 / 30 / 100, Ctrl+C (Linux). Not yet recorded.
- [x] Write the README (install, permissions and the privacy note, usage)
- [ ] README measured latency section (depends on Phase 5)
- [x] `cargo clippy` and `cargo fmt` clean (confirm before tagging a new version)
- [x] Release workflow builds Linux and Windows, packages both, and publishes checksums on tag
- [x] Tag `v0.1.0`

**Done when:** every success metric in `PRD.md` section 9 is checked off. Currently 3 of 6 are confirmed.

---

## Windows backend (added during Phase 6) ⏳ Compiles, untested

**Goal:** Windows x86_64 binary using Raw Input.

- [x] Add `windows-sys` under `[target.'cfg(windows)'.dependencies]` with the required feature flags (including `Win32_Graphics_Gdi`)
- [x] Platform dispatch in `input/mod.rs`
- [x] Raw Input backend in `input/windows.rs` (hidden message-only window, `WM_INPUT` loop)
- [x] `cargo check --target x86_64-pc-windows-msvc` passes
- [x] Release workflow builds the Windows binary in CI
- [ ] Replace `static mut PRESS_TX` with `OnceLock<Sender<()>>`
- [ ] Manual test on a Windows machine: typing produces clicks, Ctrl+C exits
- [ ] Filter auto-repeat per virtual key code (required before calling Windows "supported")
- [ ] Handle Raw Input registration failure with an error message and exit 1

**Done when:** the manual Windows test passes and auto-repeat is filtered. Until then, label Windows as experimental in release notes.

---

## Backlog (not scheduled)

Pick from this only after v0.1.0 is tagged and Phase 5 is done:

- [ ] Release sound on key up (FR-9)
- [ ] `--sound <file.wav>` custom sample (FR-10)
- [ ] Skip duplicate virtual devices from remappers
- [ ] Soft clipping instead of hard clamp
- [ ] Mechvibes pack compatibility
- [ ] Per-key sounds (space and enter differ)
- [ ] Typing-speed dynamics
- [ ] Hotkeys
- [ ] macOS input backend
- [ ] Native PipeWire host if ALSA latency disappoints

## Rules for scope

1. If it isn't in a phase, it isn't in v0.1.
2. If a phase runs over 2x its estimate, stop and cut scope, don't extend the schedule.
3. New ideas go to the backlog the moment you have them, then you go back to the current task.
