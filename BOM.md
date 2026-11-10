# SHOPPING LIST (v2, 9 Nov 2026)  -  show this to the shopkeeper

Prices in rupees are **my estimates** (memory plus a few online listings), not live quotes. **Ask** = offer this; **Max** = walk away above it. Parts marked "if available" are optional.

## BUY (one shop trip)

| Qty | Item (say exactly this) | Used for | Ask | Max |
|---|---|---|---|---|
| 1 | **0.96 inch OLED, I2C, SSD1306, 4-pin** (GND VCC SCL SDA) | display | 180 | 250 |
| 2 | **Male header strip 2.54 mm, 1x40** | solder onto the Dabao | 8 | 15 |
| 1 | **Solder wire 0.8 mm, 60/40 with rosin core**, 15-20 g | your solder is almost over | 40 | 70 |
| 14 | **Zener diode 3.3 V, 0.5 W** (BZX55C3V3 or 1N4728A) | one per piezo pad (10 drum pads + ChocoBao + spares) | 2 | 4 |
| 4 | **Diode 1N4148** | general spares | 1 | 2 |
| 14 | **Resistor 1 MΩ**, 1/4 W | piezo bleeder, one per pad | 1 | 2 |
| 14 | **Resistor 2.2 kΩ**, 1/4 W | piezo series, one per pad | 1 | 2 |
| 3 | **Resistor 330 Ω**, 1/4 W | LEDs (you have some, but buy 3 so you never have to borrow from your parallel pack) | 1 | 2 |
| 12 | **Resistor 1 kΩ**, 1/4 W | synth mixer, pot timing, spares | 1 | 2 |
| 6 | **Resistor 10 kΩ**, 1/4 W | spares / pull-ups | 1 | 2 |
| 2 | **Resistor 4.7 kΩ**, 1/4 W | OLED pull-ups only if the module has none | 1 | 2 |
| 8 | **Capacitor 100 nF (104) ceramic** | audio filters, pot timing | 1 | 3 |
| 4 | **Capacitor 10 µF, 16 V electrolytic** | audio coupling | 2 | 4 |
| 2 | **Capacitor 470 µF, 16 V electrolytic** (220 µF is fine too) | steadies the amp's 5 V, plus a spare | 4 | 8 |
| 1 | **Speaker 8 Ω, 0.5-1 W, small** (optional) | quiet tests; your 4 Ω 10 W one can overload USB | 40 | 70 |
| 1 | **CD74HC4067 16-channel analog mux module** (optional) | only if you want more than 8 drum pads on a classic ESP32 | 100 | 180 |
| 1 | **Multimeter, cheapest digital one WITH a continuity buzzer and diode test** (see guide below) | check voltages, find shorts, read resistors | 280 | 450 |
| 1 | **9 V battery** (only if the meter does not come with one) | powers the meter | 30 | 45 |
| 1 | **Female header strip 2.54 mm, 1x40** (optional) | a socket on a breadboard for the OLED/amp modules | 10 | 20 |

**Rough total:** core parts (OLED, headers, solder, diodes, resistors, capacitors, multimeter) about **₹650-900**; with the optional speaker, mux and female header about **₹800-1,150**. The OLED and the multimeter are most of the cost.

## Cheapest multimeter: what to look for
Your uses are: check 3.3 V and 5 V, test that two pins are **not** shorted after soldering, read resistor values, check a diode's direction. That needs only **DC volts, resistance, continuity buzzer, diode test**. You do not need auto-ranging, AC volts, "True RMS", or the 10 A range.
- Look for the buzzer symbol (a speaker or sound-wave icon) printed next to the diode symbol on the dial. The very cheapest ₹150-200 pocket meters (the plain "DT830B" type) often **lack the buzzer**; the "DT832" or "MAS830L" type usually has it for roughly ₹250-450. These model names and prices are from memory, so judge by the dial, not the name.
- Test it in the shop: touch the two probes together; it should beep and read near 0.
- Include batteries and probes. Keep the red probe in the VΩmA socket, never the 10 A one.
- Settings you will use: DC volts 20 V range for 3.3 V and 5 V; the beep/diode position for shorts; 20k or 200k range for resistors.

If the shop is out of 3.3 V zeners, ask for **1N5226B** (also 3.3 V). If they only have 1N5819 Schottky diodes, take two per pad instead (one to 3.3 V, one to GND).

---

## YOU ALREADY HAVE (do not buy)
- Soldering iron (pencil tip, adjustable temperature) and the soldering gun, flux (full), big and mini breadboards, single-core hook-up wire, USB cable. Header pins: set the pencil iron to about 320-340 °C and keep each joint under 3 seconds. Use the iron, not the gun, on the Dabao.
- 12 tactile 4-leg push buttons (6 needed now, 6 spare)
- 2 potentiometers, 3 legs, marked **20k** = 20 kΩ end to end (the middle leg slides between the ends). Fine for our timing method.
- Buzzer (check: touch it to 3.3 V; a steady beep = active, nothing or a click = passive)
- 330 Ω resistors (for the LED)
- LEDs, PAM8403 amp module
- 4 Ω 10 W speaker (works, but keep the volume low: see below)
- 8-10 piezo pads (ESP32 finger drum), at least one for ChocoBao
- ESP32 / ESP32-S3 boards, your Dabao

