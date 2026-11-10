# Dabao working procedure

Sources: xous-core `README-baochip.md`, baochip.com/dabao-intro, bio-sim / bio-loader READMEs, plus what we observed on our own board. Items marked **(observed)** are things we saw, not documented; **(unverified)** are not yet tested.

## 0. Mental model

Two USB personalities, told apart by USB ID (VID 1D50):

| State | PID | What Windows shows | Meaning |
|---|---|---|---|
| Bootloader (boot1) | 6196 | `BAOCHIP` drive + serial (was COM11) | Ready to receive UF2 files; console has `audit`, `boot`, `reset`... |
| Running app (Xous) | 6197 | serial (was COM14) + HID keyboard + FIDO | Your firmware is running; console shows `[console]` |

COM numbers change between states. Never hardcode them: `python tools\baoport.py` finds the port.

Boot chain: boot0 (burned, locked) -> boot1 (bootloader, updatable, avoid) -> loader -> kernel (xous) -> apps. Images are three UF2 files: `loader.uf2`, `xous.uf2`, `apps.uf2`.

## 1. Hard rules (do not break)

1. **Never type** at any console: `lockdown` (revokes dev key, kills our ability to load own firmware), `self_destruct`, `paranoid`, `altboot`. `baoport.py send` refuses them.
1b. **`bootwait toggle|enable|disable` burns a one-way hardware counter** (boot1 source: `inc_coded` on a one-way counter). The counter is finite and irreversible, so never experiment with it. `bootwait check` is read-only and fine. `baoport.py` refuses anything but `check`. What bootwait does: when enabled, boot1 stays in bootloader at power-up until PROG is pressed (that is probably the behaviour we saw); when disabled, the app boots straight away unless PROG is held. **Our board: `bootwait is Enable`** (checked read-only), so it always waits in the bootloader after power-up; that is why the BAOCHIP drive appears without PROG and why PROG starts the app.
2. **Don't touch boot1.** No `ALTCHIP` workflow. `flash.ps1` aborts if an `ALTCHIP` drive appears.
3. 3.3 V logic only. Resistor on every LED. No shorts between neighbouring pins. Touch grounded metal before handling. Power off before wiring.
4. Reserved pins: `PC13` (USB enumeration), `PB13`/`PB14` (boot1 serial). Check `repos/dabao/docs/pinout/pins.csv` before using a pin; not every GPIO is routed.
5. Developer images permanently flag the chip as a dev device and wipe factory secrets. Already done on our board; fine.
6. Don't edit upstream repos in `repos/`. Our work goes in our own project folders.

## 2. One-time setup (done on this machine)

