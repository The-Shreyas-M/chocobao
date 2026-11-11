# Hackathon direction (revised 7 Nov 2026)

The first list (flash impersonator, pin personalities, USB sentinel, glitch lab, MMU arena) was dropped: it was written for judges, not for you. This version starts from what you own and what amuses you, then checks it against the judges' page.

Nothing here is built or tested. "Unverified" marks anything I have not confirmed on the board or in source.

## What the hackathon page actually rewards
Source: crossbar-inc.com/pages/dabao-hackathon.
- Tagline: "Show us something cool on an open chip." No categories, no required stack. Up to $5,000.
- Four criteria: **Works** (live on the board, not a slide), **New** (something the chip enables that other boards don't), **Open** (reproducible by the next person), **Uses the silicon** (BIO, MMU, crypto, IRIS; "the more of the chip, the better").
- It says outright that **"Tools count as cool"**, and lists BIO (a display, a new protocol, a sensor array), signing, verification, countermeasures, and Xous apps that need the MMU.
- It **never mentions audio**. Almost nobody will build audio, so audio is the lane where "New" is easy.
- Hardware: 20 I/Os pinned out, USB 2.0 HS, SPI, I2C, UART, ADC, camera, TRNG, key store, glitch sensors. No Wi-Fi/BT (fine for what follows).

## What you own and what it is good for
| You have | Role in the build |
|---|---|
| Piezo sensors | Secret-knock input and drum pads (analog spike, needs a clamp: GPIO is 1.8-3.3 V, not 5 V tolerant) |
| MAX heart-rate sensor | Pulse and stress signal over I2C |
| Speakers + PAM8403 | Sound output. PAM8403 is an analog amp, so it needs a PWM/PDM signal with an RC filter, not I2S |
| Buttons, LEDs, resistors | Inputs, status, plus the clamp for the piezo |
| Tiny display (planned, under ~INR 200) | A 0.96" 128x64 SSD1306 I2C OLED: 2 data wires, shares the I2C bus with the heart sensor |

Budget answer: the OLED is the only purchase I'd make. Maybe a zener diode or two for the piezo clamp (a few rupees) and pin headers.

## The idea: DECOY, a pocket vault that can lie

A USB password vault the size of a stick, built from your parts, that types passwords into your PC and can show a believable fake vault to anyone forcing you to unlock it, while your real vault stays hidden and intact.

**Your Gemini idea's catch ("if the key gets erased, how do I get it back?") has a standard answer: never erase.** Use a hidden volume, the same idea as VeraCrypt:
- Two secrets (PIN, knock pattern, or both) each derive a different key.
- The storage area is filled with random-looking data. A real vault and a decoy vault are both just encrypted blobs, and the unused space is random, so an inspector can't tell how many vaults exist.
- The duress secret opens the decoy. The real secret opens the real one. Nothing is destroyed, so you can use the real vault "like normal after the threat is gone."

**How it stacks up (each layer is a separate demoable stage):**
1. **Vault core:** encrypted entries in the chip's ReRAM, keys from the TRNG, AES through the chip's crypto engine. (Silicon: crypto, TRNG, ReRAM.)
2. **Types passwords over USB** like a keyboard (and possibly acts as a passkey device). The board already enumerates a HID keyboard and a FIDO device after our flash. (Silicon: USB. Unverified: whether a Xous app can drive the HID endpoint.)
3. **Secret-knock unlock:** knock a rhythm on the piezo; BIO samples it with exact timing. A different rhythm opens the decoy. (Silicon: BIO, ADC. Unverified: ADC usable from a Xous app; our pins.csv shows ADC0 on PC9.)
4. **Sound:** a PAM8403 speaker gives feedback. BIO generates a high-rate PDM tone stream; the real and decoy unlocks play the *same* chime so a watcher can't tell. This also makes a playable piezo-drum synth as an extra. (Silicon: BIO. Unverified: audio quality; the PAM8403 and an RC filter won't be hi-fi, so don't promise it.)
5. **Screen:** the OLED shows entries; real and decoy screens look the same. (I2C. Unverified: which controller your module has: SSD1306 vs SH1106.)
6. **Stress sense (stretch):** the heart sensor spots a racing pulse at unlock and offers the decoy. Treat it as a soft signal, not a lock: false positives would be annoying.
7. **MMU proof (stretch):** the vault runs in its own Xous process; a deliberately hostile app tries to read it and the MMU stops it. (Silicon: MMU.)

Silicon touched if you finish the stack: BIO, MMU, crypto, TRNG, ReRAM, USB, ADC. Only IRIS is missing.

**Why it can win:** it hits all four criteria, it's a real working device in your hands, it combines security, audio and hardware sensing in a way nobody else will, and every stage is demoable on its own, so a half-finished version still wins points. **Why it's yours:** it's the combination of your own parts (piezo + pulse + speaker) and your own wish (a personal password vault).

## Verified against the cloned source (7 Nov), which changes the plan
Read from `repos/xous-core`, `repos/dabao`, `repos/bio-sim`; nothing below is a web guess.
- **USB keyboard typing works today**: `usb type <text>` in dabao-console calls `UsbHid::send_str` (`apps-dabao/dabao-console/src/cmds/usb.rs`). Stage 2 is a confirmed building block.
- **Passkeys are NOT ready**: `ctap.rs` is a 53-line INIT/PING stub. A real FIDO2 passkey is a big project, so it's out of scope. Drop it.
- **Hardware AES and TRNG exist and have test commands** (`aes check128/256`, `trng pump`); the keystore service has an `AesOracle` and key-wrap opcode. The vault should use these, not hand-rolled crypto.
- **ADC is a trap**: the HAL driver exists (PA04-PA07, about 1 MHz ADC clock, at least 14 clocks per sample so about 70 kS/s), but the Dabao pinout page says "ADC drivers are under development", and the board's schematic exposes no PA pins at all (pins.csv labels `PC9` as ADC0, which contradicts the HAL). **Don't build on the ADC.** Knock sensing should use a digital pin instead: piezo through a resistor and clamp into a GPIO, with BIO timing the edges. `bio-lib` already has a BIO capacitive-touch program (`captouch.rs`, PB1), so touch pads can also replace buttons.
- **Audio has no library**: BIO examples cover WS2812 LEDs, touch, I2S *input* (`bio-sim/sw/i2s`), DALI and a QSPI-to-8080 bridge. Audio output would be our own BIO program (PDM/PWM tone into the PAM8403 through an RC filter). Plausible but unverified, and it's the "New" part.
- **Display**: the HAL has a driver for the baosec's SH1107 128x128 OLED (`sh1107.rs`) and I2C-over-uDMA (`udma/i2c.rs`). A cheap 128x64 SSD1306/SH1106 needs a small driver of our own; the Dabao's I2C0 is PB11/PB12.
- **Storage is the biggest unknown.** The Dabao image has no PDDB (no external flash). Its 4 MiB ReRAM holds boot and kernel (below 0x30_0000), the app region (0x2F_xxxx..; 0xDA000 = 872 KiB, our `dabao-console` uses about 540 KB), and 64 KiB of access-controlled data slots at 0x603E_0000. The HAL has a `Reram` writer that a std app can map (`add_range`), but nobody in-tree uses it from a detached app. Plan: test a harmless write in the unused tail of the app region first. **Never write near the kernel, boot, or data-slot ranges.**
- **PROG is wired to PC13** (the same pin as the USB SE0 switch): never use PC13.
- **`bootwait` is Enabled on your board** (read-only `bootwait check`). That's why the `BAOCHIP` drive appears by itself on every plug-in, and why PROG starts the app. Changing it burns an irreversible one-way counter, so we leave it. Plug-and-play therefore means: one command plus one PROG press.

## Honest caveats
- **Security claims need care.** I'm not a cryptographer, and neither of us should invent crypto. Use vetted primitives (the chip's AES, the TRNG, a standard KDF) and never hand-roll. The famous deniability failures are leaks: file sizes, write counters, timing, ReRAM wear patterns. A hackathon demo should be labelled a prototype. **Don't put your real passwords in it** without a proper review.
- **Our board is in developer mode.** The factory secrets were wiped when we flashed, so hardware-attestation claims don't apply. We can still demo the vault design honestly.
- Gemini overstated some points: "impossible to extract without destroying the chip" isn't something I can verify, and a wipe-on-duress design means one wrong PIN loses everything. Hidden-volume wallets (BIP-39 passphrases) also already exist on other wallets, so our novelty is the device-level deniable storage plus the physical duress inputs (knock, pulse), not the idea itself.
- **NFTs:** I'd skip minting. If you want a nod to it, the instrument can *sign* each performance with an on-chip key as proof of play, and a PC tool could mint later. It's a gimmick with real on-chain verification costs and a crowded Ethereum lane, so it's low priority.
- **Scope:** seven stages in ~5 weeks is a lot for one person. The rule: finish stage 1-3 before touching 4-7, and ship whatever works.
- **Wiring risks to check before soldering:** the piezo clamp; the heart-sensor module's pull-up rail (measure SDA/SCL idle voltage, it must be <= 3.3 V); common ground with the PAM8403; PAM8403 power from VBUS (5 V) only, never to a GPIO.

## Plug-and-play like your ESP32
Honest picture: the ESP32 trick (auto-reset by a USB-UART chip toggling DTR/RTS) doesn't exist here, because the USB port is native. What we can do: our `tools/flash.ps1` already builds, waits for the drive and copies. The remaining manual step is one PROG press. Boot1's source says the board stays in the bootloader at power-up when the `bootwait` flag is enabled, and boots the app straight away when disabled. **Do not change that flag**: toggling it burns an irreversible one-way counter. We'll first run the read-only `bootwait check` when the board is back on USB, then decide.

## Next steps
1. No-solder feasibility probes: `usb`, `test`, `touch` console commands; find out how a Xous app drives HID, ADC and the crypto services.
2. Decide the first stage to build (vault core as a serial-driven Xous app).
3. Buy the OLED; solder headers only once a pin plan exists.