**Speaker warning:** the PAM8403 could pull 0.5 A or more into a 4 Ω speaker at full volume, and USB gives about 0.5 A in total for the whole board. Keep the volume low (small signal, or a resistor in series) or you may get resets. A small 8 Ω speaker draws far less, hence the optional line.

---

## Boot without pressing RESET and PROG: is there a capacitor trick?
**No capacitor does it, and nothing needs buying.** Here is why, from the bootloader's code (`boot1/src/main.rs`):
- Your board has `bootwait` **enabled**: it stays in the bootloader on every power-up. It leaves only when it sees PROG go from **not pressed to pressed after it has started**.
- A capacitor can't make that edge. Across PROG it would hold the pin low at power-up and then let it rise, which looks like "held, then released", so the bootloader sees no press. A real fix would be a timed pulse circuit (555 or transistor), and PROG's pin (PC13) also controls the USB switch, so a stuck or fighting pulse can break USB. I don't recommend it.
- **The real fix is a setting:** `bootwait disable`. Then plugging in starts the app straight away, and holding PROG while plugging in (or tapping RESET) enters the bootloader instead. It uses **one** of about 10,000 increments of that counter, and flipping it back uses one more, so it is reversible and cheap. (I previously called it irreversible. That was too strong.) A failed or invalid app image still drops back into the bootloader, so you can't lock yourself out.
- I have **not** changed it. Say the word and I will, and I will re-test the flash workflow afterwards.

---

## Controls and pin plan (reference)
Your rule: a button spans a GND pin and the pin **two positions away**. Safe spots (board pin numbers from `repos/dabao/docs/pinout/pins.csv`):

| Button | Pin | GND used |
|---|---|---|
| LEFT | PC10 (5) | GND 3 |
| UP | PC9 (6) | GND 8 |
| DOWN | PC3 (10) | GND 8 |
| RIGHT | PC2 (11) | GND 13 |
| OK / password | PB2 (31) | GND 33 |
| BACK | PB5 (35) | GND 33 |

**Never** use: PC13 (PROG), PB13/PB14 (console), RUN (30), or anything next to GND 38 (its neighbours are 3V3 and 5 V VBUS).

| Function | Pin(s) |
|---|---|
| OLED I2C | PB11 (SCL), PB12 (SDA) |
| 4 synth voices | PC0, PC1, PC7, PC8 |
| LED | PB4 + 330 Ω |
| Piezo | PC11 |
| Potentiometer | PC12 |
| ESP32 link (later) | PB3 + PB1 |

## Circuits (starting values, untested)
- **Piezo pad** (same for the ESP32 drum): piezo between signal and GND; **1 MΩ** across the piezo; **2.2 kΩ** from the piezo to the input pin; **3.3 V zener** from the input pin to GND (cathode on the pin side). This clamps hits to about 3.3 V on the positive side and -0.7 V on the negative. ESP32 classic has 8 usable ADC1 pins (GPIO 32-39); ESP32-S3 has 10 (GPIO 1-10), so 10 pads fit an S3 directly, or use the mux on a classic ESP32.
- **Potentiometer (20 kΩ):** ends to 3.3 V and GND, middle leg to **1 kΩ** to PC12, **100 nF** from PC12 to GND. A BIO program times the charge (about 2 ms at 20 kΩ).
- **Amp (PAM8403):** power from the board's **VBUS pin (40)**, never from a GPIO; **470 µF** across its supply. Each synth voice: GPIO, **1 kΩ**, shared node; **100 nF** from the node to GND; **10 µF** in series into the amp input.
- **LED:** PB4, **330 Ω**, LED, GND.
- **OLED:** VCC to the 3.3 V pin, GND, SCL to PB11, SDA to PB12.

---

## What changed from v1 (so nothing looks forgotten)
| v1 item | v2 | Why |
|---|---|---|
| Multimeter | **back in the list** (my mistake dropping it) | you need it |
| Soldering iron | **removed** | you already have an adjustable pencil iron |
| 1N5819 Schottky diodes x3 | replaced by **3.3 V zeners x14** | one part per piezo pad instead of two, and it also serves the 10 ESP32 drum pads |
| 1N4148 diodes x4 | kept | general spares |
| 330 ohm resistors | **restored, x3** | cheap insurance, you have some already |
| 1k, 10k, 1M, 4.7k resistors | kept (1M and a new 2.2k raised to 14 for the drum pads) | |
| 100 nF, 10 uF, 470 uF capacitors | kept; **two** 470 uF again | |
| Male header strips x2 | kept | |
| Female header strip | **kept as optional** | |
| Breadboard, jumper wires | not bought | you have breadboards and single-core wire |
| Tactile switches (spares) | not bought | you have 12 |
| Solder + flux | solder kept, flux not bought | your flux is full |
| LED, pot, piezo, PAM8403, speaker | not bought | you have them |
| NEW | 9 V battery (if needed), piezo parts for 10 drum pads, optional 16-channel mux, optional 8 ohm speaker | |
