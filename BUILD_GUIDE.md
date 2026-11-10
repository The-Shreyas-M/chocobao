## BUILD GUIDE v1 (board on the breadboard, headers soldered), 9 Nov 2026

### Take out now (only these)
- Dabao on the breadboard + USB cable, **multimeter**
- **6 tactile buttons**, **1 LED + one 330 ohm resistor**, single-core wires (red for 3.3 V, black for GND, other colours for signals)

### Leave in the bag for now (no firmware uses them yet, wiring them only adds risk)
OLED, PAM8403 + speakers, buzzer, both pots, piezos, zeners, caps, 1k/10k/1M/2.2k resistors, ESP32s.

### Before power: 3 meter checks (USB unplugged, meter on the BEEP/continuity setting)
1. **3V3 pin to GND pin**: must NOT beep (no short).
2. **VBUS pin (40) to GND pin**: must NOT beep.
3. Each soldered neighbouring pin pair you are not using: must NOT beep (you already checked shorts; re-check anything you touched).
Find GND pins by the board silkscreen / `repos/dabao/docs/pinout/index.html`, and confirm one by beeping it to the USB-C metal shell.

### Wiring: because each pin now has its own breadboard column, your "one pin gap" rule no longer applies
Run a black wire from a **GND pin** to the breadboard's GND rail. Then each button goes from **the pin's column** to **the GND rail**:

| Button | Pin | (board pin) |
|---|---|---|
| OK / types a password | **PB2** | 31 |
| UP | PC9 | 6 |
| DOWN | PC3 | 10 |
| LEFT | PC10 | 5 |
| RIGHT | PC2 | 11 |
| BACK | PB5 | 35 |

Wire **PB2 first, test it, then add the other five** (today only PB2 does anything; the others are read by firmware we write next).
LED: **PB4** column to the 330 ohm resistor to the LED's **long leg**; LED's short leg (flat side) to the GND rail. (Nothing lights yet: firmware for it comes next.)

**Never connect** a button or wire to: RUN (30), 3V3 (36), ON (37), VSYS (39), VBUS (40), PC13 (1, the PROG button), PB13/PB14 (15/16, console).

### After power-up: measure 3V3 pin to GND with the meter: expect about 3.3 V (3.2-3.4). Anything else: unplug and tell me.

### The display: wait.
It can do nothing until I write its driver, so wiring it now only adds risk. The real danger is **5 V reaching SDA/SCL** (the chip's pins are not 5 V tolerant; many OLED modules pull SDA/SCL up to whatever VCC is). When we do it: power it from the **3V3 pin only**, never VBUS; before connecting SDA/SCL to the board, power the OLED alone from 3V3 and measure SDA and SCL to GND: they must read 3.3 V or less. Then connect.

---

# Display wiring (0.96" I2C OLED, SSD1306), added 9 Nov 2026

The firmware drives it already (text only, 128x64). **If no display is attached, nothing changes and nothing breaks.**

## Connections (4 wires)
| OLED pin | Dabao pin | Board pin |
|---|---|---|
| VCC | **3V3** (never VBUS or VSYS) | 36 |
| GND | any GND | e.g. 25 or 28 |
| SCL | **PB11** | 27 |
| SDA | **PB12** | 26 |

Pins 26 and 27 sit between two GND pins (25 and 28), so GND is right beside the data pins. Pin 36 (3V3) has PB5 (your BACK button) on one side and pin 37 **ON** on the other: keep that wire clear of both.

## Safe procedure (the only real danger is 5 V reaching SDA/SCL)
1. **USB unplugged.**
2. Power the OLED **alone** from the 3V3 pin and GND (do NOT connect SCL/SDA yet). Plug USB in.
3. With the multimeter on DC volts, measure **SDA to GND** and **SCL to GND** on the OLED module. They must read **3.3 V or less** (many modules pull them up to their supply). If either reads above 3.5 V: unplug and tell me, do not connect them.
4. Unplug USB, connect SCL to PB11 and SDA to PB12, plug in.
5. About 4 seconds after the app starts, the screen should show: `ChocoBao`, `last key: --`, `OK tap : password`, `OK hold: 6 words`. Press a button and `last key` changes.

## If the screen stays blank
- Unplug, check the four wires and that SCL/SDA are not swapped.
- Some modules use address 0x3D or are SH1106 chips (the picture is shifted or garbled): tell me the text printed on the module.
- Ask me to run `i2cdetect scan`: it should show a device at `3c`.
