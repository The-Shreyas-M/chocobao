# TODO

## STATUS 12 Nov 2026 (v3.3.2, flashed and tested on the board)
Working and checked on hardware: ESP32 -> BIO capture -> Notary signature -> verify.py (VALID / INVALID), MMU demo (sandbox killed, key intact, second run says restart), settings saved across restarts, 18 games, 4 songs.
Open: record the 2-minute video; optional GitHub release with the three .uf2 files; optional next step: move the Notary signer into its own process; unknown: why the board sometimes lands in the bootloader right after flashing (type `boot` on COM11 or press RESET).

## SAVING SETTINGS (v3.0.2)
`chocobao/src/store.rs` now saves through the keystore service's app key slots (the first attempt, mapping the ReRAM controller from the app, crashed the whole app at boot). It runs on its own thread and is on by default; hold BACK while the board starts to skip it. To confirm: change Settings -> Screensaver, wait a second, Power -> Restart, check it is still changed.

## STATUS 11 Nov 2026 (firmware v2.9.0, flashed) - read this first
Built and flashed: 27 games (Doom 5 levels first, Jetpack Joyride, Road Rush, Subway Run, Ninja Run, Star Drifter, Space Invaders, Tetris, Pac-Man...), Music > Tracks (5 songs from MIDI) and Tools (metronome), Notary + verify.py, TRNG health, button scope, CPU stats, record-player screen, 3 savers. OLED bus 1 MHz, frame pacing, self-healing screen, BIO notes queued ahead.
Dropped by the owner: hardware wallet (no room next to the games), isolated-Notary process, glitch/tamper sensors, AM radio.
Open items:
- [ ] Owner decisions: licence (suggest Apache-2.0); MMU demo (kill a misbehaving game thread) yes/no; should Seed words be typed instead of shown; which GitHub account to publish from.
- [ ] Deliverables (nothing written yet): README with reproduction steps, protocol spec for Notary, 2-minute video (owner records), write-up that states the limits. No new features after 12 Dec; deadline 15 Dec 21:00 ET.
- [ ] Flash space: apps.img is about 793 KB of 870 KB. Anything big needs compression or removal.
- [ ] BUILD_GUIDE / README still mention older menus in places.
- [ ] Unverified on the real board: Ninja Run, Star Drifter, Road Rush, Subway Run, Jetpack vehicles, AURA playback after the queue-ahead sound rewrite.


Status key: [ ] open, [~] in progress, [x] done. Rewritten 10 Nov 2026 (firmware v1.1). Older items are kept at the bottom.

## Findings so far (read first)
- v0.8+ is stable: 18+ min of use without a freeze (before: every 0.5-2.5 min). Cause was most likely the unbounded spin in the HAL I2C driver (patch: `patches/xous-i2c-bounded-spin.patch`). Keep `tools/watch_diag.py` as the regression test after every big change (`diag_log.txt`, git-ignored).
- One app-loop stall of 2.4 s and an OLED push of 377 ms were seen: probably typing a long password over USB or a full-screen wipe. Watch `loopmax`/`pushmax`.
- Flash: hold PROG, tap RESET, release PROG, then `tools/flash_auto.py loader.uf2 xous.uf2 apps.uf2` (always all three after a rebuild). `bootwait` decision still open (hands-free flashing).

## Done (needs the user's eyes on the real screen)
- [x] Menu: Utilities / Games / Lab / About with icons and one-line descriptions; first-open help screens; wipe transitions.
- [x] Utilities: Password maker (17 types: UP/DOWN type, LEFT/RIGHT length, OK new, hold OK type), Passphrase, PIN maker, Dice and coin (10 dice), Random number, Stopwatch, Timer with alarm, Entropy meter.
- [x] Games (9): Mempool Dodger, Snake, Pong, Flappy Bao, Simon, Reaction, ChocoDoom, **Pac-Man**, Magic 8-ball. BACK once pauses, BACK twice quits. Tunes per game.
- [x] Lab: Button test, Buzzer test, SOS, Sound on/off, Diagnostics. Bootloader item removed.
- [x] Boot/Home: chocolate keeps pouring over a bao (no steam). Screensaver after 45 s idle (moving bun, dark after 2.5 min more); any button wakes and resumes, games paused.
- [x] Neumorphic look started: raised cards for menu rows and password panels, sunken panel for the secret. Passphrase wraps by words.
- [x] BUILD_GUIDE.md rewritten (all connections). HANDOFF.md rewritten.

