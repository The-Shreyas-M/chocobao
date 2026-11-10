"""bootwait: read or change the Baochip `bootwait` flag from the boot1 console.

  python tools/bootwait.py check                       read only
  python tools/bootwait.py disable --confirm           board auto-boots the app at power-up
  python tools/bootwait.py enable  --confirm           board waits in the bootloader at power-up (the factory state)
  add  --wait 280  to wait up to 280 s for the bootloader (press RESET on the board meanwhile)

What it is (xous-core libs/bao1x-api/src/offsets/common.rs): a two-state coded ONE-WAY counter at offset 80.
Each change consumes ONE increment of a counter line that is good for about 10,000 increments, so it is
reversible and cheap, but not free. This tool changes the state at most ONCE per run and re-reads it afterwards.
With bootwait Disable: plugging in starts the app; holding PROG while plugging in or tapping RESET enters
the bootloader. A missing or invalid app image still falls back to the bootloader.
"""
import argparse
import re
import sys
import time

import serial
from serial.tools import list_ports


def find_boot():
    for p in list_ports.comports():
        if p.vid == 0x1D50 and p.pid == 0x6196:
            return p.device
    return None


def talk(s, cmd, wait=1.5):
    s.reset_input_buffer()
    for ch in cmd.encode() + b"\r":  # type slowly, like the other tools
        s.write(bytes([ch]))
        time.sleep(0.01)
    end, out = time.time() + wait, b""
    while time.time() < end:
        out += s.read(max(1, s.in_waiting))
    return out.decode("utf-8", "replace")


def state_of(text):
    m = re.findall(r"bootwait is (Enable|Disable)", text)
    return m[-1] if m else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("action", choices=["check", "disable", "enable"])
    ap.add_argument("--confirm", action="store_true", help="required for disable/enable")
    ap.add_argument("--wait", type=int, default=0, help="seconds to wait for the bootloader")
    a = ap.parse_args()
    if a.action != "check" and not a.confirm:
        sys.exit("refusing: disable/enable need --confirm")

    port = None
    for i in range(max(1, a.wait)):
        port = find_boot()
        if port:
            break
        time.sleep(1)
    if not port:
        sys.exit("board is not in bootloader mode (press RESET, or hold PROG during reset if bootwait is already disabled)")
    time.sleep(2.0)
    s = serial.Serial(port, 1_000_000, timeout=0.05)
    talk(s, "", 0.5)

    help_txt = talk(s, "bootwait", 1.2)
    print("usage reported by the board:", re.findall(r"bootwait \[[^\]]*\]", help_txt) or help_txt[-120:])
    before = state_of(talk(s, "bootwait check", 1.2))
    print("bootwait before:", before)
    if before is None:
        sys.exit("could not read the current state; not changing anything")
    if a.action == "check":
        return
    want = "Disable" if a.action == "disable" else "Enable"
    if before == want:
        print(f"already {want}; nothing to do")
        return
    word = a.action if a.action in help_txt else ("toggle" if "toggle" in help_txt else None)
    if word is None:
        sys.exit("this bootloader offers neither the requested action nor toggle; not changing anything")
    print(f"sending: bootwait {word}   (one change, uses 1 increment)")
    talk(s, f"bootwait {word}", 1.5)
    after = state_of(talk(s, "bootwait check", 1.2))
    print("bootwait after:", after)
    if after != want:
        print("WARNING: state is not what was requested. Not retrying automatically.")
        sys.exit(2)
    print("done.")


if __name__ == "__main__":
    main()
