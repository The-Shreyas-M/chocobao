"""watch_diag: poll the board's `diag` console command every few seconds and log what happens.

  python tools/watch_diag.py            run until Ctrl+C (or --minutes N)

Use it while you use the device. It prints one line per poll and writes everything to diag_log.txt. When the
board restarts, the USB port disappears and comes back and `up=` starts from 0 again, so you get the exact
time of each restart plus the counters just before it:
  stuck  = how long the OLED transfer in progress has been running (ms)  -> a hung display bus
  ticks  = heartbeat of the app loop (should keep rising)                -> a frozen app thread
  panic  = panics caught (the app jumped back to HOME)
  fail   = OLED transfers that failed
"""
import argparse
import pathlib
import re
import time

import serial
from serial.tools import list_ports

APP_PID = 0x6197
LOG = pathlib.Path(__file__).resolve().parent.parent / "diag_log.txt"
PAT = re.compile(r"DIAG up=(\d+) ticks=(\d+) loopmax=(\d+) push=(\d+) fail=(\d+) pushmax=(\d+) stuck=(\d+) panic=(\d+) wd=(\d+) scr=(\d+) btn=(\d+)")


def find_port():
    for p in list_ports.comports():
        if p.vid == 0x1D50 and p.pid == APP_PID:
            return p.device
    return None


def type_slow(s, text):
    for ch in text.encode() + b"\r":
        s.write(bytes([ch]))
        time.sleep(0.012)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--minutes", type=float, default=0)
    ap.add_argument("--every", type=float, default=3.0)
    a = ap.parse_args()
    end = time.time() + a.minutes * 60 if a.minutes else None
    last = None
    s = None
    out = open(LOG, "a", encoding="utf-8")

    def log(msg):
        line = time.strftime("%H:%M:%S ") + msg
        print(line, flush=True)
        out.write(line + "\n")
        out.flush()

    log("--- watch started")
    while end is None or time.time() < end:
        port = find_port()
        if not port:
            if s:
                log("PORT GONE (board restarted or unplugged); last=" + str(last))
                try:
                    s.close()
                except Exception:
                    pass
                s = None
            time.sleep(0.5)
            continue
        if s is None:
            try:
                s = serial.Serial(port, 1_000_000, timeout=0.1)
                time.sleep(0.4)
                s.reset_input_buffer()
                log(f"connected {port}")
            except serial.SerialException:
                s = None
                time.sleep(0.5)
                continue
        try:
            s.reset_input_buffer()
            type_slow(s, "diag")
            buf = b""
            t0 = time.time()
            m = None
            while time.time() - t0 < 2.0 and not m:
                buf += s.read(max(1, s.in_waiting))
                ms = PAT.findall(buf.decode("utf-8", "replace"))
                m = ms[-1] if ms else None
            if m:
                v = list(map(int, m))
                last = v
                log("up=%ds ticks=%d loopmax=%dms push=%d fail=%d pushmax=%dms stuck=%dms panic=%d wd=%d scr=%d btn=%d"
                    % (v[0] // 1000, *v[1:]))
            else:
                log("no answer to diag (console busy or app frozen?)")
        except (serial.SerialException, OSError) as e:
            log(f"serial error: {e}")
            try:
                s.close()
            except Exception:
                pass
            s = None
        time.sleep(a.every)


if __name__ == "__main__":
    main()
