# How ChocoBao wins (strategy, 10 Nov 2026)

Judges score **Works / New / Open / Uses the silicon**. 35 days left (deadline 15 Dec, 21:00 ET).
Status labels: **[verified]** = we ran it on the board. **[in repo]** = source exists in the cloned repos, not tried by us. **[idea]** = my reasoning only.

## 1. What this chip has that others don't

| Chip feature | What it makes possible | Status for us |
|---|---|---|
| **MMU on a microcontroller** (Xous: every service is its own process) | Untrusted code (a game, a plugin) can run next to a secret-holding service and *cannot read its memory* | Xous runs [verified]; our own second process not yet |
| **4 BIO cores (PicoRV32 @ 700 MHz)**, 4 KiB code each, FIFOs | Cycle-exact pin work with zero CPU load: UART/I2S/WS2812/logic capture, PWM and tones for a passive buzzer | A BIO program ran on the board via `bio-loader` [verified]; `libs/bio-lib` lets a normal Xous app load BIO code [in repo] |
| **Hardware TRNG** | Keys, nonces, passwords, dice with no bias | Used by passwords/dice/games, live "Entropy meter" on screen [verified] |
| **Crypto engines + Ed25519** (`ed25519-dalek-bao1x`, `curve25519-dalek-bao1x`) | Fast signatures in Rust, hardware assisted | [in repo], not tried |
| **ReRAM + one-way counters** | Non-volatile monotonic counter (anti-replay) | Exists [verified by reading]; wear ~10k writes, so used sparingly [rule] |
| **Open RTL, open bootloader, serial flasher** | Anyone can audit and reproduce | Our serial flasher is 2 s per flash [verified] |
| **USB (HID keyboard) from a Rust app** | Device types secrets into any PC with no driver | [verified] |

What a normal ESP32/RP2040 cannot do: isolate code from code, and also watch pins with nanosecond timing, on one die, with open design.
What the chip cannot do (be honest in the write-up): no radio, no FPU, our board is in developer mode so keys are **not** protected against someone with the board in hand. Never write "tamper-proof".

## 2. The idea I would put the whole project behind

### "ChocoBao Notary": a pocket witness you can plug between a sensor and a PC
A sensor node (your ESP32) streams readings to the Dabao. The Dabao timestamps them with **BIO cores** (exact capture, no CPU jitter), hashes and **signs** them inside an **isolated process that no other app can read**, shows the verdict on the OLED, and hands back a signed record that anyone can verify offline with a 30-line Python script.

Why this one fits the judges:
- **Uses the silicon** in four distinct ways at once: BIO (capture), MMU (isolation), TRNG (key + nonces), crypto (Ed25519). Most entries will use one.
- **New:** "signed physical measurements from a hobby board" with a *live isolation proof* is rare; most hardware-wallet clones just sign transactions.
- **Open:** all code + a verifier script + a one-page protocol spec in the repo.
- **Works:** demo is physical: knock the piezo / press the ESP32 sensor, the OLED shows `SIGNED #17 ok`, the PC verifier prints `VALID`; flip one byte and it prints `INVALID`.

### The killer demo (30 seconds of video)
1. Menu → **Games → Snake** (the untrusted app).
2. Press a hidden "attack" combo: the game tries to read the Notary's key memory.
3. The MMU faults the *game*, the OLED shows `ACCESS DENIED - game killed`, the menu comes back, the Notary keeps signing.
That is the Baochip's headline claim (MMU + open + verifiable) shown on a 1-inch screen. [idea; needs a second Xous process, see risks]

### The fun shell (already built, keeps the demo lively)
Boot animation, password/PIN/dice/passphrase typing, six games, stopwatch, 8-ball, live TRNG test. These cost little now and make the video watchable. Do **not** let them grow further after week 2.

## 3. Plan by week (what gets built, in order)

| When | Build | "Done" means |
|---|---|---|
| **Now (10-16 Nov)** | Stabilise firmware v0.4 on the board; buzzer working; repo tidy | no freezes in 30 min of use |
| **17-23 Nov** | **BIO capture**: ESP32 TX -> BIO UART RX -> frames to the Xous app via FIFO. Also BIO tone generator for the buzzer (replaces the 500 Hz hack) | ESP32 prints "hello", OLED shows the same bytes |
| **24-30 Nov** | **Notary core**: Ed25519 key from TRNG, sign(frame || counter || BIO timestamp), keep counter in RAM; PC verifier script | `verify.py record.json` -> VALID / INVALID |
| **1-7 Dec** | **Isolation**: put the signer in its own process, game in another, IPC `sign(hash)` only; the attack demo | game cannot read key; kill shown on OLED |
| **8-12 Dec** | Polish: README with reproduction steps, protocol spec, video, write-up with the limits spelled out | a stranger can reproduce from the README |
| **13-15 Dec** | Buffer only. No new features after 12 Dec | submitted a day early |

## 4. Risks and the fallback for each
- **Second Xous process may be hard to package** (the app packer accepts several programs [in repo], untested). Fallback: two *threads* plus a clearly worded claim ("logical separation, MMU separation next"); never claim MMU isolation we did not demonstrate.
- **BIO UART needs timing work.** Fallback: use the HAL's UART (UDMA) for the ESP32 link and keep BIO for the buzzer/tone + a logic-capture side demo.
- **Ed25519 crate may not build for the Xous target.** Fallback: `curve25519-dalek` plain build; signing speed is irrelevant at 1 record/s.
- **ReRAM wear:** counter lives in RAM; saving is an explicit menu action, rotated slots. Flash count so far ~20 of ~10,000.
- **Hands-free bootloader entry does not work yet** (PC13-low + CPU reset lands back in the app). Not a judging item; keep PROG+RESET for flashing and do fewer, bigger flashes.

## 5. Other ideas I considered and rejected
- **Audio synth / chiptune on 4 BIO cores:** great sound demo, weak "New", and the audio hardware (amp, speaker) is the risky part. Keep as stretch: the BIO tone generator gives most of the benefit.
- **Hardware wallet:** crowded space and our dev-mode board cannot honestly claim wallet-grade security.
- **Logic analyser / protocol sniffer on BIO:** very on-theme, but it is a tool, not a story. Becomes a stretch "Probe" screen that reuses the capture code.
- **Another game console:** judged as "fun" only.

## 6. What I need from you
- Tell me which buzzer sound you hear in **Tools -> Buzzer test** (steady / fast buzz / slow buzz). That decides the buzzer code.
- The ESP32 you want as the sensor node (S3 is fine) and one sensor (the piezo pad or a button is enough for week 1).
- A licence for the repo (I suggest Apache-2.0 + a note that Xous-derived files keep their upstream licence).
