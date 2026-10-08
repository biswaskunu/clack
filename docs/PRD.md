# PRD: clack (working name)

**Status:** Draft v0.1
**Owner:** Biswash
**Type:** Fun side project / weekend toy
**Platform:** Linux only (v0.1)

---

## 1. Summary

`clack` is a tiny command-line tool that plays mechanical-keyboard click sounds through your speakers whenever you type, on *any* keyboard. You start it with one command, optionally set the volume, and stop it with Ctrl+C.

```
clack --volume 60
```

The defining quality is **low latency**: the click must feel attached to the keypress. A click that arrives late feels worse than no click at all.

## 2. Problem

- Membrane and laptop keyboards feel quiet and mushy; some people like the audible feedback of a mechanical board.
- Existing tools (e.g. Mechvibes) are Electron apps: heavy on RAM, GUI-first, and often criticised for audio lag.
- There is no minimal, native, scriptable option on Linux.

## 3. Target user

- Primary: the author (it's a fun project and he'll use it daily).
- Secondary: Linux users on quiet keyboards, streamers, and people who like the "thock".

## 4. Goals

| ID | Goal |
|----|------|
| G1 | A click plays on every key press, on every connected keyboard. |
| G2 | Keypress-to-audio software latency stays low (target p95 under 15 ms, see section 8). |
| G3 | Volume is set once at launch via a CLI flag. |
| G4 | Single static-ish binary, no GUI, no config file, near-zero idle CPU and small RAM. |
| G5 | Finishable: v0.1 should be achievable in roughly 15-18 hours total. |

## 5. Non-goals (v0.1)

Deliberately out of scope. Say no to these until v0.1 ships:

- Hotkeys (mute, volume up/down), tray icon, or any GUI
- Windows and macOS support
- Multiple sound packs, per-key sounds, or pack format compatibility
- Pitch or velocity variation, "dynamics" based on typing speed
- Config files, daemon or systemd mode, hotplug of keyboards
- Recording or logging keystrokes (never, in any version)

## 6. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-1 | On start, discover all keyboard input devices under `/dev/input`. | Must |
| FR-2 | Play a click sample on each key **press** event. | Must |
| FR-3 | Ignore auto-repeat events (holding a key must not machine-gun clicks). | Must |
| FR-4 | `--volume <0-100>` sets output gain; default 50; out-of-range values are rejected with a clear error. | Must |
| FR-5 | Overlapping presses must layer (fast typing never cuts a click short or drops it). | Must |
| FR-6 | Ctrl+C exits cleanly with exit code 0. | Must |
| FR-7 | A bundled default click sample so the binary works with no extra files. | Must |
| FR-8 | `--verbose` prints periodic latency stats (see section 8). | Should |
| FR-9 | Separate release-sound on key up. | Could |
| FR-10 | `--sound <file.wav>` to use a custom sample. | Could |

## 7. CLI specification

```
clack [OPTIONS]

OPTIONS:
  -v, --volume <0-100>   Output volume (default: 50)
      --verbose          Print latency stats once per second
  -h, --help             Print help
  -V, --version          Print version
```

**Exit codes:** `0` normal exit (Ctrl+C), `1` runtime error (no keyboards, no audio device, permission denied), `2` invalid arguments (handled by clap).

Exact user-facing messages live in `DESIGN.md`.

## 8. Non-functional requirements

| ID | Requirement |
|----|-------------|
| NFR-1 | **Latency:** software key-to-callback wait under 1 ms; estimated total (queue wait plus output buffer) p95 under 15 ms on a typical PipeWire or ALSA setup. |
| NFR-2 | **Idle cost:** CPU near 0% while no keys are pressed; RSS under about 20 MB. |
| NFR-3 | **Privacy:** key codes are used only to trigger a sound. They are never written to disk, logged, or sent anywhere. |
| NFR-4 | **Robustness:** the audio callback never blocks, allocates, or panics. |
| NFR-5 | **Zero cost:** only free tools and CC0 or permissively licensed assets. |

> **Honest limit:** the `--verbose` number is a *software estimate*. True acoustic latency (finger to ear) would need a loopback or microphone test, which is out of scope.

## 9. Success metrics

v0.1 is "done" when all of these are true:

1. Typing on the built-in keyboard and a USB keyboard both produce clicks.
2. `clack --volume 0` is silent and `--volume 100` is clearly louder than `--volume 30`.
3. Typing at full speed for 30 seconds produces no dropped or clipped clicks and no audible glitches.
4. `--verbose` shows p95 estimated latency under 15 ms on the author's machine.
5. Holding a key produces exactly one click.
6. A README lets a stranger set up permissions and run it in under 5 minutes.

## 10. Constraints

- **Time:** 1-5 hours per week.
- **Budget:** zero.
- **Stack:** Rust (author's primary stack).
- **Mentor mode:** the author writes the code; the assistant guides and reviews (see `CLAUDE.md`).

## 11. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Reading `/dev/input` needs the `input` group (or root). | First-run failure | Clear error message plus README setup steps. Understand the privacy tradeoff (see `ARCHITECTURE.md`). |
| Key remappers (keyd, kmonad, interception-tools) expose both a real and a virtual device. | Double clicks | Detect and skip duplicates; document as a known issue (see Open Questions). |
| Audio backend ignores the requested small buffer. | Higher latency than target | Request a fixed small buffer, fall back to default, print the actual buffer size in verbose mode. |
| Scope creep ("just one more feature"). | Never ships | Non-goals list above; phases have explicit done-states. |
| Bundled sample licensing. | Legal | Use only CC0 audio; record the source in `assets/LICENSE.md`. |

## 12. Open questions

1. Final name? `clack` is a placeholder; check crates.io availability before publishing.
2. How to handle remapper virtual devices (skip by name, or grab-aware dedupe)?
3. Linear amplitude or squared taper for volume? Starting with squared; revisit by ear.

## 13. Future ideas (not committed)

Release sounds, custom sample packs and Mechvibes pack compatibility, typing-speed dynamics, hotkeys, macOS and Windows, a browser pack-maker.
