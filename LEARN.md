# LEARN.md: crypto wallets and this chip, from zero

Written 7 Nov 2026. Sources: the cloned repos (`repos/xous-core`, `repos/bio-sim`, `repos/dabao`), crossbar-inc.com, baochip.com, Bunnie's blog post "Baochip-1x: A Mostly-Open, 22nm SoC for High Assurance Applications", and web searches on wallets. Marked **[my inference]** = my own reasoning, not a source. Web search results about wallets came mostly from review blogs and vendor pages; treat specific wallet claims as "check the vendor docs before relying on them".

## 1. Crypto basics in plain language

- **Blockchain / Ethereum:** a shared public ledger that anyone can read. Your balance is just an entry that only *you* can move.
- **Private key:** a huge secret number. Whoever knows it controls the money. There is no "forgot password" button.
- **Address:** a public identifier derived from the key. Safe to share. You can't reverse it back to the key.
- **Seed phrase (12 or 24 words):** a human-readable backup from which *all* your keys are derived (standards called BIP-39 and BIP-32). Lose the words and the device, and the money is gone. Someone who sees the words owns everything.
- **Signing:** to spend, you create a signature over the transaction with the private key. It proves "the key holder approved *this exact transaction*" without revealing the key. Hardware wallets exist so signing happens inside a device while the key never touches your (possibly infected) computer.
- **What you sign on Ethereum:** EIP-1559 transactions (normal transfers and contract calls), EIP-191 messages (sign-in style text), EIP-712 typed data (structured permissions), EIP-7702 delegations (authorizing a smart-contract account). The Ostros flyer lists exactly these four.
- **Clear signing vs blind signing:** the device must *show you in plain words* what you're approving. If it only shows a hash or hex, you're signing blind. The February 2025 Bybit theft (about $1.5 billion) worked by tricking signers about what they were approving, not by breaking the hardware wallets.

## 2. What a hardware wallet is

A small device with a **microcontroller** (the brain), usually a **secure element** (a hardened chip for secrets), a **screen**, and **buttons**. It holds the seed, shows what you're signing, and signs only after you press confirm.

Threats a wallet tries to resist: malware on your PC; a tampered device from the supply chain; physical theft; being **forced to unlock it** (the "$5 wrench attack"); **fault injection** (zapping the chip with a glitch or laser to skip a security check); and **blind signing**.

## 3. Do existing wallets already do "duress / decoy wallet"? Yes.

| Wallet | What it offers (per web results, verify before relying) |
|---|---|
| **Coldcard** | The strongest. A **duress PIN** opens a separate wallet derived from the same seed words; a **Brick Me PIN** destroys the secure element; a **countdown** PIN adds a covert delay. Caveat from their own docs: you can't switch back from the decoy to the real wallet without the real PIN's route. |
| **Ledger** | Passphrase wallets, and a **secondary PIN** that opens a passphrase-protected wallet. |
| **Trezor** | Hidden wallets via a **BIP-39 passphrase** (never stored on the device). No built-in duress PIN. A forum thread says the passphrase wallet can show alongside the standard one after unlocking, so "hidden" depends on how you use it. |

So the "fake wallet for a thief" concept is **already shipping**, mainly on Coldcard. A project that only re-does it would not be new.

## 4. What current wallets can't do (and where I'm unsure)

- **Be verified down to the silicon.** Mainstream secure elements are closed, so you trust the vendor. Ledger's Donjon lab showed a **laser fault-injection attack on Tangem cards** (reported lab cost about $250,000; Tangem disputes the real-world risk) and found a flaw in **TROPIC01**, the "transparent" open-architecture secure element used in Trezor Safe 7 (reported as unpatchable by a standard update; Trezor says funds remain protected). I only read secondary reports for both.
- **Clear signing is incomplete.** Trezor's open clear-signing covers contracts in its registry and warns "blind" for unlisted ones; standards (ERC-7730, an EF clear-signing standard in 2026) are still rolling out.
- **Run third-party apps safely on the device.** Most wallets ship one firmware. Xous gives apps separate memory spaces via the MMU.
- **Anything about inheritance, dead-man's switches, or wrench-attack features beyond PIN tricks:** my searches returned nothing, so I can't claim that's a gap. It needs more research before we build on it.
- **Be fully "mine to program".** Ostros is exactly that pitch: your own Rust app next to the signer.

