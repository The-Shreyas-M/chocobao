"""flash_auto: flash the ChocoBao app with ONE button hold (PROG). No RESET tap, no re-plugging.

  python tools/flash_auto.py                 flash the freshly built apps.uf2
  python tools/flash_auto.py path/to/apps.uf2 [more.uf2 ...]

What happens:
  1. If the app is running, the script types `bootloader` into its console. The app starts a 5 s countdown and
     then resets the whole chip (same as tapping RESET).
  2. YOU hold PROG during that countdown (the script tells you when). boot1 sees PROG held and stays in the
     bootloader. (Pressing PROG drops the USB connection; that is expected.)
  3. As soon as the bootloader appears the script says "release PROG", flashes (about 2 s) and types `reset`;
     bootwait is disabled, so the chip restarts straight into the app.
If the board is already in the bootloader (you did PROG + RESET yourself) it skips straight to flashing.
The very first time (firmware older than 10 Nov) the app's `bootloader` command does not exist: do PROG + RESET once.
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
    ap.add_argument("--wait", action="store_true", help="do not send the command; wait up to 3 min for the bootloader (you start it from the device: Tools > Bootloader, then hold PROG)")
    a = ap.parse_args()
    files = a.files or [str(DEFAULT)]
    blocks = []
    for f in files:
        b = bf.parse_uf2(f)
        print(f"{pathlib.Path(f).name}: {len(b)} blocks OK")
        blocks += b

    boot = port_for(BOOT_PID)
    if not boot and a.wait:
        print('waiting for the bootloader: on the device choose Tools > Bootloader, hold PROG during the 5 s countdown, release ~3 s after the screen goes dark')
        boot = wait_for(BOOT_PID, 180)
        if not boot:
            sys.exit('no bootloader appeared within 3 minutes')
    if not boot:
        app = port_for(APP_PID)
        if not app:
            sys.exit("no board found (neither app nor bootloader USB id). Plug it in.")
        print(f"app on {app}: asking it to reset into the bootloader...")
        try:
            s = serial.Serial(app, 1_000_000, timeout=0.2)
        except serial.SerialException as e:
            sys.exit(f"can't open {app} (another program using it?): {e}")
        time.sleep(0.3)
        s.reset_input_buffer()
        type_slow(s, "bootloader")
        print("\n>>> HOLD the PROG button NOW and keep holding it (the chip resets in 5 s) <<<\n")
        time.sleep(0.5)
        try:
            s.close()
        except Exception:
            pass
        boot = wait_for(BOOT_PID, 40)
        if not boot:
            sys.exit("the bootloader did not appear. Was PROG held through the reset? Run again, or hold PROG and tap RESET.")
        print(">>> bootloader is up: you can RELEASE PROG now <<<")
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
