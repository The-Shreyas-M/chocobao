# TODO

Status key: [ ] open, [~] in progress, [x] done. Rewritten 10 Nov 2026 (v0.8 state). The older list is kept at the bottom.

## Findings so far (read this first)
- **v0.8 is stable.** After the I2C patch + fewer wakeups, one run lasted 18+ minutes of normal use (btn events 335, 0 panics, 0 failed OLED pushes). Before v0.8 the board froze/restarted every 0.5-2.5 min. Likely cause: unbounded spin in the HAL I2C driver (patch: `patches/xous-i2c-bounded-spin.patch`). Not proven, but the symptom is gone.
- One app-loop stall of 2.4 s and an OLED push of 377 ms were seen once (`loopmax`/`pushmax` in `diag_log.txt`): find out what it was (probably typing a long password over USB, or ChocoDoom). Keep `tools/watch_diag.py` as the regression test.
- Flash flow: **hold PROG, tap RESET, release PROG**, then `tools/flash_auto.py` (it waits for the bootloader and flashes; add loader.uf2 + xous.uf2 only when the kernel/HAL changed).

## Round 1: UI/UX overhaul (requested 10 Nov)
- [ ] Restructure the main menu: **Utilities**, **Games**, **Lab** (silicon showcase), **About**. (Dice, password maker and stopwatch are utilities, not games.)
- [ ] **Utilities:** Password maker, Passphrase (6 words), PIN, Dice and coin, Random number, Stopwatch, Countdown timer, Entropy meter, Button test, Buzzer test, more (checksum/base converter, Morse beeper, calculator?).
- [ ] **Games:** Mempool Dodger, Snake, Pong, Flappy Bao, Simon, Reaction, ChocoDoom, **Magic 8-ball** (moved here).
- [ ] **Password maker** (rename from "Site-rules"): a clear screen with arrows. Show "^ v TYPE" (change password type), "< > LEN" (length), "OK new", "hold OK type it". Bring back an explicit **Type password** action. Many more types: everything, letters+digits, safe symbols, letters only, lowercase+digits, uppercase+digits, digits (PIN), readable (no 0/O/1/l), hex, base32, base58, passphrase styles, "no symbols but one digit", etc.
- [ ] **Dice screen:** label "DICE TYPE" with up/down arrow glyphs; show the result big and a roll history.
- [ ] **Remove "Bootloader" from Tools** (and the console command) since flashing is PROG+RESET. Update flash_auto.py/README/HANDOFF.
- [ ] **Icons + help:** every app gets an 8x8/16x16 icon in the menu and a one-screen "how to use" (shown on first open, `?`/hold OK to show again): which buttons do what, drawn with arrow glyphs.
- [ ] Screen transitions (slide/wipe) and a cleaner, consistent header/footer layout; scroll indicators; selected-item highlight.
- [ ] Better animations: richer boot logo, animated home screen (steam, blinking face, idle tricks), small per-app intro animations.

## Round 2: ChocoDoom looks 3D and readable
- [ ] Textured walls (brick/stone pattern that scales with distance), clear floor and ceiling (dither gradient), distance fog, wall-edge lines.
- [ ] Better enemy sprites (head/eyes/arms, walk animation, hit flash), pickups (health/ammo), a door or two, minimap corner, weapon bob and muzzle flash, damage vignette.
- [ ] Tutorial overlay the first time: "UP/DOWN walk, LEFT/RIGHT turn, OK shoot" with arrow glyphs. Maybe a second level.
- [ ] Performance check (frame time via `diag`), keep the display bus calm.

## Round 3: show off the silicon (the four BIO cores and friends)
- [ ] **Mandelbrot zoom** (fixed-point on the CPU first: 128x64, progressive render, smooth zoom animation, palette via dithering). Stretch: split the image over BIO cores (note: BIO cores have no hardware multiply, so use shift-add or lookup tables).
- [ ] **4-voice chiptune synth**: one BIO core per voice (needs 4 pins or a mixer); play tunes, keyboard mode with the buttons.
- [ ] **BIO logic analyser / signal scope** screen; **IR remote learn + replay** (bio/sw/nec_rx exists); **WS2812 LED effects**.
- [ ] **BIO I2C master for the OLED** (take the display off the HAL I2C service completely; cleanest fix for any residual bus trouble and a good demo).
- [ ] Hardware crypto showcase: SHA/AES speed benchmark vs software, Ed25519 sign/verify timing (feeds the Notary demo).
- [ ] TRNG health screen (use the Xous TRNG health tests) and a visual randomness display.
- [ ] Look for other chip features worth a demo: glitch sensors (read-only), RTC, temperature sensor (if exposed), USB speed, ADC pins (PWM/ADC columns in the pinout).

## Round 4: hackathon deliverables (see STRATEGY.md)
- [ ] Notary: BIO event capture, Ed25519 signing in an isolated process, PC `verify.py`, MMU "game tries to read the key" demo.
- [ ] README with reproduction steps, protocol spec, 2-minute video, write-up with the limits spelled out, licence (suggest Apache-2.0). No new features after 12 Dec.

## Hardware notes
- [ ] Buzzer: BIO tone on PB3 (pin 32) works (user heard it); keep the 330 ohm in series, no filter capacitor across the buzzer (it would cut the tones).
- [ ] 10 uF (+ 2.2 uF) across 3V3 and GND near the OLED as decoupling (recommended, optional now that v0.8 is stable).
- [ ] Speaker later: needs an amplifier or transistor stage; never drive an 8 ohm speaker from a pin.

## Housekeeping
- [ ] Update README.md and HANDOFF.md for v0.8+ (menus, button map, safe-mode switches: hold BACK at boot = no display/sound, hold DOWN = no sound; flash flow).
- [ ] Re-run `tools/watch_diag.py` for 30+ minutes after every big change; keep `diag_log.txt` out of git.
- [ ] Possibly report upstream: unbounded spin in `i2c_write_async`/`i2c_read_async`; console burst bug; Dvorak keymap issue (not posted anywhere).

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
