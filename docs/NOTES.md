# Dabao hackathon notes

## What the flyers + page say
- Board: Dabao (Baochip-1x / CrossBar "Daric"): VexRiscv RV32IMAC+MMU 350 MHz, 4x BIO PicoRV32 I/O coprocessors (700 MHz), 4 MiB RRAM, 2 MiB SRAM, USB HS, AES/ECDSA/EdDSA engines, TRNG, key store, glitch sensors. OS: Xous (Rust microkernel).
- Judging: **Works** (live demo on board) / **New** (chip enables what others can't) / **Open** (reproducible source) / **Uses the silicon** (BIO, MMU, crypto, IRIS - more is better). Deadline 15 Dec 21:00 ET. Submission = repo + video + write-up.
- Judges: Mark Davis, Jo Sung (CrossBar), Bunnie Huang (Baochip) + 3 surprise.

## Don't damage it (from xous-core README-baochip.md + dabao docs)
- boot0 is burned in and locked. Never try to defeat it. boot1 is updatable but only touch it if you must (needs alt-boot1 dance; wrong order = no USB bootloader).
- **Do NOT run `lockdown` at the boot1 console.** It revokes the developer key; afterwards you can't load your own dev images.
- Loading any developer-signed image erases Baochip secrets and bumps a one-way counter. Expected for hackathon use, just know it is permanent.
- GPIO is 3.3 V. No 5 V into pins, no LED without a resistor, no shorts to adjacent ground/power pins. Touch a grounded metal object first (ESD).
- Reserved pins: PC13 (dabao USB-clean-enumerate), PF5 (baosec), PB13/PB14 = boot1 serial console (1 Mbaud 8N1). Don't use them for your own signals.
- Bootloader: hold PROG while plugging in USB -> mass storage drive `BAOCHIP`. Copy loader.uf2, xous.uf2, apps.uf2 (all three the first time, same revision). Press PROG again to run. Later, apps.uf2 alone is enough.
- Not all I/O is routed to header pins. Check `repos/dabao/docs/pinout/pins.csv` before wiring.
- Prefer out-of-tree app builds (dabao-console style) for fast iteration; they skip anti-rollback/signing.

## Toolchain (Windows)
- Rust: rustup, **GNU host toolchain** (no MSVC installed, no Visual Studio needed), plus target `riscv32imac-unknown-none-elf`. Xous target `riscv32imac-unknown-xous-elf` comes via `cargo xtask` / xous-tools.
- Python: pipx, ziglang (BIO C programs build with `python -m ziglang build`), bio-loader, pyreadline3.
- Repos in `repos/`: xous-core (tag v0.10.1), dabao-console (out-of-tree template), bio-sim, bio-loader, dabao (KiCad + pinout).
- Build: `cargo xtask dabao dabao-console` (in xous-core). Out-of-tree: `cargo build --release --target riscv32imac-unknown-xous-elf --features board-dabao --features bao1x --features utralib/bao1x`, then `xous-app-uf2 --elf ...`.
- Not installed: podman/Verilator (only for bio-sim container; optional), VS Code Baochip extension (optional).

## Project direction (to decide together)
Strengths: hardware. Weakness: crypto. So lean on BIO + the chip's built-in crypto exposed through Xous services, not hand-rolled crypto.

## Status (2026-11-07)
- Installed and verified: rustup (GNU host), riscv32imac-unknown-none-elf target, pipx, ziglang, pyserial (in bio-loader venv).
- bio-loader: upstream pipx entry point is broken (package dir is named `bio-loader`, not importable). Use the wrapper `bio-loader.cmd` in this folder, e.g. `bio-loader.cmd --port COM3 --console` (also opens an interactive serial console).
- Nothing built or flashed yet. First step when the board is connected: plug in normally, find the COM port, open at 1 Mbaud, run `audit` (read-only).

## Board first contact (2026-11-07, read-only)
- Enumerates as USB VID 1D50:PID 6196: serial = COM11 (1 Mbaud), drive E: label BAOCHIP (FAT32, empty, ~115 MB) without pressing PROG -> boot1 is sitting in bootloader mode.
- boot1 console commands: altboot, audit, boot, boardtype, bootwait, echo, idmode, ifr, localecho, lockdown, paranoid, reset, self_destruct, skipping, uf2.
- NEVER type: `lockdown` (revokes dev key), `self_destruct`, `paranoid` (changes security policy), `altboot`/uf2 boot1 updates unless deliberate.
- `audit` result: Dabao, boot1 v0.10.0-61-g5397e1b48, stepping A0, boot0/boot1 keys all enabled (dev key NOT revoked, good), 0 attack attempts, CM7/debug fused off, boot1 receipts + anti-rollback OK. Next stage is a beta-signed image at 0x60060000.

## First build (2026-11-07)
- `cargo xtask dabao dabao-console` in repos/xous-core succeeded (needed the Xous sysroot: `cargo xtask install-toolchain --force`, from betrusted-io/rust release 1.99.1.1).
- Images: repos/xous-core/target/riscv32imac-unknown-xous-elf/release/{loader,xous,apps}.uf2 (177 KB / 2.3 MB / 542 KB). Developer-signed. NOT flashed yet.
- Flash plan: copy all three to the BAOCHIP drive (loader, xous, apps), board reboots, then open COM port @1 Mbaud and look for the `[console]` prompt.

## First flash SUCCESS (2026-11-07)
- Copied loader/xous/apps.uf2 to BAOCHIP drive, pressed PROG once -> board re-enumerated as VID 1D50:PID 6197 (composite: serial COM14, HID FIDO, HID keyboard). Bootloader mode is PID 6196 (COM11 + BAOCHIP drive).
- dabao-console `[console]` prompt on COM14 @1 Mbaud. Commands: echo, ver, trng, i2cdetect, test, usb, ws2812, touch.
- Re-enter bootloader: hold PROG while plugging in USB. COM port number changes between modes.

## Source study findings (2026-11-07)
- bootwait = Enable on our board (boot1 stays in bootloader at power-up until PROG). Toggling burns a one-way counter; do not touch.
- PROG button = PC13 (shared with USB SE0 switch). Console UART = PB13/PB14. I2C0 = PB11 (SCL) / PB12 (SDA).
- `usb type <text>` types over HID keyboard (works in dabao-console). ctap.rs is a stub only.
- ADC: HAL driver exists (PA04-07) but Dabao exposes no PA pins; pinout page says ADC drivers "under development". Use digital sensing + BIO instead.
- bio-lib has ws2812, captouch (PB1), av_trng, dali, qspi_to_8080. bio-sim examples: blink, i2s (input), nec_rx, test-ws2812.
- No PDDB on dabao. RRAM: app region offset 0x30_0000 len 0xDA000; data slots 0x603E_0000 (64 KiB, ACL'd). App persistence is untested.

## Serial flash + boot PROVEN (2026-11-07)
- tools/baoflash.py: sends UF2 blocks to boot1 console as `uf2 <base64>` (one 512-byte block per line, `localecho off` first). apps.uf2 = 1059 blocks, all acknowledged, 65 s (~16 blocks/s, ~8 KB/s of payload). Full loader+kernel+apps (5985 blocks) would be ~6 min, so use apps-only for iteration.
- `boot` over serial (no PROG, no drive copy) started the app: board re-enumerated as PID 6197 / COM14. It took roughly 30-50 s to appear (not timed precisely; a 20 s wait was too short). During the transition the old COM11 entry stays listed but cannot be opened ("device not functioning"); just wait.
- Console replies are sometimes cut/garbled by baoport.py's fixed read windows; the console itself responds (`[console]` prompt, echo).
- Still unproven: returning from the running app to the bootloader without replugging (needs an app-side `reboot`, see libs/bao1x-hal/src/clocks.rs `reboot()`), and the real boot-time.

## Flash speed benchmark (2026-11-07, apps.uf2 = 1059 blocks / 265 KiB payload)
- Old script: 65 s (61 ms/block). Cause: `s.read(4096)` with a 50 ms timeout waited the full timeout for every ~25-byte ack. Fixed (`read(max(1, in_waiting))`).
- New script, window=1: **1.0-1.2 s (230-255 KiB/s)**. Windows drive copy of the same file + volume flush: 0.5 s. So transfer speed is not a bottleneck either way.
- Pipelining (--window 8/16) stalls: boot1's main loop drains all queued bytes into ONE command line before processing, so queued `uf2` lines merge. Keep window=1.
- boot1 prints "Wrote N to 0x..." even if the ReRAM write returned an error (the error goes to a debug-only print), so the ack alone isn't proof of a good write.
- This boot1 build (v0.10.0-61) has NO `peek` command (compiled out), so byte-level readback is impossible; `baoflash.py --verify` reports that. Integrity evidence = the boot chain accepting the signed images.
- **Serial `boot` is unreliable on the host side.** Run 1: app appeared after a long delay (roughly 45-60 s, untimed). Run 2: still stuck in "limbo" after 3+ minutes: Windows keeps listing the bootloader device (COM11 + BAOCHIP drive) but COM11 cannot be opened ("device not functioning"), no PID 6197. `pnputil /scan-devices` needs admin. Hypothesis (unverified): boot1's `boot` command holds the USB SE0 line low for only 20 ms, whereas a PROG press holds it ~1 s, so Windows may not register a disconnect. A USB replug or PROG press recovers.
- PROG press while stuck in the `boot` limbo: app came up quickly, console answers (`[console]`, `ver`). So the serially-flashed image is good, and the limbo is the USB host/boot-handoff, consistent with (not proof of) the short-SE0 hypothesis. Current best workflow: replug -> `baoflash.py apps.uf2` (about 1 s) -> press PROG. Serial `boot` stays unreliable until understood.

## Overlap check against github.com/baochip, crossbar-inc and xous-core (2026-11-07)
**baochip org repos (API list):** baochip-1x, bio-sim, vex-sim, longbao (dev board breaking out ALL pins), dabao, bio-loader, cw308t-baochip (ChipWhisperer target board, i.e. fault-injection/side-channel testing is already prepared), bao-vscode-ext, ci-hat (hardware-in-the-loop CI HAT), plus forks. **crossbar-inc:** daric-hardware, daric-sdk (fork, ARM/ThreadX), daric-doc.
**Already exists (do not rebuild):**
- `xous-core/tools-bao/bao.py` (also bundled in the VS Code extension): subcommands `ports`, `monitor`, `flash` (copies UF2 to the mounted drive), `boot` (sends `boot` over serial), `boardtype`, `doctor` (basic Python/port check), `app create`, `artifacts`. Python 3.7+, pyserial.
- `bao-vscode-ext`: build, flash (drive auto-detect + verification), boot, serial monitor with Run/Bootloader port handling, one-command build-flash-boot-monitor, new-app templates, Rust/Xous toolchain and uv setup, kernel download from CI, size checks, de/ja/zh localisation.
**Not found upstream (gaps we could fill):** serial `uf2` flasher (they copy to a drive; same ~1 s), a pin-plan checker, an app-side reboot-to-bootloader command (upstream main `dabao-console` has none; the extension tells users to press RESET), BIO audio output, OLED/heart-rate drivers, any ReRAM storage layer for Dabao (no PDDB there), and any deniable store.
**Related work by others:** xous-core PR #937 / issue #875 (Supermagnum): OpenPGP smartcard (CCID) on Dabao; they are sizing a wear-aware counter log in ReRAM and asked about keystore domain separation. MicroPython port author is working on LittleFS in ReRAM. Issue #718 (bunnie): boot1 `boot` flakiness with some USB hubs/hosts; he suspects glitching on the PROG line into the USB switch after typing `boot`; macOS hides the serial port in that state. **This matches our `boot` limbo; it is a known boot1 issue, not our bug.**
**ReRAM endurance (bunnie, in #890 thread): standard quote ~10k read/write cycles, a bit higher in practice.** Lower than flash; invalidates my earlier "in-place rewrite is cheap" assumption. Also bunnie: Xous is "a single-tenant, fully trusted code base"; further per-process isolation needs a clear threat model (relevant to any "hostile app" demo).
**Conclusion:** the planned `bao` CLI duplicates official tools. Re-scope to gaps only.

## BIO console finding (2026-11-07)
- bio-loader needs a firmware console that understands `bio ready`, `bio <base64>`, `bio pin ...`, `bio clk <hz>`, `bio clear`. Our flashed v0.10.1 dabao-console has NO bio command. The tag v0.10.2 (upstream, newer) has `apps-dabao/dabao-console/src/cmds/bio.rs`. So to load BIO programs we must rebuild from v0.10.2 (our clone is at v0.10.1; the baokit-console copy we made lives in that tree and would need re-applying).
- Upstream xous-core tags: v0.10.0, v0.10.1, v0.10.2 (and rc/beta). Latest main commit 2026-11-03.

## Upgrade to xous-core v0.10.2 (2026-11-08)
- Reset our local edits (workspace line in Cargo.toml, xtask-patched link.x), removed the old baokit-console copy (source kept in src-ours/bootloader.rs), fetched tag v0.10.2, built `cargo xtask dabao dabao-console` (loader 473, xous 4595, apps 1139 blocks, all valid).
- Flashed all three over serial with tools/baoflash.py: 6207 blocks (1552 KiB) in 9.3 s (166 KiB/s). Not yet booted; next: press PROG, then check that the console has a `bio` command (v0.10.2 adds cmds/bio.rs).
- C: drive cleanup done earlier today: 8.0 GB -> about 49 GB free (caches, hibernation off, Windows cleanup, Ollama models). Pending user decision: delete C:\ac (3.9 GB) and emsdk (1.9 GB).

## First BIO program run on hardware (2026-11-08)
- Firmware: xous-core v0.10.2-0-gafa898595 (flashed over serial, started with a PROG press). Console `help` now lists `bio`.
- BIO toolchain (our own isolated copy, nothing global changed): `bio/venv` with **ziglang==0.15.2** (0.17.1 breaks build.zig: `b.pathFromRoot` was removed) and a `python3.exe` copy (build.zig calls `python3`; the Windows Store stub is useless). Sources copied from repos/bio-sim/sw to `bio/sw` so upstream stays clean.
- Build: `cd bio/sw; PATH=../venv/Scripts:$PATH python3 -m ziglang build -Dmodule=<prog> -Demit-listing=false` (the listing step needs a RISC-V objdump we don't have, so turn it off). Output `bio/sw/<prog>/<prog>.bin` (limit 0xF00 bytes) plus errata check.
- Load: `python repos/bio-loader/bio-loader/bio_loader.py --port COM14 --code <bin> --quantum 10MHz` (use the pipx venv python; `--clear` wipes it). The code is stored in keystore data slots, so it survives reboots until cleared.
- Console BIO helpers: `bio tx <n> [repeat]` writes FIFO0, `bio rx [count] [timeout_s]` reads FIFO0 (results appear as INFO log lines). Loader claims ONE core (Core0) and FIFO0; pins allowed (BIO numbers): 1,2,3,4,5,11,12,16,17,18,19,23,24,26,27,28,29. **29 = PC13 = PROG/USB-SE0: never use it.** 11/12 are I2C0.
- PROOF: `echo_once` (22 bytes): host `bio tx 10` -> core answers `bio rx` = 0x25 = 37 = 3*10+7. The looping variant `echo_math` fed its own answer back into the same FIFO; the garbage it returned matched iterations of 3x+7 exactly (start 100: iteration 48,489,026; start 0: 48,417,159), i.e. about 24M loop iterations/s (estimate from timing).
- FIFO0 is a shared queue: a core that pops and pushes the same FIFO reads its own output. Use a one-shot, or different FIFOs, or the host as the only consumer.
- For the 4-voice synth we cannot use the console loader (one core only); we will need our own Xous app using bio-lib (like ws2812) to claim all four cores.

## ChocoBao app + Dice (2026-11-08)
- New app crate `chocobao/` (copy of v0.10.2 dabao-console + `dice` and `bootloader` commands + BIP-39 wordlist). `tools/build-chocobao.ps1` mirrors it into xous-core/apps-dabao/chocobao, adds the workspace member, runs `cargo xtask dabao chocobao`.
- Built OK: apps.uf2 = 1331 blocks (about 340 KB image, region budget 872 KiB). loader/xous unchanged, flash only apps.uf2.
- `dice` commands: `dice roll [NdM]`, `dice pw [len] [full|alnum|lower|digits|hex]`, `dice words [n]` (BIP-39, 11 bits/word), add `type` to send as USB keystrokes (US layout assumed). TRNG via rejection sampling, entropy from an integer-only Q8 log2, nothing stored.
- In v0.10.2 the `trng` and `usb` console commands are behind the `extra-cmds` feature (not in the default `help`).
- `test adc` and `test temp` exist in the console (chip temperature needs no wiring: handy for Status).
- One-way counters (OWC) are ratchet flags in ReRAM that can only increase: anti-rollback per stage (loader/kernel/swap/app/baremetal), developer mode, OEM mode, paranoid mode, possible-attack counter, key/dev-key revocation, require-PQ-signature, bootwait, early-boot count. They are lifecycle state, not a timer and not a self-destruct. The real self-destruct is a separate boot1 command that requires the literal argument `void_my_warrantee` and wipes the chip permanently.
- Reset options: no 1200-baud trick, no reset command in the running console; RESET button (farthest from USB-C), unplug, or our `bootloader` command (untested).
- RESET button (farthest from USB-C) puts a running board into the bootloader without unplugging: CONFIRMED 2026-11-08 (bootwait enabled).

## Dice flashed and verified; console burst bug (2026-11-08)
- chocobao apps.uf2 flashed over serial in 2.1 s after RESET; PROG started it. Console `help`: echo, ver, i2cdetect, test, ws2812, touch, bio, dice, bootloader.
- `dice` verified on hardware: 30 commands over three runs, 0 errors; entropy figures correct (24 chars/75 symbols = 149.4 bits, 12 words = 132.0, 32 hex = 128.0). `dice ... type` NOT yet tested (it types into whatever window has focus).
- BUG FOUND: the stock app console corrupts characters that arrive in a burst (dropped/repeated characters, mangled replies; "dice pw 16 alnum" became "dcce pnnuu"). With 10 ms between characters every command is clean. This is why raw serial monitors/pasting misbehave. All our tools now type slowly (tools/chocobao_gui.py CHAR_DELAY, tools/baoport.py). Worth reporting upstream (not posted).
- New one-click helper: ChocoBao.cmd -> tools/chocobao_gui.py (tkinter + pyserial from the bio-loader venv). Finds the board by USB id, no baud/COM needed, answers parsed by format. `--cli "dice pw 16 alnum"` runs it headless.
- Port exclusivity: a serial monitor left open on COM14 makes everything else fail with "Access is denied".
- README.md written (quick start, serial settings: 1,000,000 baud 8N1, dice commands, flashing, safety).

## Standalone button typing (2026-11-08)
- User requirement: NO PC program; press a button, board types a password. Implemented in chocobao/src/button.rs (background thread started from main.rs): PB2 input with internal pull-up, 4-sample debounce, acts on release; short press = 20-char alnum password, long press (>=1 s) = 6 BIP-39 words joined by '-'. TRNG with rejection sampling, types via UsbHid::send_str, no logging of secrets, 300 ms lockout.
- Built OK (apps.uf2 ~674 KiB file, image still far under the 872 KiB region). Flash via the RESET + auto-flash watcher, then PROG. Button behaviour UNTESTED until the user wires the button (nobody has pressed it yet).
- Caveats: types into whatever has keyboard focus (no way to know whether a text field is open); letters/digits/'-' only so it is keyboard-layout-safe.
- Sandbox finding: files created under AppData by my (Claude app) processes are redirected into %LOCALAPPDATA%\Packages\Claude_*\LocalCache and are INVISIBLE to the user's own programs. That is why ChocoBao.cmd could not find the pipx python. Anything the user must run lives in the project folder: `.venv` (pyserial) now backs ChocoBao.cmd and bio-loader.cmd.

## Typing bug: board assumes a Dvorak PC (2026-11-08)
- User pressed the button and got a jumble (`nHG/NA4SsN6Rs<9<8p>...`): the board's keyboard service hard-codes `KeyMap::Dvorak` (services/bao1x-hal-service/src/servers/keyboard.rs:421, GetKeyMap), and both `send_str` and `send_keycode` convert through that map, so a US QWERTY PC receives substituted keys. (My earlier "assumes US layout" comment was wrong.)
- Fix in our app only (kernel untouched): chocobao/src/typing.rs `as_us_keyboard()` sends, for every wanted character, the character on the same physical key of a Dvorak keyboard. Verified offline against services/usb-bao1x/src/mappings.rs: all 94 printable ASCII characters produce identical key presses, 0 mismatches. Used by button.rs and `dice ... type`. If the service is ever fixed, delete the translation. Worth reporting upstream (not posted).
- BOM.md written: button rule (pin two away from a GND), 6 safe button pins (PC10, PC9, PC3, PC2, PB2, PB5), full pin plan, shopping list with ask/max prices (estimates; OLED anchors from ibots 178, Indian Hobby Center 209, IndiaMart 350).

## Button typing VERIFIED on hardware (2026-11-08)
- After flashing the typing.rs fix: 2 short + 2 long presses on a button wired PB2-GND typed `BPTkyLXgDuB4VtpgPg30`, `0EpXCAuwFU0BCUPTcPzb` (20 chars each, letters+digits only) and `glimpse-ivory-embody-embrace-card-cement`, `swing-divert-radar-bless-coin-cupboard` (6 valid BIP-39 words each). Standalone behaviour works: no PC software. (Those samples were pasted in chat: treat them as burned, never use them as real passwords.)
- Known limits: types wherever keyboard focus is; PC must be US QWERTY; no feedback LED yet.

## bootwait is a toggle, not an irreversible bomb (2026-11-09, corrects earlier wording)
- `BootWaitCoding` (libs/bao1x-api/src/offsets/common.rs) is a coded one-way counter at offset 80 with 2 states; the state is `counter value % 2` (Disable, Enable), so each flip = ONE increment of a counter line that is "only good for 10k increments" (acram.rs comment). Flipping to Disable costs 1 of ~10,000 and can be flipped back (another 1). Earlier notes calling bootwait "irreversible" were too strong. NOT changed on the board; awaiting user's explicit OK.
- boot1 main.rs: with bootwait Disable and PROG not held at power-up, `try_boot` runs the app immediately; holding PROG (or a bad/invalid image) leaves it in the bootloader. With bootwait Enable (current), the loop exits to boot only on a NOT-PRESSED -> PRESSED edge of PROG after startup. No capacitor can create that edge (PC13 also drives the USB switch), so no hardware trick; the software flip is the clean fix.
- BOM v2 written: short photo-friendly buy list (about 350-450 rupees core), separate already-have list, piezo parts for 10 ESP32 drum pads, pencil-iron advice, speaker overload warning (4 ohm 10 W on a USB-powered PAM8403).
- BOM v2.1 (2026-11-09): user has an adjustable-temperature pencil iron (removed from buy list); multimeter restored (cheapest with continuity buzzer + diode test, about 280-450 rupees est.); added 9 V battery and optional female header; change table vs v1 appended.

## bootwait flipped to Disable (2026-11-09)
- Done on the user's explicit request via tools/bootwait.py (read usage first: `bootwait [check | toggle | enable | disable]`; before: Enable; sent `bootwait disable` once; after: Disable, verified). Cost: one increment of the ~10k-increment counter line; reversible with `python tools/bootwait.py enable --confirm`.
- Expected behaviour now: plug in or tap RESET -> the app starts by itself. To enter the bootloader (for flashing): HOLD PROG while tapping RESET (or while plugging in) and release after about 1 s. The auto-flash watchers (baoflash) still work once the board is in the bootloader. The `bootloader` console command of chocobao will now just reboot into the app (it no longer parks in boot1).
- NOT yet verified: that a plain RESET now boots the app without PROG (board was left sitting in the bootloader after the change). README quick start still says to press PROG; update after verification.
- Process note: user asked me never to modify BOM.md (their shopkeeper list) unless asked. My unrequested "build guide" append was removed; it now lives in BUILD_GUIDE.md. Saved as a feedback memory.

## Six-button controller + LED codes (2026-11-09)
- chocobao/src/button.rs rewritten: OK=PB2 (types password/passphrase as before), UP=PC9, DOWN=PC3, LEFT=PC10, RIGHT=PC2, BACK=PB5 (no function yet, wiring test only), LED=PB4 output (4 mA). LED logic: 3 quick blinks at start; solid ON while any button is held; blink code on release (OK 1, UP 2, DOWN 3, LEFT 4, RIGHT 5, BACK 6); otherwise OFF. Log lines "<NAME> pressed / released after N ms" on the serial console.
- User reported the LED was ON before this firmware drives it: unexplained; PB4 may default high / be pulled up, or the LED is wired wrong. The new firmware sets PB4 low explicitly, so if it is still lit after start the wiring is the cause.
- Built OK (apps.uf2 674 KiB file). Flash needs bootloader: with bootwait Disabled, HOLD PROG while tapping RESET.

## Controller verified; display driver + hands-free flashing (2026-11-09)
- User confirmed on hardware: LED/button codes work, OK types a password. (Typed sample ysOw8bLNFb0eL6aXtjFX pasted in chat: burned.) Auto-boot after the bootwait flip works (app runs after plain RESET).
- New firmware (apps.uf2 722 KiB file, 1443 blocks): src/oled.rs (SSD1306 128x64 at I2C 0x3C, PB11 SCL / PB12 SDA, 6x12 font copied from boot1, 1 KiB framebuffer, never panics), src/display.rs (own thread + channel; if no display answers, the thread exits and events are dropped), button.rs sends Key/Typed events. Screen text: ChocoBao / last key / OK tap: password / OK hold: 6 words / typed N chars Nb.
- HANDS-FREE BOOTLOADER: found in the MicroPython port (ports/baochip/modmachine.c, mp_machine_bootloader): drive PC13 (PROG strap) LOW, hold ~100 ms, soft-reset; PC13 stays low across the reset so boot1 enters the bootloader. There is NO software "stay in bootloader" flag (BackupFlags only has warm_boot, which does the opposite). chocobao's `bootloader` command now does that (PC13 low 150 ms, then susres reboot). UNTESTED until the first run.
- tools/flash_auto.py: types `bootloader` into the app, waits for PID 6196, flashes, types `reset` into boot1, waits for PID 6197. Needs one manual PROG+RESET to install this firmware first.
- Display wiring + safe procedure appended to BUILD_GUIDE.md (NOT BOM.md): VCC 3V3 (pin 36), GND, SCL PB11 (27), SDA PB12 (26); measure SDA/SCL <= 3.3 V with the module powered alone before connecting.

- **Correction (10 Nov):** the button names in the 8 Nov entry above (UP=PC9, DOWN=PC3, LEFT=PC10, RIGHT=PC2) are the OLD, pre-rotation names. Current names (since 9 Nov, firmware controls.rs, BUILD_GUIDE.md): OK=PB2, UP=PC10, DOWN=PC2, LEFT=PC3, RIGHT=PC9, BACK=PB5.
