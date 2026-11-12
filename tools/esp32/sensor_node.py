"""ESP32 sensor node for ChocoBao (MicroPython).

Wiring (classic ESP32 DevKit): GPIO5 -> Dabao PC8 (EVENT), GND -> GND. Optional demo key: a push button between
GPIO23 and GND (the board's own BOOT button on GPIO0 works too).
Sends one 8-bit value as a burst of 10 pulses on GPIO5:
  edge 0 = start, edge 1 = a reference gap T later, then 8 data edges: the gap before each is T (bit 0) or 2T (bit 1).
Dabao's BIO core timestamps every rising edge; the reference gap lets it decode without knowing the clock speed.
Press the key (GPIO23) or BOOT (GPIO0) to send the next number; or call send(value) from the REPL.
Copy this file to the ESP32 as sensor_node.py and main.py containing only: import sensor_node; sensor_node.run()
"""
import time
import machine

T_US = 1000
EV = machine.Pin(5, machine.Pin.OUT, value=0)
BOOT = machine.Pin(0, machine.Pin.IN, machine.Pin.PULL_UP)
KEY = machine.Pin(23, machine.Pin.IN, machine.Pin.PULL_UP)


def _pulse():
    EV.value(1)
    time.sleep_us(100)
    EV.value(0)


def send(value):
    value &= 0xFF
    _pulse()
    time.sleep_us(T_US)
    _pulse()
    for i in range(7, -1, -1):
        time.sleep_us(2 * T_US if (value >> i) & 1 else T_US)
        _pulse()
    print("sent", value)


def run():
    n = 0
    while True:
        if BOOT.value() == 0 or KEY.value() == 0:
            send(n)
            n = (n + 1) & 0xFF
            while BOOT.value() == 0 or KEY.value() == 0:
                time.sleep_ms(20)
        time.sleep_ms(10)


if __name__ == "__main__":
    run()
