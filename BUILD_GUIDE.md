# ChocoBao build guide (updated 10 Nov 2026, firmware v2.1.0)

This guide uses the **names printed on the Dabao PCB** (PB2, PC10, GND, 3V3 ...), not pin numbers. Find the label on the board, solder or push the wire there.

Everything is 3.3 V logic. **Never connect a signal to VBUS, VSYS, ON or RUN, and never feed 5 V into any other pin.**

`---` is a wire, `[ ]` is a part, `GND` is any pin labelled GND. There are several; they are all the same wire inside.

## 1. Parts (everything here is already wired; nothing new to connect for v2.1.0)
| Part | Qty | Used for |
|---|---|---|
| Dabao on a breadboard, USB-C cable | 1 | the computer |
| Tactile buttons | 6 | OK, UP, DOWN, LEFT, RIGHT, BACK |
| LED + 330 ohm resistor | 1 + 1 | status light |
| 0.96" 128x64 I2C OLED (SSD1306, pins GND VCC SCL SDA) | 1 | screen |
| Buzzer | 1 | all sound |
| Multimeter | 1 | checks in section 3 |

## 2. Which label goes where (the whole wiring in one table)
| Part | Wire from the part | To the label on the Dabao |
|---|---|---|
| OK button | one leg | **PB2** |
| UP button | one leg | **PC10** |
| DOWN button | one leg | **PC2** |
| LEFT button | one leg | **PC3** |
| RIGHT button | one leg | **PC9** |
| BACK button | one leg | **PB5** |
| all six buttons | the other leg | **GND** |
| LED | long leg through a 330 ohm resistor | **PB4** |
| LED | short leg (flat side) | **GND** |
| Buzzer | + (through 330 ohm) | **PB3** |
| Buzzer | - | **GND** |
| OLED | VCC | **3V3** |
| OLED | GND | **GND** |
| OLED | SCL | **PB11** |
| OLED | SDA | **PB12** |

No resistors for the buttons: the chip has internal pull-ups. The board is mounted rotated, which is why UP/DOWN/LEFT/RIGHT are on those labels.

**Leave these alone:** RUN (that is RESET), ON, VSYS, VBUS, PC13 (that is PROG), PB13 and PB14 (the console), and 3V3 except for the OLED.

### Where the labels are
Two rows of pins. Top is the USB-C end. Left row, top to bottom: PC13, PC12, GND, PC11, PC10, PC9, PC8, GND, PC7, PC3, PC2, PC1, GND, PC0, PB14, PB13. Right row, top to bottom: VBUS, VSYS, GND, ON, 3V3, PB5, PB4, GND, PB3, PB2, RUN, PB1, GND, PB11, PB12, GND. If the silkscreen on your board runs the other way, trust the printed names, not this order.

```
        USB-C
   +----------------+
   | PC13     VBUS  |   PC13 = PROG button pin
   | PC12     VSYS  |
   | GND      GND   |
   | PC11     ON    |
   | PC10     3V3   |   <- UP (left),  3V3 (right) = OLED VCC
   | PC9      PB5   |   <- RIGHT,      BACK
   | PC8      PB4   |   <-             LED
   | GND      GND   |
   | PC7      PB3   |   <-             BUZZER +
   | PC3      PB2   |   <- LEFT,       OK
   | PC2      RUN   |   <- DOWN,       RUN = RESET (never wire)
   | PC1      PB1   |
   | GND      GND   |
   | PC0      PB11  |   <-             OLED SCL
   | PB14     PB12  |   <-             OLED SDA
   | PB13     GND   |
   +----------------+
```
Check this drawing against the names printed on your board before soldering. The order of the right row at the bottom may differ; the names are what count.

## 3. Before first power (USB unplugged, meter on the beep position)
| Check | Must be |
|---|---|
| 3V3 to GND | no beep |
| VBUS to GND | no beep |
| each pair of neighbouring soldered pins you use | no beep |
| one GND pin to the USB-C metal shell | beep |

