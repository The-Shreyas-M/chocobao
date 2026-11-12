"""serial_log: write everything the running app prints on its serial console (log lines, panics) to a file, with
timestamps, and reconnect when the board restarts. Use it while reproducing a crash:  python tools/serial_log.py out.txt
Press Ctrl+C to stop. Only one program can hold the port: stop watch_diag.py first."""
import sys
import time

import serial
from serial.tools import list_ports

APP_PID = 0x6197
out = open(sys.argv[1] if len(sys.argv) > 1 else "console_log.txt", "a", encoding="utf-8")


def log(msg):
    out.write(time.strftime("%H:%M:%S ") + msg + "\n")
    out.flush()


def find():
    return next((p.device for p in list_ports.comports() if p.vid == 0x1D50 and p.pid == APP_PID), None)


s = None
log("--- logger started")
while True:
    try:
        if s is None:
            port = find()
            if not port:
                time.sleep(0.3)
                continue
            s = serial.Serial(port, 1_000_000, timeout=0.3)
            log("connected " + port)
        data = s.read(2000)
        if data:
            for line in data.decode(errors="replace").splitlines():
                if line.strip():
                    log(line)
    except Exception as e:  # port vanished: the board restarted
        log("port lost: %s" % e)
        s = None
        time.sleep(0.5)
