# ChocoBao Notary: the story

## One sentence
A pocket **witness**: a sensor sends an event to the Baochip, the chip stamps *when* it arrived with its own I/O core, signs it with a key that other code cannot read, and anyone can check the proof offline.

## The problem
Cheap sensor nodes (ESP32, Arduino) report readings that are easy to fake once the PC, phone or cloud in between is compromised, or when the person holding the data benefits from changing it.

| Example | Who wants to cheat |
|---|---|
| Cold-chain temperature log for vaccines or food | the shipper hiding a warm hour |
| Shock or tilt sensor on a shipped crate | anyone avoiding a damage claim |
| Machine cycle counter, lab bench event log | anyone who needs the numbers to look better |
| Door, knock or button log for an access audit | anyone editing the log afterwards |

A plain log file proves nothing: you can edit a number and nobody can tell.

## What ChocoBao does about it

| Step | What happens | Why it matters |
|---|---|---|
| 1. Sensor node | An ESP32 sends an 8-bit reading as 10 pulses on one wire | the sensor stays cheap and untrusted |
| 2. Timestamp | A Baochip **I/O (BIO) core** watches the wire once per microsecond and records its own clock at the first edge | the time does not depend on what the CPU is doing (screen, USB, music), so software load cannot smear it |
| 3. Sign | The app signs `CHOCONOTARY1\|counter\|ms\|event\|bio-ticks` with Ed25519 | the signature covers the reading, the order (counter) and the chip-clock stamp |
| 4. Key safety | The key is made by the chip's hardware random-number generator and lives on its own memory page of the main app. Another process (the sandbox) that tries to read it is **killed by the kernel**; the key still signs | a misbehaving game or plugin cannot steal or tamper with it |
| 5. Proof out | Hold OK: the device types the proof into any PC (USB keyboard, no driver). `tools/verify.py` checks it with pure Python | no cloud, no app to trust, 30 lines to audit |

Change one character of the reading and the verifier says INVALID. Replay an old line out of order and the counter check complains.

## Why it needs this chip

| Baochip feature | Role | A typical microcontroller board |
|---|---|---|
| BIO cores | cycle-exact capture of pulses with zero CPU involvement | interrupts that jitter when the CPU is busy |
| MMU with real processes (Xous) | the sandbox cannot read the signer's memory; the kernel kills it | one flat address space: any bug can read the key |
| Hardware TRNG | the signing key is born unpredictable | weaker or software random numbers |
| Open design and open firmware | the whole path can be audited and reproduced | closed or half-closed parts |

## What the demo shows (2 minutes)
1. Press the ESP32 button. The Dabao beeps, the Notary screen shows `ESP32:v=6:bio=...` and a signature.
2. Hold OK: the proof types itself into a text file on the PC.
3. `python tools/verify.py proofs.txt` prints `VALID`. Change the value and it prints `INVALID`.
4. Lab -> MMU demo, press OK: "KILLED by the MMU", "key intact, sig VALID". A second process just tried to read the key and the kernel stopped it.
5. Quick tour of the rest: 21 games including Doom, music played by a BIO core, settings that survive a restart.

## Honest limits
| Limit | What it means |
|---|---|
| The sensor can lie | the Notary proves *when this value arrived and that nobody changed it afterwards*, not that the sensor told the truth |
| Developer mode | the board is not locked down; someone holding the board with a debugger is out of scope |
| Key is RAM only | it is new every power-up; the public key must be recorded when the first proof is made |
| Code in the main app can read the key | only *other processes* are isolated; moving the signer into its own process is the next step |
| The BIO clock count is 30 bits | it wraps about every 1.5 s, so it places an event inside a second; the millisecond field orders events across seconds |

## Why it is worth a prize
- It does something a hobby board normally cannot: a signed, precisely timed record of a physical event, with isolation you can watch the kernel enforce.
- It uses four chip features that fit together instead of one feature in isolation.
- Everything is open, small and checkable: firmware, protocol (below), verifier, build steps.
- It is real and runs on the desk: wire, button, screen, proof, verify.
- The rest of the device (games, music, tools) shows the chip is a full small computer, not a single trick.

## Protocol
A proof is one line, fields separated by `|`:

`CHOCONOTARY1|count|ms|event|signature_hex|public_key_hex`

| Field | Meaning |
|---|---|
| count | 1, 2, 3 ... per power-up; must only increase |
| ms | milliseconds since the app started (CPU clock) |
| event | for the ESP32: `ESP32:v=<0..255>:bio=<30-bit BIO clock count at the first pulse>`; for the buttons: `UP`, `DOWN`, `LEFT`, `RIGHT` |
| signature | Ed25519 over the first four fields joined with `|` |
| public key | 32-byte Ed25519 key of this power-up |

Sensor frame (ESP32 to Dabao, one wire, rising edges): edge 0 start; edge 1 one reference gap T later; then 8 data edges, each a gap of T for bit 0 or 2T for bit 1, most significant bit first. The reference gap travels with every frame, so the decoder needs no clock calibration.