## 4. Buttons (6x)
| Button | Leg 1 to | Leg 2 to |
|---|---|---|
| OK | PB2 | GND |
| UP | PC10 | GND |
| DOWN | PC2 | GND |
| LEFT | PC3 | GND |
| RIGHT | PC9 | GND |
| BACK | PB5 | GND |

```
 label (PB2, PC10 ...) ----+
                           |
                       [button]      (the same for all six)
                           |
 GND ----------------------+
```
Wire OK first, test it, then add the rest. Test in **Settings -> Button test**.

## 5. LED
| From | Through | To |
|---|---|---|
| PB4 | 330 ohm resistor | LED long leg |
| LED short leg (flat side) | | GND |

```
 PB4 ---[330]---|>|--- GND        (|>| = LED, flat side to GND)
```
It blinks 3 times at start, is on while a button is held, then blinks a code (1 OK, 2 UP, 3 DOWN, 4 LEFT, 5 RIGHT, 6 BACK).

## 6. Buzzer (the only sound output; v2.1.0 makes it sound fuller in software)
| Option | Buzzer + | Buzzer - | Result |
|---|---|---|---|
| Simple (what you have) | PB3 through 330 ohm | GND | works |
| Loud bridge (bare piezo disc only, optional) | PB3, no resistor | PB1 | about 4x the power |

```
 PB3 ---[330]---( buzzer + )
                ( buzzer - )--- GND
```
No capacitor across the buzzer. A coil-type buzzer under about 100 ohm must not be bridged (measure it first). If you do nothing, nothing changes: keep PB1 empty.

## 7. OLED (the only real danger is 5 V on SDA/SCL)
| OLED pin | Goes to |
|---|---|
| VCC | 3V3, never VBUS |
| GND | GND |
| SCL | PB11 |
| SDA | PB12 |

Safe procedure: connect only VCC and GND first, plug in USB, measure SDA to GND and SCL to GND on the module (both must read 3.3 V or less), unplug, then connect SCL and SDA. Keep those two wires short and away from the buzzer wire. A 10 uF capacitor across 3V3 and GND near the OLED is optional (long leg to 3V3).

## 8. First power-up
1. Measure 3V3 to GND: 3.2 to 3.4 V. Anything else: unplug.
2. The LED blinks 3 times, then the boot animation, then Home (a bao spinning in a microwave). BACK opens the menu: Tools, Games, Music, Lab, Settings.
3. Settings -> Button test shows ON while each button is held. Settings -> Buzzer test plays a scale.
4. After the idle time a screensaver starts (Settings -> Screensaver and Idle time); any button wakes it.

## 9. Music and sound
Music -> Blinding Lights and Attention are converted from MIDI files with `tools/midi2tune.py`. The buzzer is one voice; the player makes it fuller by flipping each note with its octave every 12 ms. To add a song:

```
python tools/midi2tune.py song.mid                     lists the tracks
python tools/midi2tune.py song.mid --track 0 --name MY_SONG --speed 1.0 > out.rs
```
Paste the result into `chocobao/src/music.rs` and add a line to `TUNES`. A speaker through an amplifier is **not** part of this build (the PAM8403 has a gain of about 16 and killed the first mini speaker); do not connect one without a resistor divider.

## 10. If something does not work
| Symptom | Check |
|---|---|
| Screen blank | OLED VCC on 3V3 (not VBUS), SCL (PB11) and SDA (PB12) not swapped, wires seated; some modules are address 0x3D or an SH1106 (shifted picture): tell me the text on the module |
| Screen glitches or the board restarts | shorten SCL/SDA wires, add the 10 uF capacitor, try another USB port; hold BACK while booting to start with display and sound off |
| A button does nothing | wire goes label to button to GND; look at Button test and the LED blink code |
| No sound | + on PB3 through 330 ohm, - on GND; Settings -> Sound on/off says ON; Settings -> Diagnostics last line says `BIO tone ok` |
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
