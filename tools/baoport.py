"""Find the Dabao's serial port and optionally send console commands.

Usage:
  baoport.py                      -> show which mode the board is in and its COM port
  baoport.py send "ver" "help"    -> send commands to the running firmware console, print replies
  baoport.py watch [seconds]      -> print whatever the board emits (default 10 s)

USB IDs seen on our board (VID 1D50):
  PID 6196 = boot1 bootloader (BAOCHIP drive + serial console, 1 Mbaud)
  PID 6197 = running Xous app (serial console, HID keyboard, FIDO)

Never send: lockdown, self_destruct, paranoid, altboot (see PROCEDURE.md).
"""
import sys
import time

import serial
from serial.tools import list_ports

BAUD = 1_000_000
MODES = {0x6196: "BOOTLOADER (boot1)", 0x6197: "RUNNING APP (Xous)"}
FORBIDDEN = {"lockdown", "self_destruct", "paranoid", "altboot"}


def find():
    for p in list_ports.comports():
        if p.vid == 0x1D50 and p.pid in MODES:
            return p.device, MODES[p.pid]
    return None, None


def read_for(s, seconds):
    end = time.time() + seconds
    out = b""
    while time.time() < end:
        out += s.read(4096)
    return out.decode("utf-8", "replace")


def main():
    port, mode = find()
    args = sys.argv[1:]
    if not port:
        print("No Dabao found (VID 1D50). Check USB cable / bootloader state.")
        return 1
    if not args:
        print(f"{port}  {mode}")
        return 0
    s = serial.Serial(port, BAUD, timeout=0.5)
    try:
        if args[0] == "watch":
            secs = float(args[1]) if len(args) > 1 else 10
            print(read_for(s, secs))
        elif args[0] == "send":
            for cmd in args[1:]:
                words = cmd.lower().split() or [""]
                if words[0] in FORBIDDEN or (words[0] == "bootwait" and words[1:] != ["check"]):
                    print(f"REFUSED: '{cmd}' is on the do-not-type list (bootwait only allows 'check')")
                    continue
                read_for(s, 0.3)
                for ch in cmd.encode() + b"\r":  # the app console corrupts bursts: type slowly
                    s.write(bytes([ch]))
                    time.sleep(0.01)
                print(read_for(s, 2.5))
        else:
            print(__doc__)
    finally:
        s.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
