# ChocoBao build guide (hardware)

You need: a Dabao board with headers on a breadboard, 6 push buttons, a 0.96" I2C OLED (SSD1306, 128x64), a buzzer, an LED and a 330 ohm resistor, jumper wires, a USB-C cable.
Everything is 3.3 V. Names below are the labels printed on the board (PB2, PC10, GND, 3V3 ...).
**Never connect anything to:** VBUS, VSYS, ON, RUN, PC13 (PROG), PB13, PB14.

## 1. All connections

| Part | Part pin | Dabao label |
|---|---|---|
| Button OK | one leg | PB2 |
| Button UP | one leg | PC10 |
| Button DOWN | one leg | PC2 |
| Button LEFT | one leg | PC3 |
| Button RIGHT | one leg | PC9 |
| Button BACK | one leg | PB5 |
| All 6 buttons | other leg | GND |
| OLED | VCC | 3V3 |
| OLED | GND | GND |
| OLED | SCL | PB11 |
| OLED | SDA | PB12 |
| Buzzer | + (through the 330 ohm resistor) | PB3 |
| Buzzer | - | GND |
| LED | long leg (through a 330 ohm resistor) | PB4 |
| LED | short leg (flat side) | GND |

No resistors for the buttons (the chip has pull-ups). Optional, louder buzzer: remove the resistor and wire buzzer - to PB1 instead of GND (bare piezo disc only).

## 2. Where the labels are (top = USB-C end)

| Left row | | Right row | |
|---|---|---|---|
| PC13 | do not use | VBUS | do not use |
| PC12 | | VSYS | do not use |
| GND | | GND | |
| PC11 | | ON | do not use |
| **PC10** | UP | **3V3** | OLED VCC |
| **PC9** | RIGHT | **PB5** | BACK |
| PC8 | | **PB4** | LED |
| GND | | GND | |
| PC7 | | **PB3** | BUZZER + |
| **PC3** | LEFT | **PB2** | OK |
| **PC2** | DOWN | RUN | do not use |
| PC1 | | PB1 | (optional buzzer -) |
| GND | | GND | |
| PC0 | | **PB11** | OLED SCL |
| PB14 | do not use | **PB12** | OLED SDA |
| PB13 | do not use | GND | |

If the order printed on your board differs, trust the printed names.

## 3. Check before power (USB unplugged, meter on beep)

| Check | Must be |
|---|---|
| 3V3 to GND | no beep |
| VBUS to GND | no beep |
| Any two neighbouring wired pins | no beep |

## 4. First power-up

| Step | What you see |
|---|---|
| Plug in USB-C | LED blinks 3 times, boot animation, then the home screen |
| Settings -> Button test | each button shows ON while held |
| Settings -> Buzzer test | a scale plays |
| Nothing on screen | check OLED VCC on 3V3 (not VBUS) and SCL/SDA not swapped |
| No sound | buzzer + on PB3 through the resistor, - on GND, Settings -> Sound is ON |

## 5. Flashing a build

| Step | Action |
|---|---|
| 1 | Hold **PROG** (button nearest USB-C), tap **RESET** (farthest from USB-C), release PROG |
| 2 | `python tools/flash_auto.py loader.uf2 xous.uf2 apps.uf2` (always all three) |
| 3 | It flashes in about 10 s and starts the app |

## 6. Safe-mode (hold while the board starts)

| Hold | Result |
|---|---|
| BACK | no display, no sound |
| DOWN | no sound |

## 7. ESP32 sensor node (optional, for the Notary demo)

A classic ESP32 DevKit sends a sensor value to the Dabao over one wire. A Dabao I/O core timestamps the pulses, the Notary signs the event, the PC verifies it.

| ESP32 | Dabao label | Purpose |
|---|---|---|
| GPIO5 | PC8 | event line (the only data wire) |
| GND | GND | common ground, required |
| GPIO23 to GND | not on the Dabao | demo key: a push button between GPIO23 and GND |

| Step | Action |
|---|---|
| 1 | Power the ESP32 from its own USB cable. Do not connect its 3V3 or 5V pins to the Dabao |
| 2 | Flash MicroPython once: `python -m esptool --port COMx erase_flash`, then `write_flash -z 0x1000 ESP32_GENERIC-*.bin` |
| 3 | Copy `tools/esp32/sensor_node.py` and `tools/esp32/main.py` to the ESP32 (`mpremote cp`) |
| 4 | On the Dabao open Lab -> Notary, then press the ESP32 key (or its BOOT button): the screen shows `ESP32:v=<n>:bio=<ticks>` and a signature |
| 5 | Hold OK to type the proof into the PC, run `python tools/verify.py proofs.txt` |

If you wired GPIO21/22 to PB12/PB11 earlier, those two wires are not used and can be removed. Check the link with `python tools/diag_once.py`: `ev=edges/frames/bad` should count up when the ESP32 sends.
