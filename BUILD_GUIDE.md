# ChocoBao build guide: every connection (updated 10 Nov 2026, matches firmware v0.8)

Everything below is 3.3 V logic. The Dabao cannot make 5 V; 5 V exists only on the VBUS pin. **Never connect a signal to VBUS, VSYS, ON or RUN/RST_N, and never feed 5 V into any other pin.**

## 1. Parts for the full build
| Part | Qty | Notes |
|---|---|---|
| Dabao on a breadboard (headers soldered) + USB-C cable | 1 | |
| Tactile buttons | 6 | OK, UP, DOWN, LEFT, RIGHT, BACK |
| LED + 330 ohm resistor | 1 + 1 | status LED |
| 0.96" 128x64 I2C OLED, SSD1306 (4 pins: GND VCC SCL SDA) | 1 | |
| Buzzer (active or passive both work) + 330 ohm resistor | 1 + 1 | |
| Electrolytic capacitor 10 uF (optional but recommended) | 1 | supply decoupling, see 5 |
| Single-core wires | ~25 | red = 3.3 V, black = GND, other colours = signals |
| Multimeter | 1 | for the checks in 3 and 6 |

## 2. Pin map (board header pin numbers)
Header pins are numbered as printed in `repos/dabao/docs/pinout/index.html`. PROG (PC13) is pin 1, RESET (RST_N) is pin 30.

| Function | Chip pin | Board pin | Goes to |
|---|---|---|---|
| OK | PB2 | 31 | button, other side to GND |
| UP | PC10 | 5 | button, other side to GND |
| DOWN | PC2 | 11 | button, other side to GND |
| LEFT | PC3 | 10 | button, other side to GND |
| RIGHT | PC9 | 6 | button, other side to GND |
| BACK | PB5 | 35 | button, other side to GND |
| LED | PB4 | 34 | 330 ohm resistor, then LED long leg (anode); LED short leg (flat side) to GND |
| Buzzer + | PB3 | 32 | 330 ohm resistor, then buzzer `+`; buzzer `-` to GND |
| OLED SCL | PB11 | 27 | OLED SCL |
| OLED SDA | PB12 | 26 | OLED SDA |
| OLED VCC | 3V3 | 36 | OLED VCC (never VBUS or VSYS) |
| OLED GND | GND | 25 or 28 (next to SDA/SCL) | OLED GND |

Buttons use the chip's internal pull-ups: no resistors are needed, just pin to button to GND. The board is mounted rotated, which is why UP/DOWN/LEFT/RIGHT map to the pins above (OK and BACK unchanged).

**Never connect anything to:** RUN / RST_N (30), 3V3 (36) except the OLED VCC wire and capacitor, ON (37), VSYS (39), VBUS (40), PC13 (1, the PROG button), PB13/PB14 (15/16, the console).

## 3. Before first power (USB unplugged, meter on continuity/beep)
1. **3V3 pin (36) to GND**: must NOT beep (no short).
2. **VBUS pin (40) to GND**: must NOT beep.
3. Each pair of neighbouring soldered pins you use (for example 31/32/34/35): must NOT beep.
4. Find GND pins from the pinout page and confirm one by beeping it to the USB-C metal shell.

## 4. Wiring order (do it in steps, check after each)
1. **GND rail:** black wire from a GND pin to the breadboard GND rail.
2. **Buttons:** one wire from each pin in the table to its button, the other button leg to the GND rail. Wire PB2 (OK) first and test, then add the other five.
3. **LED:** PB4 (34) to the 330 ohm resistor to the long leg; short leg to GND. After the app starts the LED blinks 3 times; it is on while any button is held and blinks a code after release (1 = OK, 2 = UP, 3 = DOWN, 4 = LEFT, 5 = RIGHT, 6 = BACK).
4. **Buzzer:** PB3 (32) to the 330 ohm resistor to buzzer `+` (the side marked +); buzzer `-` to GND. Nothing else; **no capacitor across the buzzer** (it would cut the high notes).
5. **OLED (safe procedure, the only real danger is 5 V on SDA/SCL):**
   1. USB unplugged. Connect OLED **VCC to 3V3 (36)** and **GND to GND** only. Do NOT connect SCL/SDA yet.
   2. Plug USB in. With the meter on DC volts measure **SDA to GND** and **SCL to GND** on the OLED module: both must read 3.3 V or less. If either is above 3.5 V, unplug and do not connect them.
   3. Unplug USB, connect **SCL to PB11 (27)** and **SDA to PB12 (26)**. Keep these two wires short and away from the buzzer wire.
6. **Decoupling capacitor (recommended):** 10 uF electrolytic across 3V3 and GND on the breadboard, close to the OLED. **Long leg to 3V3, stripe (short leg) to GND.** Add a 2.2 uF in parallel if you like. Wrong polarity can burst it, so check before powering.

## 5. First power-up
1. Plug in USB. Measure 3V3 pin to GND: expect 3.2 to 3.4 V. Anything else: unplug.
2. The LED blinks 3 times, then the OLED plays the boot animation (a chocolate bao bun), then the Home screen. BACK opens the menu.
3. Press every button in **Tools -> Button test**: the five direction/OK entries show `ON` while held.
4. **Tools -> Buzzer test** plays a C major scale up and down on the buzzer.

## 6. If something does not work
| Symptom | Check |
|---|---|
| Screen blank | VCC on 3V3 (not VBUS), SCL/SDA not swapped, wires seated. Some modules have address 0x3D or an SH1106 chip (shifted or garbled picture): tell me the text printed on the module. |
| Screen glitches or the board restarts | Add the 10 uF capacitor; shorten the SCL/SDA wires; check the USB cable/port. Test with **hold BACK while the board boots**: that starts it with the display and sound off (LED only). If it still restarts there, it is power/wiring, not the display code. |
| A button does nothing | Wire goes pin to button to GND (not 3V3); look at Button test and the LED blink code. |
| No sound | `+` on pin 32 through the 330 ohm, `-` on GND; Tools -> Sound on/off must say ON; Tools -> Diagnostics last line should read `BIO tone ok`. |
| LED never lights | Long leg towards the resistor/PB4, flat side to GND. |

## 7. Flashing (no menu item any more; planned to be removed)
1. Hold **PROG** (button nearest the USB-C port), tap **RESET** (farthest from USB-C), release PROG.
2. Run `.venv\Scripts\python.exe tools\flash_auto.py` (app only). After a kernel/HAL change add the loader and kernel images too (see HANDOFF.md).
3. The board restarts into the app by itself.

## 8. Safe-mode switches (hold while the board boots, release after the LED blinks)
- Hold **BACK**: no display, no sound (nothing touches I2C or the BIO).
- Hold **DOWN**: no sound only.