- Rust via rustup, **GNU host toolchain** (no MSVC): `rustup default stable-x86_64-pc-windows-gnu`; target `riscv32imac-unknown-none-elf`.
- Xous sysroot: `cargo xtask install-toolchain --force` (run in `repos/xous-core`). **Must match the rustc version exactly**; after any `rustup update`, rerun it. Releases: github.com/betrusted-io/rust/releases.
- Python: `pip install --user pipx ziglang pyreadline3`; bio-loader venv at `%LOCALAPPDATA%\pipx\pipx\venvs\bio-loader` (upstream's `bio-loader.exe` entry point is broken; use `bio-loader.cmd` in the project root, or `tools\baoport.py`).
- xous-core is pinned to tag **v0.10.1** (`repos/xous-core`). Board's boot1 is v0.10.0-61, which accepted it.
- Cargo needs internet (crates.io); a transient DNS error once failed a build, just retry.

## 3. Daily loop: firmware (Xous app)

```
# build + flash everything (first time / after kernel change)
.\tools\flash.ps1 -Build
# iterate on your app only (loader+kernel unchanged)
.\tools\flash.ps1 -Build -Mode apps
# talk to it
python tools\baoport.py send "help" "ver"
```
Manual equivalent:
1. `cargo xtask dabao dabao-console` in `repos/xous-core` (outputs in `target\riscv32imac-unknown-xous-elf\release\`).
2. Board in bootloader mode (drive `BAOCHIP` visible): copy `loader.uf2`, `xous.uf2`, `apps.uf2` (first time) or only `apps.uf2`.
3. **Press and hold PROG for a while until the drive disappears (observed)**; the board reboots into the app. The README says "press PROG".
4. Verify: `python tools\baoport.py` should say `RUNNING APP (Xous)`; console shows `[console]`.

How the board got into bootloader mode **(observed)**: it enumerated as `BAOCHIP` by itself when plugged in with no PROG, because there was no valid app yet. Whether it will do so again, or needs PROG held at plug-in once an app is installed, is **(unverified)**. Documented method: hold PROG while plugging in USB.

Our own apps: use the out-of-tree template `repos/dabao-console` (copy it to our own folder; only app code, pinned to a xous-core git rev). Build with
`cargo build --release --target riscv32imac-unknown-xous-elf --features board-dabao --features bao1x --features utralib/bao1x`,
convert: `cargo install xous-tools` then `xous-app-uf2 --elf target\riscv32imac-unknown-xous-elf\release\<app>`, producing `apps.uf2`. In-tree builds (inside xous-core) give signing and anti-rollback; out-of-tree is faster.

### 3b. Serial flashing (no drive, no PROG): proven 2026-11-07
```
# board must be in bootloader mode (replug USB; bootwait is enabled)
python tools\baoflash.py repos\xous-core\target\riscv32imac-unknown-xous-elf\release\apps.uf2 --boot
```
`baoflash.py` validates every block (UF2 magic, family 0xA7D76373, address 0x60060000..0x603DA000) before sending and refuses unless the board is in bootloader mode. apps.uf2 takes about 65 s. After `--boot`, wait up to about 60 s for the app to enumerate (check with `python tools\baoport.py`); in the meantime the old bootloader COM port stays listed but can't be opened. Returning from a running app to the bootloader still needs a USB replug until we add an app-side reboot command.

## 4. Daily loop: BIO (no flashing)

BIO programs (C or asm, < 0xF00 bytes per `bio-loader`) are uploaded **to the running app over serial**, no UF2 needed. Requires the running `dabao-console` firmware.
1. Write C in `repos/bio-sim/sw/<prog>/main.c` (copy `blink`, `nec_rx`, `i2s` as templates).
2. Build: `cd repos\bio-sim\sw; python -m ziglang build "-Dmodule=<prog>"` -> `.bin`.
3. Simulate first (Verilator, needs podman/docker, not installed yet): `./container-run configs/<prog>.jsonc`; view waves with `biowave.py` or BIO-surfer. Let the simulator catch timing mistakes before touching hardware.
4. Load: `bio-loader.cmd --port COMx --code prog.bin --quantum 10MHz --pads PC0,PC2`; `--clear` to wipe; `--console` for interactive serial.
5. BIO facts to design with: 4 cores, RV32E (16 regs), ~3 cycles/instr, 4 KiB code per core, 4x 8-deep FIFOs shared with host, registers x21-x26 GPIO, x20 halt-to-quantum, x27-x30 events, x31 core id + cycle counter. Quantum clock up to 700 MHz; external pin can act as the quantum clock (see `nec_rx`). Known errata are auto-patched by the build.

## 5. Bring-up checklist before soldering anything

- [x] Board enumerates, `audit` clean (boot0/boot1 OK, 0 attack attempts, dev key not revoked).
- [x] Own build flashed, console reachable.
- [ ] `test`, `usb`, `touch` console commands tried (touch uses PB1).
- [ ] Pick the project, decide which pins it needs from `pins.csv`, then solder headers.
- [ ] Simulate any BIO code in bio-sim before first hardware run.

## 6. If something goes wrong

| Symptom | Likely cause / action |
|---|---|
| No device at all | Cable is charge-only, or try another port. |
| `BAOCHIP` drive but no app after copy | PROG not held long enough; hold until drive disappears. Check all needed UF2 copied. |
| App doesn't enumerate (PID stays 6196) | Mismatched loader/kernel/apps revisions: recopy all three from the same build. |
| Console silent | Wrong COM (it changed); run `baoport.py`. Baud is 1,000,000 8N1. |
| xtask: "Toolchain for riscv32imac-unknown-xous-elf not found" | Rerun `cargo xtask install-toolchain --force` (version mismatch after rustup update). |
| cargo: cannot resolve index.crates.io | Transient network; rerun. |
| Hardware unresponsive | Re-enter bootloader (replug, hold PROG), recopy images. boot0/boot1 stay intact, so this is always recoverable by UF2. |

## 7. Files in this project

- `NOTES.md` running log of facts and observations
- `PROCEDURE.md` this file
- `ideas.md` hackathon ideas
- `tools/flash.ps1`, `tools/baoport.py` helpers; `bio-loader.cmd` BIO uploader wrapper
- `repos/` upstream clones (read-only for us): xous-core, dabao-console, bio-sim, bio-loader, dabao
