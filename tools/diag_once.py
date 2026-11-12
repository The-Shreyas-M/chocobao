"""Print the board's `diag` line once (includes the sensor-event counters: ev=edges/frames/bad last=value gap=BIO clocks)."""
import time

import serial
from serial.tools import list_ports

port = next((p.device for p in list_ports.comports() if p.vid == 0x1D50 and p.pid == 0x6197), None)
if not port:
    raise SystemExit("app port not found")
s = serial.Serial(port, 1_000_000, timeout=0.3, write_timeout=2)
s.reset_input_buffer()
for ch in b"diag\r":
    s.write(bytes([ch]))
    time.sleep(0.012)
time.sleep(0.5)
text = s.read(4000).decode(errors="replace")
for line in text.splitlines():
    if "DIAG" in line:
        print(line.strip())
