# ChocoBao build guide (updated 10 Nov 2026, firmware v1.5.0)

Everything is 3.3 V logic. The Dabao cannot make 5 V; 5 V exists only on the VBUS pin. **Never connect a signal to VBUS, VSYS, ON or RUN/RST_N, and never feed 5 V into any other pin.**

How to read this guide: every part has a table (what goes where) and, when it is more than two wires, a drawing. `---` is a wire, `[ ]` is a part, `GND` is any ground pin.

## 1. Parts
| Part | Qty | Used for | Have it? |
|---|---|---|---|
| Dabao on a breadboard (headers soldered), USB-C cable | 1 | the computer | yes |
| Tactile buttons | 6 | OK, UP, DOWN, LEFT, RIGHT, BACK | yes |
| LED + 330 ohm resistor | 1 + 1 | status light | yes |
| 0.96" 128x64 I2C OLED (SSD1306, pins GND VCC SCL SDA) | 1 | screen | yes |
| Buzzer (active or passive) | 1 | sound (current) | yes |
| PAM8403 amplifier module | 1 | only for the speaker upgrade (section 9) | yes |
| Speaker 4 ohm 10 W (big) | 1 | recommended for the upgrade | yes |
| Speaker 8 ohm mini | 1 | later, only with the extra-quiet resistor (section 9); the first one blew | check |
| Resistors 330 ohm, 1k, 10k, 22k | a few | see each section | yes (22k: buy or use 2x 10k + 2.2k) |
| Capacitors 10 uF, 470 uF electrolytic | 1 each | speaker upgrade | yes |
| Multimeter | 1 | checks in section 3 | yes |

## 2. Pin map (board header pin numbers, from `repos/dabao/docs/pinout/index.html`)
PROG (PC13) is pin 1, RESET (RST_N) is pin 30.

| Function | Chip pin | Board pin | Goes to |
|---|---|---|---|
| OK | PB2 | 31 | button, other leg to GND |
| UP | PC10 | 5 | button, other leg to GND |
| DOWN | PC2 | 11 | button, other leg to GND |
| LEFT | PC3 | 10 | button, other leg to GND |
| RIGHT | PC9 | 6 | button, other leg to GND |
| BACK | PB5 | 35 | button, other leg to GND |
| LED | PB4 | 34 | 330 ohm, then LED long leg |
| Sound out | PB3 | 32 | buzzer + (section 6) or amp input (section 9) |
| Sound out, 2nd phase | PB1 | 29 | only for the loud bridge buzzer, else leave empty |
| OLED SCL | PB11 | 27 | OLED SCL |
| OLED SDA | PB12 | 26 | OLED SDA |
| OLED VCC | 3V3 | 36 | OLED VCC |
| Ground | GND | 25, 28 and others | everything's ground |
| 5 V (only for the amp) | VBUS | 40 | amp 5V |

Buttons use the chip's internal pull-ups: no resistors, just pin to button to GND. The board is mounted rotated, which is why UP/DOWN/LEFT/RIGHT sit on those pins.

**Never connect anything to:** RST_N (30), 3V3 (36) except the OLED VCC, ON (37), VSYS (39), PC13 (1, PROG), PB13/PB14 (15/16, the console).

## 3. Before first power (USB unplugged, meter on the beep position)
| Check | Must be |
|---|---|
| 3V3 (36) to GND | no beep |
| VBUS (40) to GND | no beep |
| each pair of neighbouring soldered pins you use | no beep |
| one GND pin to the USB-C metal shell | beep |

## 4. Buttons (6x)
| Button | Leg 1 to | Leg 2 to |
|---|---|---|
| OK | pin 31 | GND |
| UP | pin 5 | GND |
| DOWN | pin 11 | GND |
| LEFT | pin 10 | GND |
| RIGHT | pin 6 | GND |
| BACK | pin 35 | GND |

```
 Dabao pin ----+
               |
             [button]      (the same for all six)
               |
 GND ----------+
```
Wire OK first, test it, then add the rest. Test in **Settings -> Button test**.

## 5. LED
| From | Through | To |
|---|---|---|
| pin 34 | 330 ohm resistor | LED long leg |
| LED short leg (flat side) | | GND |

```
 pin 34 ---[330]---|>|--- GND        (|>| = LED, flat side to GND)
```
It blinks 3 times at start, is on while a button is held, then blinks a code (1 OK, 2 UP, 3 DOWN, 4 LEFT, 5 RIGHT, 6 BACK).

## 6. Buzzer (what is built now)
| Option | Buzzer + | Buzzer - | Result |
|---|---|---|---|
| Simple (current) | pin 32 through 330 ohm | GND | works, quieter |
| Loud bridge (bare piezo disc only) | pin 32, no resistor | pin 29 | about 4x the power |

```
 pin 32 ---[330]---( buzzer + )
                   ( buzzer - )--- GND
```
No capacitor across the buzzer. A coil-type buzzer under about 100 ohm must not be bridged (measure it first).

