# TODO

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
