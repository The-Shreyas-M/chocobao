# ChocoBao plan (draft v1, 8 Nov 2026)

**One line:** a pocket multitool on the Baochip: random numbers, sound, games and a signed-sensor bridge for ESP32s, all in one app image you flash once, with every app switchable at runtime. Working name **ChocoBao** (check trademarks, domains, and ask on the Discord about using "Bao").

Deadline 15 Dec 2026, 21:00 ET. Submission = repo + video + write-up. Judges score: Works / New / Open / Uses the silicon.

## Rules we keep (so the demo is honest and the chip stays healthy)
1. **ReRAM wears out (about 10k writes per cell, per Bunnie). Nothing else does.** Keep all state in RAM, write only on an explicit "save", rotate saves over many slots, never log or count in ReRAM, ship BIO code inside the app image. Dev flashing is the main wear: we are at about 9 writes of 10,000 on the app region. Flash only when needed; test on the PC first. Never flash loader/kernel again.
2. **Never touch** the one-way counters (`bootwait`, `lockdown`, `paranoid`, `require-pq`) or `self_destruct`. `baoport.py` refuses them.
3. **No overclaiming.** Our board is in developer mode (factory secrets wiped), signing is software Ed25519/secp256k1, a signature proves "this device said so", not that the physical world was true. Never write "tamper-proof".
4. **Power:** the chip cannot make 5 V. PAM8403 runs from the **VBUS pin (40)** (USB 5 V). Never feed 5 V to a GPIO. 3.3 V logic only.
5. **Pins:** BIO-safe set PB1-PB5, PC0-PC3 (PB11/PB12 are I2C). **Never PC13** (PROG), PB13/PB14 (console).

## Hardware
| Part | Use | Notes |
|---|---|---|
| Dabao | brain | 2 MiB RAM, 4 MiB ReRAM, 20 pins |
| 0.96" 128x64 I2C OLED (SSD1306) | menus, passwords, games | recommended; 0.91" 128x32 works for menus but is poor for games; skip the 0.71" |
| PAM8403 + speaker | audio | VCC from VBUS, RC filter + cap on each input |
| 3-4 push buttons | input | PB1-PB3 (+PB4) |
| piezo | drum pad / knock | needs resistor + clamp, 3.3 V max |
| LEDs + resistors | status | PB4 or PC pins |
| ESP32 | sensor node for Notary | 2-wire link via BIO UART |

Pin budget (17 BIO pins available): audio PC0-PC3, buttons PB1-PB3, LED PB4, I2C OLED PB11/PB12, ESP32 link 2 pins (PB5 + one of PC7/PC8). Fits with room to spare.

## Apps and what they exercise
| App | Chip units | Direction | Tier |
|---|---|---|---|
| **Dice**: passwords, BIP-39 passphrases, dice, typed into the PC | TRNG, USB keyboard, display | Signing (seed words), Tooling | Core, **built** |
| **Synth**: 4 voices, one per BIO core, buttons/piezo | BIO x4, TRNG | BIO | Core |
| **Notary**: ESP32 frames in over BIO UART, signed, sent back; parser isolated from signer | BIO, key store, crypto, MMU | BIO, Signing, Xous/MMU | Core |
| **Status**: dev-mode badge, boot hashes vs published, chip temperature, sensor flags | boot chain, temp sensor, sensors | Verification, Countermeasures | Core |
| **Games** (below) | OLED, buttons, BIO audio, TRNG | BIO, Xous | Core (one game), stretch (rest) |
| **Seed & Sign**: seed words from TRNG, `personal_sign` shown on OLED | TRNG, display, secp256k1 | Signing | Stretch |
| **Probe**: BIO logic analyser, streamed to PC | BIO, USB | BIO, Tooling | Stretch |
| **Notes**: small encrypted notes, save-on-demand | ReRAM (rotating slots), crypto | n/a | Stretch |

### Crypto minigames (you asked for these)
- **Mempool Dodger** (core game): arcade dodger on the OLED. Falling "blocks" and "51% attack" hazards, patterns from the TRNG, chiptune from BIO, 2 buttons. High score kept in RAM, optional save.
- **Seed Memory**: Simon-says with BIP-39 words and tones; memorise a growing seed phrase.
- **Hash Race** (benchmark toy): how many SHA-256 per second does the chip do? Show it live, compare with your ESP32.

### Direction coverage check
BIO: Synth, Notary link, Probe. Signing: Dice (seed words), Seed & Sign. Verification: Status. Countermeasures: Status (read-only sensor flags; registers exist at 0x40053000 / 0x40052000 but no app reads them yet, so unverified). Xous/MMU: separate processes, Notary parser vs signer, a hostile-app demo. Tooling: serial flasher (done, 1 s), pin checker, Windows BIO build kit (done), OLED driver, reboot command.

## Build and flash workflow
- Source of truth: `chocobao\` in this folder. `tools\build-chocobao.ps1` mirrors it into xous-core and runs `cargo xtask dabao chocobao`.
- Flash only the app: `python tools\baoflash.py <apps.uf2>` (about 1.4 s), then PROG (hold about 1 s) to start it.
- Getting into the bootloader without unplugging: RESET button, or the app's own `bootloader` command (added, untested).
- Several apps in ONE image: the packing tool accepts multiple programs (confirmed in the source). Budget: **872 KiB total**; chocobao is about 340 KB today. Measure after each app.
- Demo wrinkle: after plug-in the board waits in the bootloader until PROG is pressed (`bootwait` is enabled; changing it is irreversible). Decide at the very end.

## Milestones (38 days from 7 Nov)
1. **Now:** Dice flashed and tested; reboot command tested; order OLED (0.96", SSD1306).
2. **Week 2:** OLED driver + menu; Synth on one core, then four.
3. **Week 3:** Notary: BIO UART from an ESP32, signing, isolation demo.
4. **Week 4:** Status app, Mempool Dodger, polish.
5. **Last 5 days:** repo, README with reproduction steps, 2-minute video, write-up. No new features after day 33.

## Unknowns to settle early
- Can an app write ReRAM safely for saves, and where? (untested)
- Four BIO cores from an app (console loader only claims Core0); BIO audio quality; BIO UART accuracy at the ESP32's baud rate.
- Which Rust crates for Ed25519/secp256k1 compile in this Xous environment (the vault apps use `ed25519-compact`).
- Whether an app can map and read the sensor registers.
- Total image size once all apps are in.
