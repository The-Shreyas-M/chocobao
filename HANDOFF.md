# HANDOFF: ChocoBao on the Baochip-1x Dabao (state at 10 Nov 2026)

Read this first in a fresh session. Details live in PLAN.md, STRATEGY.md, NOTES.md, LEARN.md, BUILD_GUIDE.md, PROCEDURE.md.

## Goal
Win the Dabao Open Hackathon (deadline **15 Dec 2026, 21:00 ET**; judged on Works / New / Open / Uses-the-silicon) with **ChocoBao**: a standalone pocket multitool (menu OS on an OLED, six buttons, TRNG, USB-keyboard typing, games, buzzer). Planned hero feature: **Notary** (BIO-timestamped events, Ed25519-signed in an isolated Xous process, verified by a PC script; a game that tries to read the key gets killed by the MMU). The user does **not** want an ESP32 in the project; the sensor can be any pin (button, piezo, IR). See STRATEGY.md.

## User preferences (important)
- **Never edit `BOM.md`** (the user's shopkeeper shopping list). Put new content in other files.
- Git: repo is initialised on the **personal** account `The-Shreyas-M` (`.gitcp.md`, local-only). **Do not push** until asked. Use the `gitcp` skill for commit/push. Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Minimise ReRAM writes (~10k cycles/cell; ~20 flashes so far). Never touch one-way counters other than the already-set `bootwait` (currently Disabled); never `self_destruct`.
- Do not touch D: drive, restore points, LM Studio, or anything the user uses regularly (C: cleanup was done earlier).
- The user is a hardware-skilled maker, crypto novice, uses Dvorak on the board's keyboard service (typing.rs pre-compensates).
- The user dislikes being asked to press buttons repeatedly; keep flashes few and batched.
- The user does not own an ESP32 (said so on 10 Nov), has a buzzer (type unknown), 8 ohm 0.25 W speaker (do NOT drive directly from a pin; PAM8403 destroyed one earlier), 4 ohm 10 W speaker (too big).

## Hardware as wired now (Dabao pin numbers = header pins)
| Function | Pin |
|---|---|
| OK | PB2 (31) |
| UP | PC10 (5) |
| DOWN | PC2 (11) |
| LEFT | PC3 (10) |
| RIGHT | PC9 (6) |
| BACK | PB5 (35) |
| LED (330 ohm) | PB4 |
| OLED SSD1306 128x64, I2C 0x3C | SCL PB11 (27), SDA PB12 (26), 3V3 pin 36 |
| Buzzer (+) | **PC0 = header pin 14**, other leg GND (firmware; earlier I wrongly said PC11 = pin 14, PC11 is pin 4) |
Board facts: PROG = PC13 = header pin 1 (also USB SE0 control); RST_N = pin 30. Never use PC13, PB13/PB14 (console), 3V3/VSYS/VBUS pins as signals. Buttons are between pin and GND (internal pull-ups).

## Build and flash
- Source of truth: `chocobao/` here. Build: `powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-chocobao.ps1` (mirrors into `repos/xous-core/apps-dabao/chocobao`, runs `cargo xtask dabao chocobao`; GNU toolchain + Xous sysroot already installed). Output: `repos/xous-core/target/riscv32imac-unknown-xous-elf/release/apps.uf2`.
- Flash: `.venv/Scripts/python.exe tools/flash_auto.py` (serial UF2 flasher, ~2 s). New flow (built 10 Nov, **not yet run on hardware**): the script types `bootloader` into the app console; the app counts down 5 s then resets the whole chip; **the user holds PROG during the countdown** and releases when the script says so; script flashes and types `reset`. If the app is too old to have that command, the user does PROG + RESET once.
- USB IDs: bootloader PID 0x6196 (BAOCHIP drive, serial console 1 Mbaud), app PID 0x6197 (COM14 here). `bootwait` is **Disabled**: plug-in or RESET boots the app; holding PROG during reset gives the bootloader.
- Console quirks: the app console corrupts bursts, type >= 10 ms/char; echo is garbled; the app logs nothing useful on serial. Python run via `.venv` in the project folder (Windows app sandbox redirects AppData).
- Why software PROG did not work: boot1 re-configures PC13 as input+pull-up at start (`setup_dabao_boot_pin`), so driving it from the app before a reset does not hold. CPU-only reset and whole-SoC reset both landed back in the app. Not caused by `warm_boot` (that flag is OS-managed and not set by our app).

## Firmware v0.4/0.5 (what exists in `chocobao/src`)
`main.rs` wires threads; `controls.rs` (button poll 5 ms, bank reads, instant press, LED codes); `display.rs` (OLED thread, diff push, 3 s re-init, stuck-bus watchdog); `oled.rs` (Canvas drawing + Panel I2C); `logo.rs` (boot animation: chocolate bao); `app.rs` (menu OS, fixed 10 ms tick, catch_unwind recovery); `games.rs` (Dodger, Snake, Pong, Flappy Bao, Simon, Reaction via a `Game` trait); `sound.rs` (buzzer on PC0; toggles ~500/250 Hz or steady DC so both active/passive buzzers should sound; **buzzer type still unknown**); `rng.rs` (TRNG helpers, password **profiles** for site rules, BIP-39 passphrase); `typing.rs` (Dvorak-aware USB typing); `bootmode.rs` (bootloader entry + watchdog restart); `cmds/` (console commands: dice, bootloader, etc.).
Menu: Passwords (site-rules password, type 6 words, show 6 words, PIN), Dice, Games (6), Tools (Stopwatch, 8-ball, Entropy meter, Button test, Buzzer test, SOS, Sound on/off, Bootloader), About. Home: OK types a 20-char password, hold OK types 6 words, BACK opens menu.

## What is verified vs not (be honest in claims)
- Verified on board (earlier builds): serial flashing, TRNG dice/passwords, USB typing with Dvorak fix, six buttons + LED, OLED driver, v0.1 menus/games (with freezes), BIO test program via bio-loader.
- **NOT verified**: v0.4/v0.5 firmware has never been flashed (board still runs v0.3). Unknown: whether the freezes are fixed, buzzer type/sound, boot animation, new games, site-rules passwords, the new hold-PROG flash flow.
- Freeze/glitch cause is a hypothesis: the old v0.3 buzzer thread toggled a pin at audio rate (IPC flood) and the display I2C ran on the UI thread.
- TRNG: Dabao uses the on-chip ring-oscillator TRNG passed through the kernel (already conditioned/whitened per source comment); no independent proof of quality. Fine for passwords/games; mix extra entropy for real keys.

## Next steps (in order)
1. User does one manual PROG+RESET; flash v0.5; test everything on the list in the last chat message (freezes, buttons, boot logo, games, Tools > Buzzer test: report which sound is heard, site-rules password, Bootloader flow).
2. Fix whatever breaks; get 30 min of use without freezing.
3. BIO work via `libs/bio-lib` (std apps can load BIO code): pitched buzzer tone generator, then pin-event capture. Optional BIO showcase: IR remote learn/replay (bio/sw/nec_rx exists), WS2812, logic capture.
4. Notary: Ed25519 (`ed25519-dalek-bao1x`), key from TRNG, signed events, PC `verify.py`; isolation demo needs a second Xous process (untested; fallback = threads + honest wording).
5. README with reproduction steps, protocol spec, video, write-up. License TBD (suggest Apache-2.0). No new features after 12 Dec.

## Update 10 Nov (v0.8+)
- The in-app bootloader command and the Tools menu item were REMOVED. Flash with: hold PROG, tap RESET, release PROG, then `.venv/Scripts/python.exe tools/flash_auto.py` (app only) or with loader.uf2 xous.uf2 apps.uf2 after a kernel/HAL change.
- v0.8 includes a HAL patch (patches/xous-i2c-bounded-spin.patch) and is stable (18+ min). See todo.md for the next rounds.
