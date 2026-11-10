# ChocoBao

A pocket multitool on the Baochip-1x (Dabao board). Today it has **Dice**: passwords, BIP-39 passphrases and dice rolls from the chip's hardware random-number generator, which it can also type into your PC. More apps are planned in [PLAN.md](PLAN.md): synth, signed-sensor bridge for ESP32, games.

Status: early. Everything below was tested on the board unless it says otherwise.

## Quick start: the button (no PC software)

1. Wire a push button between **PB2** and **GND** (see "Wiring" below). No resistor needed.
2. Plug the board into USB-C, then **press and hold PROG for about 1 second** (the board waits in its bootloader after power-up until you do).
3. Click into any text field and **press the button**. The board types a random password.
   - **Short press:** 20 random letters and digits (about 119 bits of entropy).
   - **Long press (hold about 1 second, then release):** 6 random BIP-39 words joined by `-` (66 bits).

It types wherever the keyboard focus is, so click the right field first. Nothing is stored, shown or logged. The board is a USB keyboard; no driver or program is needed on the PC.

### Wiring
```
PB2 (board pin 31) ----[push button]---- GND (board pin 33, or pin 28)
```
Pin numbers follow `repos/dabao/docs/pinout/index.html` (open it in a browser and look for the labels PB2 and GND). Never connect anything to a pin marked 5 V (VBUS) or use PC13. Careful: pin 30 (RUN) sits right next to PB2; touching it to GND resets the chip, which just drops it back into the bootloader (press PROG to continue).

## Optional: helper window and serial console (Windows)

`ChocoBao.cmd` opens a small window with Password, Passphrase and Dice buttons (it sets up a Python environment in this folder on first run, needs internet once). It is optional; the button works without it.

## If you want the raw console instead

The app talks over a normal USB serial port (no driver needed on Windows).

| Setting | Value |
|---|---|
| Port | the new `COM` port that appears when the app runs (Device Manager, Ports). It is **not** the same number as in bootloader mode. `python tools\baoport.py` prints it. |
| Baud | **1,000,000** (1 Mbaud) |
| Format | 8 data bits, no parity, 1 stop bit (8N1), no flow control |
| Enter key | CR (`\r`) |

PuTTY: Connection type Serial, Serial line `COMx`, Speed `1000000`. Press Enter and you should see `[console]`. Then type `help`.

**Important (found 8 Nov): the board's console corrupts text that arrives in a burst.** Pasting a command, or a tool that sends a whole line at once, can turn `dice pw 16 alnum` into garbage like `dcce pnnuu` (characters dropped or repeated). Typing by hand is usually fine. If you paste or script it, add a delay of about 10 ms per character (Tera Term: Setup, Serial port, "Transmit delay" 10 msec/char). That is the reason the helper window exists: it types slowly for you. This is a bug or limit in the stock Xous console, not in our code.

## Dice commands

```
dice                     help
dice roll [NdM]          e.g. dice roll 2d6   (N 1..100, M 2..1000)
dice pw [len] [set]      random password, len 4..128 (default 20)
                         sets: full (default), alnum, lower, digits, hex
dice words [n]           n random BIP-39 words joined by '-' (default 6)
add the word `type`      send the result as USB keystrokes instead of printing it:
                         dice pw 20 alnum type   (you get 3 seconds to click the target field)
```

Every result shows its entropy in bits. Facts about how it works:
- Random numbers come from the chip's hardware TRNG, drawn with rejection sampling (no modulo bias).
- Nothing is stored on the board (no ReRAM writes) or in the helper window.
- `type` and the button work on a PC set to **US QWERTY** (the usual layout). The board itself assumes a Dvorak PC, so the app compensates; if your PC uses another layout, characters may be wrong (see `chocobao/src/typing.rs`). Verified on hardware 8 Nov.
- The console and the helper window show the result, so it can end up in terminal scrollback or the clipboard history. Prefer `type` for real passwords.
- No floating point is used (the CPU has none); entropy is computed with an integer-only log2.

## Flashing and development

Everything is documented in [PROCEDURE.md](PROCEDURE.md). The short version:

```bash
# build (mirrors chocobao\ into xous-core and runs cargo xtask)
powershell -ExecutionPolicy Bypass -File tools\build-chocobao.ps1
# put the board in the bootloader: press RESET (the button farthest from USB-C), then
python tools\baoflash.py repos\xous-core\target\riscv32imac-unknown-xous-elf\release\apps.uf2
# start the app: press and hold PROG for about 1 second
```

Flashing the app takes about 2 seconds. Do not flash more than you need: ReRAM is rated for about 10,000 writes per cell (see the wear rules in [PLAN.md](PLAN.md)).

Setup done on this PC (for reference): Rust (GNU toolchain) plus the Xous sysroot matching the compiler, Python with `pyserial`, and `bio-loader`'s environment (`ChocoBao.cmd` borrows its Python). xous-core is checked out at tag **v0.10.2** in `repos/xous-core`.

## Safety rules (please keep)
- Never type `lockdown`, `self_destruct`, `paranoid`, `altboot` or `bootwait` (anything but `check`) in a boot1 console. They are irreversible. `tools\baoport.py` refuses them.
- 3.3 V logic only, never 5 V on a pin. Never use pin PC13 (it is the PROG button). The chip cannot make 5 V; use the board's VBUS pin for 5 V from USB.
- This board is in developer mode: factory secrets were wiped when we flashed our own firmware. Nothing here is "tamper-proof".

## Files
| File | What |
|---|---|
| `ChocoBao.cmd`, `tools/chocobao_gui.py` | one-click helper window |
| `chocobao/` | the app source (copy of the Xous dabao-console plus `dice`, `bootloader`) |
| `tools/build-chocobao.ps1`, `tools/baoflash.py`, `tools/baoport.py` | build, serial flasher, port finder |
| `bio/` | BIO toolchain (Zig 0.15.2) and test programs |
| `PLAN.md`, `PROCEDURE.md`, `LEARN.md`, `NOTES.md`, `ideas.md`, `todo.md`, `C_cleanup.md` | plan, how-to, background, log, ideas, tasks, disk cleanup |
