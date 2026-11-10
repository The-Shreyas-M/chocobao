"""flash_auto: hands-free flashing of the ChocoBao app. No buttons.

  python tools/flash_auto.py                 flash the freshly built apps.uf2
  python tools/flash_auto.py path/to/apps.uf2 [more.uf2 ...]

What it does:
  1. If the app is running, it types `bootloader` into the app's console. The app drives PROG (PC13) low
     and resets the chip, so boot1 stays in the bootloader (the same as holding PROG while tapping RESET).
  2. It flashes the UF2 over the boot1 serial console (about 2 s for the app).
  3. It types `reset` into boot1. Because bootwait is disabled, the chip restarts straight into the app.
Falls back with a clear message if a step fails. Needs the board's `bootloader` command (firmware built
after 9 Nov); the very first time you may still need one manual PROG + RESET to install that firmware.
"""
import argparse
import pathlib
import sys
import time

import serial
from serial.tools import list_ports

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import baoflash as bf  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT = ROOT / "repos/xous-core/target/riscv32imac-unknown-xous-elf/release/apps.uf2"
APP_PID, BOOT_PID = 0x6197, 0x6196


def port_for(pid):
    for p in list_ports.comports():
        if p.vid == 0x1D50 and p.pid == pid:
            return p.device
    return None


def wait_for(pid, seconds):
    end = time.time() + seconds
    while time.time() < end:
        dev = port_for(pid)
        if dev:
            return dev
        time.sleep(0.25)
    return None


def type_slow(s, text):
    for ch in text.encode() + b"\r":
        s.write(bytes([ch]))
        time.sleep(0.01)  # the app console corrupts bursts
    s.flush()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="*")
    a = ap.parse_args()
    files = a.files or [str(DEFAULT)]
    blocks = []
    for f in files:
        b = bf.parse_uf2(f)
        print(f"{pathlib.Path(f).name}: {len(b)} blocks OK")
        blocks += b

    boot = port_for(BOOT_PID)
    if not boot:
        app = port_for(APP_PID)
        if not app:
            sys.exit("no board found (neither app nor bootloader USB id). Plug it in.")
        print(f"app on {app}: asking it to enter the bootloader...")
        try:
            s = serial.Serial(app, 1_000_000, timeout=0.2)
        except serial.SerialException as e:
            sys.exit(f"can't open {app} (another program using it?): {e}")
        time.sleep(0.3)
        s.reset_input_buffer()
        type_slow(s, "bootloader")
        time.sleep(1.0)
        try:
            s.close()
        except Exception:
            pass
        boot = wait_for(BOOT_PID, 30)
        if not boot:
            sys.exit("the board did not enter the bootloader by itself. Fall back to: hold PROG, tap RESET, release PROG.")
    print(f"bootloader on {boot}")
    time.sleep(2.0)

    s = serial.Serial(boot, 1_000_000, timeout=0.02)
    bf.read_for(s, 0.3)
    bf.send_cmd(s, "localecho off", timeout=0.5)
    t0 = time.time()
    err = bf.flash_pipelined(s, blocks, 1, lambda n: None)
    if err:
        sys.exit(f"flash FAILED: {err}")
    kb = sum(sz for _, sz, _ in blocks) / 1024
    print(f"flashed {len(blocks)} blocks ({kb:.0f} KiB) in {time.time() - t0:.1f}s")

    print("resetting into the app...")
    type_slow(s, "reset")
    time.sleep(0.5)
    try:
        s.close()
    except Exception:
        pass
    app = wait_for(APP_PID, 60)
    if app:
        print(f"app is running on {app}. Done, no buttons pressed.")
    else:
        sys.exit("the app did not come back within 60 s. Tap RESET once; if it is still missing, tell me.")


if __name__ == "__main__":
    main()