## v2.0.0 (built 10 Nov night, NOT flashed, nothing seen on the real screen yet): the whole backlog below was implemented
- [x] Home: bao now visibly turns (width breathes, seeds travel around), microwave/bar as asked.
- [x] 11 new games (`games2.rs`): Breakout, Block Drop, 2048, Tic-Tac-Toe, Connect 4, Memory Pairs, Whack-a-Mole, Minesweeper (hold OK = flag), Space Shooter, Lunar Lander, Maze Runner. PC smoke-tested only.
- [x] ChocoDoom round 2: brick walls, floor/ceiling dither, minimap, how-to overlay, 2 levels (OK after a win starts the other level), autofire.
- [x] Tools (`tools2.rs`): Calculator, Unit converter, Base converter, Metronome, Habit counter, Morse trainer, Text presets (edit the PRESETS list, hold OK types), Flashlight.
- [x] Music: Chord synth (4 voices time-shared on one pin). Lab: Button scope (CPU sampled), TRNG health, Notary.
- [x] Notary (in-app): Ed25519 key from the TRNG kept in RAM only, each UP/DOWN/LEFT/RIGHT press signs "CHOCONOTARY1|n|ms|event"; hold OK types `msg|sig|pub`; `tools/verify.py` checks it on the PC (pure Python, RFC 8032 vector tested).
- [x] Settings: Idle time, Invert screen. Savers: 13 eye moods (crying, laughing, bored added), Bao and bar, Surprise me.
- [ ] NOT done: isolated Notary process + MMU "game reads the key and is killed" demo (needs a second Xous process/service and kernel plumbing); glitch/tamper sensors (the SENSORC registers belong to the loader/kernel and sit next to one-way wipe counters, deliberately not touched); display contrast; custom text input; tunes (still need the MIDI files for `tools/midi2tune.py`); countdown-to-date (the clock has no date); real 4-pin 4-voice synth (needs wiring).
- Flash size: apps.img is about 685 KB of roughly 870 KB, so not much room is left.

## Backlog from the 10 Nov evening session (user's list, do in this order)
Done in v1.5.0 (built, NOT flashed yet, unseen on screen): Doom smooth + responsive (time-based movement with acceleration, shots every 0.16 s, hold OK to autofire, wider hit cone, button poll 4 ms), "by Shreyas" in About / home / boot, new chip-themed boot (chip draws itself, 4 cores light, padlock, TRNG noise settles into the title), landscape microwave + shaded 3D-ish spinning bao + 2-column chocolate bar, Eyes saver with 10 moods + mouth that eats a chocolate bar + drifting pixels (no more blank screen), Piano removed, BUILD_GUIDE rewritten with a table per part and ASCII wiring (speaker section 9), `tools/midi2tune.py`.
- [ ] **Flash v1.5.0 and check:** Doom feel (turn, sprint, shoot an enemy that appears), boot animation, home, eyes + feeding, About name. If Doom is still laggy: measure with `diag` (loopmax, pushmax); the OLED push (about 30 ms per frame over I2C) may be the limit, then raise the I2C clock or draw fewer pixels.
- [ ] **Tunes from the sheets:** the music is still my by-ear guess and sounds wrong. Get a MIDI of each sheet (MuseScore: Download -> MIDI), run `python tools/midi2tune.py song.mid` to list tracks, then `--track N --name BLINDING_LIGHTS --bars 1-16 > out.rs` and paste into `chocobao/src/music.rs`. Sheets given: Blinding Lights (168 bpm, 3 flats, Perfect Easy piano arr.) and Attention (100 bpm, 1 flat, arr. Sadie King).
- [ ] **More games** (ideas: Breakout, Tetris-like blocks, Minesweeper, 2048, Tic-tac-toe vs computer, Maze runner, Space shooter, Lunar lander, Whack-a-mole, Memory pairs, Connect 4).
- [ ] **More tools:** calculator, unit converter, coin/dice presets, morse trainer, metronome, hex/binary converter, countdown to a date, habit counter, flashlight (full-screen white), text typer presets (type your email/address on hold).
- [ ] **More customisation:** per-user name on the home screen, screensaver idle time, display contrast/brightness, key-repeat speed, sound volume steps (duty/pitch), start-up animation on/off, theme (invert screen), choose the first screen. Settings in RAM only unless the user asks (ReRAM write endurance).
- [ ] **More screensaver expressions / ideas:** extra eye moods (crying, laughing, sneezing, bored), mouth lip-sync to music, bao eating the chocolate bar.
- [ ] **Speaker:** user has not wired it. When they do: follow BUILD_GUIDE section 9 (divider, knob at minimum, big 4 ohm speaker first). Never wire a speaker directly to a pin.
- [ ] **Chip showcase features (the user asked, none built yet):**
  - 4-voice synth, one BIO core per voice (needs 4 pins into a resistor mixer, then the amp).
  - BIO "scope": a BIO core samples our own pins and draws the waveform.
  - TRNG health screen: show the raw bits, bias, runs test.
  - The Notary: BIO timestamps events, an isolated process signs with Ed25519, PC `verify.py`, MMU kills a game that tries to read the key.
  - Glitch / tamper sensors: check whether the chip exposes them to apps (read `libs/bao1x-hal` sensor and glitch-detect registers first, do not assume).
  - Idea, unverified: AM "radio transmitter" demo: a BIO core pulses a pin near 1 MHz amplitude-modulated by a tune, a nearby AM radio may pick it up from a few cm. Receiving FM/AM is not realistic with this chip.
