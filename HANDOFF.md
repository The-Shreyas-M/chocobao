# HANDOFF: ChocoBao on the Baochip-1x Dabao (state at 10 Nov 2026, firmware v1.1)

**Read this first in a fresh session, then do "Next stages" at the bottom.** Details: todo.md (task list), STRATEGY.md (hackathon plan), BUILD_GUIDE.md (wiring), NOTES.md (running log), LEARN.md (chip notes), PLAN.md (older plan).

## 1. Goal
Win the Dabao Open Hackathon (deadline **15 Dec 2026, 21:00 ET**; judged on Works / New / Open / Uses-the-silicon) with **ChocoBao**: a standalone pocket multitool on the Baochip-1x Dabao (Rust/Xous): menu OS on a 128x64 OLED, six buttons, hardware TRNG, USB-keyboard typing, games, buzzer. Planned hero feature (STRATEGY.md): a **Notary**: BIO cores timestamp events, an isolated process signs them with Ed25519, a PC script verifies, and a game that tries to read the key is killed by the MMU. **No ESP32** (user has none). Fun apps are the shell around it; no new features after 12 Dec.

## 2. User preferences (important)
- **Never edit `BOM.md`** (shopkeeper list). Docs: BUILD_GUIDE.md, HANDOFF.md, todo.md are fine to edit.
- Git: local repo on the **personal** account `The-Shreyas-M` (`.gitcp.md`, git-ignored). **Do not push** until asked. Use the `gitcp` skill. Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Minimise ReRAM writes (about 40 flashes so far of ~10k). Never touch other one-way counters; never `self_destruct`.
- The user dislikes button presses and repeated asks: batch flashes. Flash flow is PROG+RESET (see 5). They declined a menu/bootloader command ("doesn't work"), asked for a command-driven way, and **a decision on enabling `bootwait` is still open** (see 5).
- User has no ESP32, no capacitor added (display works fine), a buzzer that is wired **to GND** (not the loud bridge), 6 buttons (controls are enough, add nothing), one OLED. Does not want extra hardware purchases.
- The user wants: neumorphic-style UI (soft raised/sunken panels), descriptive icons and instructions for every app, cool animations, more apps, use of the 4 BIO cores and other chip features, Mandelbrot zoom, Pac-Man (done), screensaver (done).
- Do not touch D: drive, restore points, LM Studio; C: cleanup was done earlier.

## 3. Hardware as wired (header pin numbers; see BUILD_GUIDE.md)
OK=PB2 (31), UP=PC10 (5), DOWN=PC2 (11), LEFT=PC3 (10), RIGHT=PC9 (6), BACK=PB5 (35); LED=PB4 (34) via 330 ohm; OLED SSD1306 I2C 0x3C: SCL=PB11 (27), SDA=PB12 (26), VCC=3V3 (36); **buzzer + = PB3 (32)** (through 330 ohm), **buzzer - = GND** (current wiring, works, quieter). Loud option: move buzzer - to **PB1 (pin 29)** and remove the resistor (piezo disc only; firmware already drives PB3/PB1 in opposite phase). PROG = PC13 = pin 1, RST_N = pin 30. Never use PC13, PB13/PB14, RST_N, 3V3 (except OLED), VSYS, VBUS, ON as signals. No capacitors fitted.

## 4. Repo layout
`chocobao/src/`: `main.rs` (threads), `controls.rs` (button poll, LED codes), `display.rs` (OLED thread, diff push, stats), `oled.rs` (Canvas drawing: text, rects, icon, arrow, card/inset neumorphic panels, wipe; Panel: I2C push), `logo.rs` (boot + home animation: chocolate pouring over a bao), `app.rs` (menu OS, all non-game screens, screensaver, help screens, transitions), `games.rs` (Game trait + Dodger, Snake, Pong, Flappy, Simon, Reaction), `doom.rs` (ChocoDoom raycaster), `pacman.rs` (Pac-Man), `rng.rs` (TRNG helpers, 17 password types), `sound.rs` (BIO-core tone generator + tunes), `typing.rs` (Dvorak-aware USB typing), `bootmode.rs` (restart helpers, USB re-announce), `diag.rs` (counters), `cmds/` (console commands incl. `diag`, `reboot`, `dice`).
`tools/`: `flash_auto.py` (serial flasher), `watch_diag.py` (polls `diag`, writes `diag_log.txt`), `build-chocobao.ps1`, `baoflash.py`, `baoport.py`, `bootwait.py`. `patches/xous-i2c-bounded-spin.patch` (HAL fix, applied to `repos/xous-core`, which is git-ignored).

