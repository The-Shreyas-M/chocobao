# ChocoBao voiceover script

Read each block in order. Pause one second between blocks. Speak slowly. Total about 2 minutes.
Record in a quiet room, phone 20 cm from your mouth, one take per block is fine, I will cut them.

## How to record

| Choice | What to do |
|---|---|
| Audio only | Phone voice recorder, send me the file (m4a or mp3 is fine) |
| Face on camera | Phone landscape, face lit from the front, send the video; I put it in a corner of the demo |
| Names | Say "Baochip", "Dabao", "ESP32", "Ed25519" (ed twenty-five five-one-nine) |

## Script (blocks match the order of your demo video)

| # | Block | Say |
|---|---|---|
| 1 | Intro | "This is ChocoBao, a pocket witness built on the Baochip Dabao. It also plays games, but first I will show the real point." |
| 2 | ESP32 and wire | "A cheap ESP32 sensor node sends one reading over one wire. The ESP32 is not trusted." |
| 3 | Notary, press BOOT | "I press the button on the ESP32. A Baochip I/O core stamps the pulses with its own clock. The CPU is not involved, so the screen, USB and music cannot smear the time. The device signs the reading with Ed25519." |
| 4 | Hold OK, text typed | "I hold OK, and the device types the proof into the PC. It is a USB keyboard, so there is no driver and no cloud." |
| 5 | verify.py VALID | "Anyone can check this offline with a short Python script. It says valid." |
| 6 | Edit one character | "I change one character, and the check fails." |
| 7 | MMU demo | "The signing key lives in its own process. Here a second, untrusted process is told exactly where the key is, and tries to read it. The kernel's memory protection kills only that process. The app and the signer carry on, and the key still signs. On most microcontrollers, any bug can read the key." |
| 8 | Quick cuts | "It is also a small computer: twenty-one games including Doom, music made by an I/O core, tools, and settings saved in the chip." |
| 9 | Limits | "The limits: the sensor itself could lie, so this proves when a value arrived and that nobody changed it, not that it is true. The board is in developer mode, and the key is new every power-up." |
| 10 | Silicon | "It uses the I/O cores, the memory management unit with separate processes, the hardware random number generator, and the chip's Ed25519 engine." |
| 11 | Close | "Everything is open source: github dot com slash The-Shreyas-M slash chocobao. Thank you." |

## Send me

| Item | Where |
|---|---|
| Demo video from the phone | Put the file in the project folder, tell me the file name |
| Voiceover audio or face video | Same folder, tell me the file name |
| Anything you want cut | Tell me the time in the video (for example "cut 0:40 to 0:55") |