## 5. This chip, in plain words

**Baochip-1x** (CrossBar's sister chip is **Daric**): one 22 nm chip containing a whole computer plus a security system. The **Dabao** is a bare board with just the chip, USB-C and two buttons. The device at the stall was most likely **Ostros** (CrossBar's wallet: 2.8" E Ink touch screen, USB-C, Ethereum signer on Xous; the flyer says fingerprint/NFC/BLE come with later firmware).

| Part | What it is | Analogy |
|---|---|---|
| **CPU: VexRiscv, 350 MHz, with MMU** | The main brain. The MMU gives each program its own private memory. | Apartment walls: one app can't read another's room. Normally missing on small chips. |
| **BIO: 4 PicoRV32 cores at 700 MHz** | Four tiny programmable assistants for pins; they can wiggle and watch pins with exact timing. | Four stopwatch-precise helpers. They can speak protocols (SPI, I2S, WS2812 LEDs...) without bothering the main CPU. |
| **4 MiB ReRAM** | Storage on the chip itself, non-volatile (like flash). | A notebook inside the chip; nothing to unsolder. |
| **Crypto engines + TRNG** | Hardware AES, hashing, big-number math and a true random number generator. | Dedicated calculators for cryptography. |
| **Glitch sensors + security mesh** | Circuits that notice voltage/clock tampering and probing of the chip surface. | A tripwire foil over the chip. |
| **Key slots + one-way counters** | Hardware-protected key storage and counters that can only go up. | A ratchet fuse: changes can't be undone. |
| **Boot chain: boot0 (locked) -> boot1 -> OS** | Each stage checks the next one's signature. | A chain of trust from silicon upward. |
| **Xous OS** | A small Rust operating system built around the MMU. | Each service is its own process. |

What's "open": the logic that computes on data is published for inspection. What's closed: the analog parts (USB PHY, PLL, voltage regulators, pads), which Bunnie argues act as "wires" you can check from the outside. He openly calls it "mostly open", not fully open.

**IRIS** is Bunnie's way of looking *through* the back of the chip with infrared to check that the transistors match the published design. Our Dabao is built in a package that allows it; doing so needs lab gear and we don't have any.

## 6. How it differs from boards you know

| | ESP32 | RP2350 (Pico 2) | **Baochip-1x (Dabao)** |
|---|---|---|---|
| Wi-Fi / Bluetooth | Yes | No | **No** |
| Programmable I/O | Limited | PIO (8 state machines) | **BIO (4 real RISC-V cores)** |
| Memory isolation between apps | Weak | Arm TrustZone-M | **Full MMU** |
| Secrets hardware | eFuses | Secure boot | **Key slots, mesh, glitch sensors, one-way counters** |
| Openness | Closed silicon | Closed silicon | **RTL open, boot chain open** |
| Best at | Connected gadgets | Cheap hobby I/O | **Isolated, verifiable, secret-keeping gadgets with exact I/O timing** |

Honest limits: it is **not** a radio chip, it is **not** as fast as a big Arm core, and its security claims are **untested** (Bunnie himself invites red-team attacks on the countermeasures).

## 6b. It is a computer: the real specs

Everything below is read from the cloned source (`libs/bao1x-api`, `utralib`, `docs/`) unless marked otherwise.

| Item | Baochip-1x on the Dabao |
|---|---|
| **Main CPU** | **32-bit** RISC-V (VexRiscv, `RV32IMAC`), **one core**, 350 MHz, with an MMU. I = base integers, M = hardware multiply/divide, A = atomics, C = compressed instructions. |
| **No FPU** | There is **no hardware floating point** (no F/D extension). `float` math runs in slow software, so use fixed-point for audio and DSP (BIO's own math library is fixed-point). |
| **I/O co-processors** | BIO: 4 x PicoRV32 (RV32E, 16 registers) at 700 MHz, 4 KiB of code/data each, 8-deep FIFOs. |
| **RAM** | **2 MiB SRAM** (0x6100_0000). Plus 2 x 128 KiB "IFRAM" used as DMA buffers by USB, I2C, UART. |
| **Storage / "ROM"** | **4 MiB ReRAM** (0x6000_0000), non-volatile, runs code in place. There is no separate user-visible ROM and no external flash on the Dabao. boot0 is a locked region of the same ReRAM. |
| **Clocks** | System clock 700 MHz default (`DEFAULT_FCLK_FREQUENCY`); CPU runs at 350 MHz. |
| **I/O** | 20 pins broken out; 3.3 V logic (pads accept 1.8-3.3 V; not 5 V tolerant); UART, I2C, SPI, PWM (8 channels in the MicroPython port), SD/camera blocks (drivers still in development). |
| **USB** | USB 2.0 **High Speed** device (480 Mbit/s), via the USB-C port. |
| **Crypto** | Hardware AES, hashing, public-key engine, TRNG; key slots; one-way counters; glitch sensors; security mesh. |
| **Not there** | Wi-Fi, Bluetooth, hardware float, external flash or RAM on Dabao, a reliable RTC (the community MicroPython port reports the RC-based RTC drifts roughly 200 s/day). |

**ReRAM memory map (Dabao):**
```
0x000000  boot0  (locked, 128 KiB)
0x020000  boot1  (updatable bootloader, 256 KiB)
0x060000  loader
0x0A0000  Xous kernel + services (the xous.uf2 we built is about 2.3 MB)
0x300000  application region, 0xDA000 = 872 KiB   <- your Xous app lives here
0x3DA000  end of user-writable storage; key/data slots (64 KiB, access-controlled) after
```
User-writable space is `0x60000 .. 0x3DA000`, about **3.5 MiB**.

**What size of program fits:**
| Way to run code | Code space | Notes |
|---|---|---|
| **Xous app** (what we flashed) | about **872 KiB** (our `dabao-console` is ~540 KB) | Rust `std`, MMU isolation, but the kernel already uses most of the ReRAM and a share of the 2 MiB RAM. |
| **Baremetal** (`cargo xtask baremetal-bao1x`) | up to **~3.5 MiB** | `no_std` + `alloc`, USB serial console, no OS. The closest thing to Arduino-style "just my code". |
| **MicroPython** (community port, experimental) | small | `Pin`, I2C, SPI, PWM, ADC, TRNG, USB REPL. No filesystem or UART yet. |

Fits: USB gadgets, sensor devices, signers and vaults, small OLED UIs, audio synthesis, protocol bridges, BIO-driven custom I/O. Doesn't fit: anything wireless, camera streaming, real machine-learning models, big GUIs or web servers, float-heavy DSP unless rewritten in fixed-point.

## 6c. Versus your ESP32 and ESP32-S3

The ESP32 columns are from Espressif's public datasheets as I remember them, **not fetched this session; verify before quoting them**.

| | ESP32 (classic) | ESP32-S3 | **Baochip-1x (Dabao)** |
|---|---|---|---|
| CPU | Xtensa LX6, 32-bit, dual core | Xtensa LX7, 32-bit, dual core | RISC-V RV32IMAC, 32-bit, **single core** |
| Clock | up to 240 MHz | up to 240 MHz | 350 MHz |
| Hardware float | single precision | single precision | **none** |
| SRAM | 520 KB | 512 KB | **2 MiB** |
| Code storage | external flash, typically 4 MB | external flash, typically 8-16 MB, optional PSRAM | **4 MiB ReRAM on-chip**, no external flash |
| Wireless | Wi-Fi + Bluetooth classic/BLE | Wi-Fi + BLE 5 | **none** |
| USB | no native USB (UART bridge on the board) | USB OTG full speed (12 Mbit/s) | **USB 2.0 High Speed (480 Mbit/s)** |
| GPIO | about 34 pins | about 45 pins | **20 pinned out** |
| Programmable I/O | RMT, LEDC, I2S | RMT, LEDC, I2S, LCD/camera | **4 real RISC-V cores (BIO)** |
| Memory isolation | none (basic permissions) | basic permissions | **full MMU** |
| Security | secure boot, flash encryption, eFuses | same family | **hardware key slots, mesh, glitch sensors, one-way counters, open RTL** |
| Software | Arduino, ESP-IDF, MicroPython, Rust | same | Rust/Xous, baremetal Rust, experimental MicroPython; **no Arduino core** |
| Price | about $4-8 dev board | about $6-12 | about $12 (Dabao, per baochip.com) |

**Honest verdict.** Baochip wins on isolation, verifiability, USB speed, RAM, exact-timing I/O and secret-keeping. It loses on radios, GPIO count, floating point, ecosystem, community size and ease of use. As a daily driver for **non-wireless** projects it can get there; today the friction (flashing, toolchain, no libraries) is the gap, and that's exactly what tooling can fix.

## 6d. Is Rust "optimized"?

Partly a myth. Rust compiles through the same LLVM backend as C/C++ (clang), so speed is about the same as good C. It has no garbage collector and no hidden runtime, so it is predictable. The real win is **memory safety enforced by the compiler**: whole classes of bugs (buffer overflows, use-after-free) can't exist in safe code, which is why a security chip's OS is written in it. It does *not* automatically make programs smaller; with `std` and generics, Rust binaries can be larger than C. `no_std` Rust gets close to C sizes. The embedded ecosystem is younger than Arduino's.

## 6e. Why it isn't "just plug and flash" yet (from the source)

1. **The flash ceremony.** The board sits in the bootloader on every power-up (`bootwait` is Enabled; see PROCEDURE.md) and you copy UF2s then press PROG. *But the bootloader already has tools we haven't used:* a **serial `uf2` command** that writes firmware blocks over the USB serial port (with address and family checks that refuse to touch boot0/boot1), and a **`boot` command** that starts the app without PROG. An `esptool`-style flasher over serial is possible today; nobody has packaged it. **[from source; unverified on hardware]**
2. **Getting back into the bootloader from a running app.** The HAL has a `reboot()` that writes the same reset register as the bootloader's `reset` command; with `bootwait` enabled, a reset should land in boot1. The community MicroPython port exposes `machine.bootloader()`. **[unverified on hardware]**
3. **No beginner API.** There is no `digitalWrite`/`pinMode`, no library catalogue, no pin-conflict checker. Today you write Rust against the HAL or use the experimental MicroPython port.
4. **Setup pain.** On this machine we hit: rustup defaulting to MSVC, a Xous-specific Rust sysroot that must match the compiler version exactly, `pipx` hanging, an upstream `bio-loader` packaging bug, and a transient DNS failure. A `doctor` command would have saved hours.
5. **Developer mode is one-way.** Flashing your own image permanently marks the chip as a developer device. Good tooling should say so before it happens.

## 6f. Idea: Baochip tools + a ReRAM-native deniable store

**The steering-wheel principle:** keep the *interface* familiar (like PDDB, like `std::fs`, like Arduino), and swap the engine underneath for one designed for ReRAM.

- **Why not copy PDDB:** PDDB assumes flash: 4 KiB erase sectors, about 100k erase cycles, log-structured writes and garbage collection (`docs/flash.md`). ReRAM is written in place in small units, so most of that machinery is unnecessary.
- **ReRAM-native sketch** **[my design thinking, untested; needs a threat model and expert review]:**
  - Fill the whole store with random noise at setup, so used and unused space look the same.
  - Each secret layer ("basis") has a key derived from a passphrase; a key + a logical name pseudo-randomly picks the physical slot, so there is no index to find; every slot is authenticated-encrypted.
  - **Correction (found 7 Nov in xous-core issue #890):** my first sketch assumed in-place rewrites were nearly free. They are not. Bunnie quotes the standard ReRAM endurance as **about 10,000 read/write cycles** (the process-manual rating; "a bit higher in practice"), which is *lower* than the ~100k erase cycles Xous assumes for flash. So a hot value (a counter, an index) rewritten in place would wear out its cells fast. A ReRAM-native design has to be wear-aware (spread or log hot writes), so "no log, no wear handling" is wrong. Randomised placement helps for cold data only.
  - The known deniability hazard: writing layer A can overwrite hidden layer B's slots. PDDB solves this by making you mount all layers before writing; we'd need the same kind of rule, or enough redundancy.
- **Unknowns to check first:** exact write granularity (the blog says 32-byte pages), and whether a std app can map and write the free area safely. (Endurance is now known to be about 10k cycles; see the correction above.) Under Xous the free area is small (~300 KB slack in the app region); under baremetal it's about 3 MiB.
- **Context:** the community MicroPython port lists a LittleFS on ReRAM as "in progress, non-trivial", so storage on this chip is a recognised need.

**Proposed toolkit ("baokit", working name):**
- `bao doctor`: checks Rust toolchain, matching sysroot, zig, ports, board state.
- `bao flash`: serial `uf2` flasher + `boot` (drive-copy fallback), no PROG press.
- `bao monitor`, `bao run` (build + flash + monitor), `bao bootloader`.
- `bao pins`: checks a pin plan against `pins.csv` (reserved pins, voltage, conflicts).
- `bao bio`: build, simulate, load BIO programs in one command.
- `bao new`: project templates (blink, OLED, BIO audio, storage demo).
- A small Arduino-style Rust crate on top of the HAL, and the ReRAM store behind a PDDB-like API.

## 7. The "GPUs weren't designed for AI" question **[my inference]**

GPUs were built to draw pixels and became the engine of AI because they happened to do huge parallel math. By the same logic, what is this chip's accidental superpower?

My bet: **it combines three things that normally live on separate chips and never trust each other**:
1. **hardware-isolated code** (MMU),
2. **cycle-exact real-world I/O** (BIO),
3. **built-in cryptography and secret storage** (crypto engines, key slots).

A normal secure element can hold keys but can't watch pins with nanosecond timing. A normal microcontroller can watch pins but can't isolate code or hold secrets safely. This chip can do both in one die. So the thing nobody designed it for, but that fits: **a trustworthy referee between untrusted things and the physical world.** Concrete shapes: a notary that *signs real sensor measurements*; a verifiable guard sitting on a bus (USB, SPI) between two devices that don't trust each other; a tamper-evident black box. It is a hypothesis, not established; the only check is building something and seeing whether it holds.

## 8. What Bunnie values (from his blog and the project)

- **Verifiability over promises.** The whole pitch is "check it yourself": IRIS, published RTL, deterministic builds.
- **Honesty about limits.** He says "mostly open", discloses what's closed, and invites red-teaming of untested countermeasures. Overclaiming will hurt you with this judge.
- **The MMU argument.** A microcontroller-class chip with real memory isolation is his core bet; projects that need it are on-message.
- **Rust and Xous, built in the open.** He co-wrote Xous.
- **High-assurance users** (people facing serious surveillance risk), not just crypto speculators.
- **He already built deniable storage.** Xous's database is literally called **PDDB: Plausibly Deniable DataBase**, and its `vault2` app has a "deniable" basis. The Dabao image has **no PDDB** (it has no external flash).

## 9. Glossary
- **MCU:** microcontroller, the chip running the code. **SE:** secure element, a hardened secrets chip.
- **RISC-V:** an open instruction set (like ARM, but free). **VexRiscv / PicoRV32:** two open RISC-V CPU designs.
- **RTL:** the human-readable hardware design of a chip. **TRNG:** true random number generator.
- **UF2:** a file format you copy onto a USB drive to flash a board.
- **Fault injection:** deliberately glitching a chip (voltage, clock, laser) so it skips a security check.
- **Side channel:** leaking secrets through power use or timing.