- [ ] **Passwords never shown:** done for Password/Passphrase/PIN. Seed words still shows the words (needed to write them down): ask the user if those should be typed too.

## Round 1b: polish after seeing it on the real screen
- [ ] Flash v1.1 and collect feedback on: boot animation, home, icons, help screens, wipe, screensaver, neumorphic panels, Pac-Man speed, Doom.
- [ ] Neumorphic style everywhere (Dice, Random, Timer, Stopwatch, Entropy, Lab screens); consistent header/footer; nicer scroll indicator.
- [ ] Richer per-app intro animations; animated menu icons for the selected row.
- [ ] Make "hold OK to type" visible (progress bar while holding).

## Round 2: ChocoDoom looks 3D and readable
- [ ] Textured walls (brick/stone that scales), floor and ceiling (dither gradient), distance fog, edge lines.
- [ ] Better enemies (head/eyes/arms, walk animation, hit flash), pickups, doors, minimap, weapon bob, damage vignette, level 2.
- [ ] Tutorial overlay on first run. Frame-time check with `diag`.

## Round 3: show off the silicon
- [x] (built v1.2, unseen on screen, `mandel.rs`, Lab > Mandelbrot) **Mandelbrot zoom** (fixed-point on the CPU, progressive render, smooth zoom, dithered palette). Stretch: split rows over BIO cores (BIO has no hardware multiply).
- [x] (built v1.2, `seed.rs`, Utilities > Seed words, matches reference vectors; never typed/saved) **Seed words with valid BIP-39 checksum** (12/24 words, hardware SHA) in Utilities. TRNG + hash engine, no extra hardware.
- [ ] **4-voice chiptune synth** (one BIO core per voice, needs a mixer or 4 pins; or time-multiplex on one pin).
- [ ] **BIO scope / self-sniffer**: sample our own pins (buttons, buzzer, maybe OLED lines) with a BIO core and draw the waveform. No extra hardware.
- [ ] **BIO I2C master for the OLED** (take the display off the HAL I2C service; cleanest stability fix).
- [ ] Crypto benchmarks (SHA/AES/Ed25519 timing), TRNG health screen (Xous TRNG health tests).
- [ ] Skipped (needs hardware we don't have): IR replay, WS2812 effects, ESP32 sensor node.

## Ideas from the pasted project list (simple words)
- [ ] Seed/dice generator: already mostly done; add checksum seeds (above).
- [ ] Canary / dead-man timer: a countdown that must be reset with a button pattern; RAM only, shows an alert when it expires. **Never wipe the chip** (`self_destruct` is real and permanent).
- [ ] TOTP vault: needs the time (no battery clock; enter it by hand) and a saved secret (ReRAM write only on an explicit "save"). Medium effort, only if time allows.
- [ ] Logic sniffer: only as a self-sniffer demo (above).

## Round 4: hackathon deliverables (see STRATEGY.md)
- [ ] Notary: BIO event capture, Ed25519 signing in an isolated process, PC `verify.py`, MMU "game tries to read the key" demo. Event source = a button / piezo, no ESP32.
- [ ] README with reproduction steps, protocol spec, 2-minute video, write-up with limits spelled out, licence (suggest Apache-2.0). No new features after 12 Dec.

## Hardware notes
- [ ] Buzzer is wired + to PB3 (pin 32) through 330 ohm and - to GND (works). Loud option: - to PB1 (pin 29), remove the resistor (piezo disc only).
- [ ] 10 uF across 3V3/GND not fitted (optional, display is fine without).
- [ ] Never drive an 8 ohm speaker from a pin.

## Housekeeping
- [ ] Update README.md (button map, menus, flash flow, safe mode) and keep HANDOFF.md / BUILD_GUIDE.md current after every change.
- [ ] Possibly report upstream: unbounded spin in `i2c_write_async`/`i2c_read_async`; console burst bug; Dvorak keymap issue (not posted anywhere).
- [ ] Decide on `bootwait` (see HANDOFF.md section 5).

---

# Older items (7 Nov, partly obsolete)
Status key: [ ] open, [~] in progress, [x] done. Updated 7 Nov 2026.

## Flashing and tooling (current focus)
- [x] Serial flash speed: fixed (65 s -> 1.0-1.2 s). Pipelining does not work; see NOTES.md.
- [~] **App-side `bootloader` command** (added to chocobao, built, UNTESTED on hardware) (out-of-tree app, not edits to `repos/`). Call the HAL `reboot()` (`libs/bao1x-hal/src/clocks.rs`, writes `SFR_RCURST0`). Verify it lands in boot1 given `bootwait` is Enabled. This closes the loop: command -> reboot -> flash over serial -> `boot`, no replug.
- [ ] **Fix serial `boot` reliability**: app sometimes never enumerates after `boot` (Windows stale-device limbo). Test whether a PROG press recovers it, whether a host-side port reset or USB re-enumeration helps without admin, and time the good case.
- [x] ~~`bao` CLI~~ DROPPED: tools-bao/bao.py and bao-vscode-ext already do ports, monitor, flash, boot, doctor, app create, build-flash-monitor. Only build the missing pieces (see NOTES.md overlap check): pin-plan checker, serial flasher, app-side reboot command.
- [ ] Make `baoport.py` console reads reliable (garbled replies from fixed read windows).
- [ ] Decide naming and repo layout for the toolkit; keep our work out of the `repos/` clones.

## Storage (ReRAM-native deniable store)
- [x] ReRAM endurance: about 10k r/w cycles per bunnie (xous-core #890). Still to find: write granularity.
- [ ] Re-evaluate the store design under 10k-cycle endurance and against Supermagnum's wear-aware counter log work (xous-core #937) before committing.
- [ ] Storage probe: harmless write in the unused tail of the app region (0x3DA000 is the end of writable space; never touch boot, kernel or data-slot ranges). Confirm a std app can map and write ReRAM.
- [ ] Threat model + design doc for a ReRAM-native plausibly-deniable store with a PDDB-like API; get it reviewed before claiming anything.

## ChocoBao (see PLAN.md)
- [x] Dice + button typing: flashed and verified on hardware (8 Nov).
- [ ] Order 0.96" SSD1306 OLED.
- [ ] OLED driver, then Synth, Notary, Status, Mempool Dodger.

## BIO next steps
- [ ] Decide first wired test: LED blink (PB4 or PC0) with a resistor, then a 1-bit PDM tone into the PAM8403 through an RC filter. Needs headers soldered (or jumper wires held in the holes for a short test).
- [ ] Pin plan for 4 voices: PC0..PC3 (BIO16-19), contiguous.
- [ ] Write our own Xous app using bio-lib to claim all four cores (console loader claims only Core0).

## Hardware
- [ ] Buy the 0.96" I2C OLED (check controller: SSD1306 vs SH1106).
- [ ] Pin plan from pins.csv (avoid PC13 = PROG, PB13/PB14 = console UART, I2C0 = PB11/PB12); solder headers only after the plan exists.
- [ ] If sensors are used: piezo clamp, check heart-sensor pull-up voltage <= 3.3 V, common ground for the PAM8403.

## Decisions and research
- [ ] Settle the flagship: toolkit + ReRAM store, and what demo shows it ("blank board to working in 60 seconds", store with a hidden layer).
- [ ] Check what the community MicroPython port and `bao-vscode-ext` already cover so we don't duplicate them.
- [ ] Possible later: BIO audio output driver, I2C OLED driver, upstream PRs to xous-core.

## Done
- [x] Toolchain set up on Windows (GNU Rust host, Xous sysroot, pipx, ziglang, bio-loader wrapper).
- [x] First build and flash (dabao-console), console verified.
- [x] Read-only `audit` and `bootwait check` (bootwait = Enable).
- [x] Serial flashing proven: `tools/baoflash.py`, 1059 blocks acknowledged, `boot` started the app without PROG.
- [x] PROCEDURE.md, LEARN.md, ideas.md, NOTES.md written.
- [x] v2.1.0: tunes converted from the user's MIDI files, fuller buzzer (octave shimmer), AM beacon dropped, BUILD_GUIDE rewritten with PCB labels.
