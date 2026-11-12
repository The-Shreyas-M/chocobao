# ChocoBao video script (about 2 minutes)

Goal in one line: **a pocket witness. A sensor event is timestamped by an I/O core, signed by a key that lives in its own process, and anyone can verify it. A hostile process cannot touch the key.**

## Before you press record

| Do | Why |
|---|---|
| Plug the Dabao in, press **PROG** once so the app starts (the board waits in the bootloader after power-up) | otherwise the screen stays blank |
| Do **not** open Lab -> MMU demo before recording | the sandbox can only be killed once per power-up; to retry, unplug, replug, press PROG |
| ESP32 powered from its USB cable, wire GPIO5 -> PC8 and GND -> GND in view | the viewer sees the real wire |
| On the PC: a text editor open and focused (this is where the proof is typed), a terminal open in the project folder | the proof arrives by USB keyboard |
| Terminal ready: `python tools/verify.py proofs.txt` (file will be created from the typed text) | live verification |
| Record the PC screen, and film the Dabao OLED with a phone held close | the OLED text is small |
| Speak slowly, short sentences | judges read on the second watch |

## The 2 minutes

| Time | Show | Do | Say |
|---|---|---|---|
| 0:00-0:10 | The Dabao on the desk, home screen (bao in the microwave) | nothing | "This is ChocoBao, a pocket witness built on the Baochip Dabao. It also plays games, but I'll show the real point first." |
| 0:10-0:25 | ESP32 and the wire to the Dabao | point at the single wire | "A cheap ESP32 sensor node sends one reading over one wire. The ESP32 is not trusted." |
| 0:25-0:55 | Dabao: Lab -> Notary. Then the ESP32 | press the ESP32 **BOOT** button; show the OLED line `ESP32:v=..:bio=..` and the signature | "A Baochip I/O core stamps the pulses with its own clock. The CPU is not involved, so screen, USB and music cannot smear the time. The signature is Ed25519." |
| 0:55-1:10 | PC editor focused | on the Dabao **hold OK** until the proof is typed into the editor | "Hold OK and the device types the proof into any PC. No driver, no cloud." |
| 1:10-1:30 | Terminal | save the text as `proofs.txt`, run `python tools/verify.py proofs.txt` -> **VALID**. Edit one character in the file, run again -> **INVALID** | "Anyone can check this offline with a short Python script. Change one character and it fails." |
| 1:30-1:55 | Dabao: Lab -> MMU demo | press **OK** once; read the screen: `KILLED by the MMU`, `signer signs, same key` | "The signing key lives in its own process. Here a second, untrusted process is told exactly where the key is and tries to read it. The kernel's memory protection kills only that process. The app and the signer carry on and the key still signs. On most microcontrollers any bug can read the key." |
| 1:55-2:10 | Quick cuts: Doom, Ninja Run, a song playing, Settings | a few seconds each | "The same device is a full small computer: 21 games, music made by an I/O core, tools, settings saved in the chip. All open source." |
| 2:10-2:20 | Honest limits on screen or spoken | none | "Limits: the sensor itself could lie, so this proves when a value arrived and that nobody changed it, not that it is true. The board is in developer mode and the key is new every power-up." |
| 2:20-2:25 | GitHub page | none | "Everything is open: github.com/The-Shreyas-M/chocobao. Thank you." |

## If something goes wrong during the take

| Problem | Fix |
|---|---|
| Notary shows no ESP32 line | check the wire GPIO5 -> PC8; press BOOT again; `python tools/diag_once.py` shows `ev=edges/frames/bad` counting up |
| Proof not typed | click into the editor first, then hold OK about one second |
| MMU demo says "sandbox was already killed" | unplug, replug, press PROG, then run it again |
| Screen blank after power-up | press PROG |

## One-sentence answers for the questions judges ask

| Question | Answer |
|---|---|
| Why is this new? | A signed, precisely timed record of a physical event from a hobby board, with key isolation you can watch the kernel enforce. |
| What does the chip give you that others do not? | I/O cores for jitter-free timing, a real MMU with separate processes, a hardware random-number generator, and an open design. |
| Can the sensor lie? | Yes. The Notary proves when the value arrived and that it was not altered afterwards. |
| Is the key safe? | It lives in its own process and the kernel kills anything else that touches it. The board is in developer mode, so it is not hardened against someone holding it with a debugger. |
| Can I reproduce it? | Yes: README, BUILD_GUIDE (including the ESP32 table), `docs/STORY.md`, a patch for xous-core, Apache-2.0 license. |
