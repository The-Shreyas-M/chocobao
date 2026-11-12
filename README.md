# ChocoBao

A pocket multitool and arcade for the **Baochip-1x (Dabao board)**: Rust on the Xous microkernel, a 128x64 OLED, six buttons and a buzzer. No PC needed.

Made by Shreyas for the Dabao Open Hackathon. License: Apache-2.0.

## The idea in one minute

A sensor node (an ESP32) sends a reading to the Baochip over one wire. An I/O core timestamps it without the CPU, the Notary signs it with a key that other processes cannot read, and anyone can check the proof offline with a short Python script. Change one character and the check fails. Full story, protocol and limits: [docs/STORY.md](docs/STORY.md).

## What it does

| Area | Contents |
|---|---|
| Games (21) | Doom (5 levels), Jetpack Joyride, Road Rush, Subway Run, Ninja Run, Star Drifter (3D space), Space Invaders, Tetris, Pac-Man, Tower Blocks, Maze (3 levels), Lunar Lander, Breakout, Snake, Pong, Flappy Bird, Minesweeper, Solitaire, 2048, Magic 8-ball |
| Music | 4 songs from MIDI files played by a BIO core on the buzzer, metronome |
| Tools | Password maker (types into the PC), passphrase, PIN, dice, random number, stopwatch, timer, calculator, converters, morse trainer, text presets, BIP-39 seed words |
| Lab | Mandelbrot zoom, button scope, TRNG health test, **Notary** (Ed25519 signed events), **MMU demo** |
| Settings | Screensaver, idle time, sound, buzzer test, button test, CPU stats, diagnostics, About |
| Power | Sleep, restart, shut down |

## Uses the chip

| Chip feature | Used for |
|---|---|
| BIO cores (I/O coprocessors) | buzzer tone generator running on its own core, notes queued ahead so music never stutters |
| Hardware TRNG | passwords, seeds, dice, game randomness, Notary keys, TRNG health test |
| Ed25519 | Notary signs `CHOCONOTARY1\|n\|ms\|event`; `tools/verify.py` checks it on the PC |
| USB HID | types secrets into any PC, no driver; secrets are never shown on the OLED |
| UDMA I2C | OLED at 1 MHz, about 11 ms per full frame |
| ReRAM (via keystore app slots) | settings, high scores and seen help screens saved |
| Xous MMU, two processes | Lab -> MMU demo: a sandbox process reads the Notary's key page address, the kernel kills only the sandbox, the key still signs |
| BIO event capture | an I/O core timestamps pulses from an ESP32 sensor node; the Notary signs each reading |

## Quick start

| Step | Action |
|---|---|
| 1 | Build the hardware: [BUILD_GUIDE.md](BUILD_GUIDE.md) (buttons, OLED, buzzer, LED) |
| 2 | Put the board in bootloader mode: hold PROG, tap RESET, release PROG |
| 3 | Flash the three images (below) |
| 4 | Plug in and play |

## Build from source (Windows)

| Step | Command |
|---|---|
| Tools | Rust (GNU toolchain), the Xous toolchain, Python 3 with `pyserial` |
| Get Xous | `git clone https://github.com/betrusted-io/xous-core repos/xous-core` and check out `afa8985` |
| Patch Xous | `cd repos/xous-core && git apply ../../patches/xous-core-chocobao.patch` |
| Build | `powershell -File tools/build-chocobao.ps1` (set `CARGO_NET_OFFLINE=true` if the registry is slow) |
| Flash | `python tools/flash_auto.py <loader.uf2> <xous.uf2> <apps.uf2>` from `repos/xous-core/target/riscv32imac-unknown-xous-elf/release/` |
| Watch health | `python tools/watch_diag.py` |

The app source is in `chocobao/`; the build script copies it into the Xous workspace.

## Controls

| Button | Menus | Games |
|---|---|---|
| UP / DOWN | move | game specific |
| LEFT / RIGHT | change value | steer / move |
| OK | select | fire / act |
| BACK | back | pause, then quit |

First time you open any app a help screen explains its keys.

## Tools on the PC

| File | Purpose |
|---|---|
| `tools/verify.py` | verifies Notary proofs (pure Python Ed25519) |
| `tools/midi2tune.py` | turns a MIDI file into a tune for the buzzer |
| `tools/watch_diag.py` | live health counters, logs restarts |
| `tools/flash_auto.py` | flashes the three images over the serial bootloader |

## Notary in one minute

| Step | Action |
|---|---|
| 1 | Lab -> Notary, press UP/DOWN/LEFT/RIGHT: each press creates a signed event |
| 2 | Click a text box on the PC, hold OK: the device types `CHOCONOTARY1\|count\|ms\|event\|signature\|publickey` |
| 3 | Save lines to `proofs.txt`, run `python tools/verify.py proofs.txt` |
| Result | `VALID` for every line; change one character and it says `INVALID` |

The key is made by the TRNG and lives in RAM only. It proves that whoever held that key signed that text. It does **not** prove the clock is true or protect against someone with physical access to the board.

## Status and honest limits

| Item | State |
|---|---|
| Board mode | developer mode: secrets are not protected against someone holding the board |
| Notary isolation | the key sits on its own page of the main app; the sandbox process cannot read it. Code inside the main app can |
| MMU demo | two real processes in one image; tested on the board as described in docs |
| Saved settings | stored via the keystore service's app slots; boots and runs stable, persistence across a restart still being confirmed |
| Games and tools | tested on the board by hand, the newest ones less than the older ones |
| Flash | app image uses about 800 KB of 870 KB |

## Repository

| Path | Contents |
|---|---|
| `chocobao/` | the firmware (Rust) |
| `tools/` | build, flash, verify, MIDI converter |
| `patches/` | the small changes to xous-core |
| `docs/` | design notes, plan, todo |
| `bio/` | BIO program sources used while experimenting |