## 5. Build and flash
- Build: `powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-chocobao.ps1` (mirrors `chocobao/` into `repos/xous-core/apps-dabao/chocobao`, runs `cargo xtask dabao chocobao`). Outputs in `repos/xous-core/target/riscv32imac-unknown-xous-elf/release/`: `loader.uf2`, `xous.uf2`, `apps.uf2`.
- **After every rebuild flash ALL THREE images** (app-only after a rebuild sent the board into a bootloader loop at ~3 s once): `R=repos/xous-core/target/riscv32imac-unknown-xous-elf/release; .venv/Scripts/python.exe -u tools/flash_auto.py $R/loader.uf2 $R/xous.uf2 $R/apps.uf2` (about 8 s).
- Getting into the bootloader: **hold PROG (nearest USB-C), tap RESET (farthest), release PROG**, then run the command above (it waits up to 3 minutes). The script flashes, then types `boot`; the app re-announces itself on USB so Windows lists COM14 again (worked twice).
- `bootwait` is **Disabled** (plug-in boots the app). Facts: it is a toggle stored in a one-way counter: every change burns one counter step (not a single irreversible flip, but the number of changes is finite; the chip keeps it for factory/dev use: with Enable the bootloader never auto-starts the app and waits for commands, which is how you recover a bad app; the alt-boot1 image forces it on to avoid boot loops). If enabled: console `reboot` + `flash_auto.py` would flash with no button presses, but plugging in/RESET then waits in the bootloader until `boot` or a PROG tap. Enable/disable with `tools/bootwait.py` (needs the bootloader). **Ask the user before changing it.**
- USB ids: bootloader 1d50:6196 (COM11), app 1d50:6197 (COM14). Console 1 Mbaud; type >= 10 ms/char; the serial error at the end of flash_auto.py is harmless.
- Safe mode (hold at boot): BACK = no display and no sound; DOWN = no sound only.
- Stability tool: `.venv/Scripts/python.exe tools/watch_diag.py --minutes 60` (kill old watchers with PowerShell `Get-CimInstance Win32_Process | ? CommandLine -like '*watch_diag*' | Stop-Process`, they hold the COM port).

## 6. Status
- **v1.4.0 (flashed, running)**: menus are now Tools, Games, Music, Lab, Settings (Settings holds Screensaver, Sound, Buzzer test, Button test, Diagnostics, About). New: microwave+bao+chocolate-bar home screen, 4 screensavers (`saver.rs`: bun, auto Pong, Eyes with moods, Escape artist), Clock (set by hand, RAM only), Music player with Blinding Lights and Attention (`music.rs`, written by ear, approximate), Piano. BUILD_GUIDE 4b has the 8 ohm speaker via PAM8403. Not seen on the real screen yet.
- **v1.3.0 (repo head, built, NOT flashed)**: easier Pac-Man (ghost cage with door, player starts bottom centre, no dead ends, pellets are blinking rings, 4 lives, slower ghosts), password/PIN/passphrase secrets are never drawn on the OLED (masked, hold OK types), new boot/home animation (drop, splash, rising liquid with XOR title). If the build says `failed to get rend`, set `CARGO_NET_OFFLINE=true`.
- **v1.2 (repo head) adds Seed words (Utilities, `seed.rs`) and Mandelbrot zoom (Lab, `mandel.rs`); built and committed, NOT flashed. Flash all three images with the next PROG+RESET.** Mandelbrot controls: OK pause, UP/DOWN zoom, LEFT/RIGHT target.
- **v1.0 flashed and running** (stable 90+ s watched; v0.8 ran 18+ min). **v1.1 (this repo head) is built and committed but NOT flashed**: it adds Pac-Man, neumorphic cards/panels, a passphrase screen layout fix.
- Never seen on the real screen by me: boot animation, home, icons, help screens, wipes, screensaver, neumorphic look, Doom, Pac-Man, Timer, Random number. Expect layout tweaks from the user's feedback.
- Freezes/restarts before v0.8 were most likely the unbounded spin in the HAL I2C driver (patched; flashed with loader+xous). Not proven.
- Buzzer works (BIO tone on PB3). Loudness: single-ended; bridge option above.
- TRNG is the on-chip ring-oscillator TRNG (kernel pass-through, already whitened per source comments). Fine for passwords/games; mix more entropy for real keys.
- Linux: not realistic (2 MiB RAM; no verified port); Xous is the supported OS. Real DOOM does not fit (WAD ~4 MB vs ~0.87 MB app region), hence ChocoDoom.

## 7. Gotchas learned
- **Version number lives in `chocobao/Cargo.toml` (`version`); the About screen reads it at build time. Bump it with every flash** (now 1.4.0).
- Bash tool: big heredocs with quotes sometimes fail to parse. Write files with the Write tool and apply edits with a small Python script (`tools/_edit_*.py`, then delete). Write requires a prior Read of an existing file.
- Windows: `pkill` does not kill Windows Python; use PowerShell. PowerShell `Start-Process` breaks on paths with spaces; use Bash with absolute paths.
- Do not toggle a pin at audio rate through IoxHal (floods the HAL); use the BIO tone generator. Keep OLED I2C on its own thread.
- boot1 leaves the bootloader loop on any PROG press after the first release (bounce can start the app).

## 8. Next stages (do in this order)
1. Get a PROG+RESET from the user, flash v1.1 (all three images), run `watch_diag.py`, ask the user to check every new screen; fix layout/animation feedback (neumorphic look, passphrase screen, icons, help text).
2. Ask about `bootwait` (hands-free flashing) if the user is still annoyed by PROG+RESET.
3. Round 2 (todo.md): ChocoDoom looks 3D (textures, floor/ceiling, sprites, minimap, tutorial).
4. Round 3: **Mandelbrot zoom** (fixed-point on the CPU; stretch: BIO cores), **valid BIP-39 seed words (12/24 with checksum via the hardware SHA)**, 4-voice BIO synth, BIO scope/self-sniffer of our own pins, IR learn/replay is NOT possible without a receiver (skip), TRNG health screen, crypto benchmarks.
5. Ideas from the pasted project list that need no extra hardware: seed/dice generator (mostly done, add checksum), canary/dead-man countdown (RAM only; never wipe the chip), TOTP vault (manual time entry, ReRAM save only on explicit action), self-sniffer demo.
6. Round 4: Notary (Ed25519, PC `verify.py`, isolation demo), README with reproduction steps, protocol spec, 2-minute video, licence (suggest Apache-2.0). Update docs after every change; commit locally; never push without being asked.