## 7. OLED (the only real danger is 5 V on SDA/SCL)
| OLED pin | Goes to |
|---|---|
| VCC | pin 36 (3V3), never VBUS |
| GND | pin 25 or 28 |
| SCL | pin 27 |
| SDA | pin 26 |

Safe procedure: connect only VCC and GND first, plug in USB, measure SDA to GND and SCL to GND on the module (both must read 3.3 V or less), unplug, then connect SCL and SDA. Keep those two wires short and away from the sound wire. A 10 uF capacitor across 3V3 and GND near the OLED is optional (long leg to 3V3).

## 8. First power-up
1. Measure 3V3 to GND: 3.2 to 3.4 V. Anything else: unplug.
2. The LED blinks 3 times, then the boot animation: a chip draws itself, four cores light, a padlock snaps shut, the noise settles into "ChocoBao", then "secure. fun. open." and "by Shreyas". Then Home (a bao spinning in a microwave). BACK opens the menu: Tools, Games, Music, Lab, Settings.
3. Settings -> Button test shows ON while each button is held. Settings -> Buzzer test plays a scale.
4. After 45 s idle a screensaver starts (pick it in Settings -> Screensaver); any button wakes it.

## 9. Speaker upgrade through the PAM8403 (not wired yet; continue with the buzzer until you want this)
**Why the mini speaker blew:** the PAM8403 multiplies its input by about 16 (24 dB). A 3.3 V square wave straight into it asks for several watts, a 0.5 W mini speaker cooks in seconds. The fix is to shrink the signal first (a resistor divider) and keep the module's volume knob at minimum when you power up. **A pin can never drive a speaker by itself.**

**Easiest and safest: use the BIG 4 ohm 10 W speaker** (it survives mistakes). It can pull a lot of current from USB, so the divider below keeps the signal small (quiet room volume, well inside USB's limit).

| From | Through | To |
|---|---|---|
| pin 32 (PB3) | 10 kohm resistor | node A |
| node A | 330 ohm resistor | GND |
| node A | 10 uF capacitor (the `+` leg towards node A) | amp input L |
| amp input GND | | Dabao GND |
| amp 5V | | pin 40 (VBUS) |
| amp GND | | Dabao GND |
| 470 uF capacitor | across amp 5V and GND (long leg to 5V) | |
| big speaker, 2 wires | | amp left output (L+ and L-), either way round |

```
 Dabao                                    PAM8403 module
 -----                                    --------------
 pin 32 --[10k]--+--(+)10uF(-)---------->  L-IN
                 |
               [330]                       GND-IN <--+
                 |                                   |
 GND ------------+-----------------------------------+--- GND
 pin 40 (VBUS) --------------+-------------------->  5V
                         (+)470uF(-)
 GND ------------------------+-------------------->  GND
                                           L+ L- ---> speaker (any order)
```

Rules so nothing blows:
1. Volume knob at minimum when you plug in, then raise it slowly while a tune plays.
2. Only 4 ohm 10 W speaker first. For the 8 ohm mini speaker later, change the 10 kohm to **22 kohm** (4x quieter) and keep the knob below one quarter.
3. If it hisses or is too quiet, make the top resistor smaller **but never below 4.7 kohm**.
4. If the Dabao resets when the music is loud, the speaker takes too much USB current: turn the knob down.
5. PB1 (pin 29) stays unconnected in this wiring.
6. Real 4-voice sound is not built yet (one pin = one voice). Later: four BIO pins, each through 1 kohm into the same node A.

## 10. If something does not work
| Symptom | Check |
|---|---|
| Screen blank | OLED VCC on 3V3 (not VBUS), SCL/SDA not swapped, wires seated; some modules are address 0x3D or an SH1106 (shifted picture): tell me the text on the module |
| Screen glitches or the board restarts | shorten SCL/SDA wires, add the 10 uF capacitor, try another USB port; hold BACK while booting to start with display and sound off |
| A button does nothing | wire goes pin to button to GND; look at Button test and the LED blink code |
| No sound | + on pin 32 through 330 ohm, - on GND; Settings -> Sound on/off says ON; Settings -> Diagnostics last line says `BIO tone ok` |
| LED never lights | long leg towards the resistor, flat side to GND |

## 11. Flashing
1. Hold **PROG** (button nearest the USB-C port), tap **RESET** (farthest from USB-C), release PROG.
2. Run `.venv\Scripts\python.exe -u tools\flash_auto.py <loader.uf2> <xous.uf2> <apps.uf2>` with the three files from `repos\xous-core\target\riscv32imac-unknown-xous-elf\release\` (always all three; see HANDOFF.md).
3. It flashes in about 8 s and starts the app by itself. The serial error at the very end is harmless.

## 12. Safe-mode switches (hold while the board boots, release after the LED blinks)
| Hold | Result |
|---|---|
| BACK | no display, no sound (nothing touches I2C or the BIO) |
| DOWN | no sound only |
