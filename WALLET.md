# ChocoVault: hardware wallet plan (design only, nothing is built yet)

Status: proposal. A wallet holds money, so it is built in stages and each stage is tested before the next.

## What the chip gives us (checked in this repo)
| Feature | Use in a wallet | Status |
|---|---|---|
| TRNG | generate the seed (BIP-39 entropy) | works, used by Seed words |
| SHA-512 / HMAC / PBKDF2 (crates `sha2`, `hmac`, `pbkdf2`) | BIP-39 seed, SLIP-0010 key derivation | crates available offline |
| Ed25519 (`ed25519-dalek-compat`, hardware engine possible) | sign transactions for Ed25519 chains (Solana, Stellar, Cardano-style) | used by Notary |
| secp256k1 (Bitcoin, Ethereum) | needs the `k256` crate, **not in the offline registry**; needs network once | open |
| ReRAM data slots (0x603E_0000, 64 KiB, ACL) | keep the encrypted seed across power cycles | untested, ~10k writes |
| Boot signature chain, debug fused off | attacker cannot swap in different firmware | already true on this board |
| Glitch/tamper sensors | not used by us on purpose (their registers sit next to one-way counters) | no |

## Threat model (what "secure" can honestly mean here)
- Protects against: malware on the PC (keys never leave the device, every signature is confirmed on the OLED), a lost device (PIN with limited tries), a copied flash dump (seed encrypted with a PIN-derived key).
- Does **not** protect against: a determined lab attacker with the board (side channels, fault injection), or a bug in our own code. Xous is a single-tenant trusted code base: the wallet app and the games would share one address space, so a real wallet build must ship **without** the games.
- "Highest level of security" is not something we can claim. The goal is a clear, honest, auditable design.

## Stages
1. **Vault (RAM only):** TRNG entropy, 24 BIP-39 words shown once on the OLED for backup, SLIP-0010 Ed25519 key, PIN screen, address shown as QR/hex. Nothing written to ReRAM. Safe to demo.
2. **Signing over USB:** a small PC tool sends a transaction, the OLED shows amount and address, hold OK to sign, the signature goes back. Reuses the Notary code and its `verify.py`.
3. **Persistence:** seed encrypted with a key from PBKDF2(PIN) + device secret, stored in a rotating ReRAM slot, retry counter in RAM+slot (never in a one-way counter), wipe after 10 wrong PINs.
4. **Hardening:** constant-time compares, zeroize everything, no secrets in logs, disable the console commands in wallet mode.

## Decisions needed from the owner
1. Which chain first: Ed25519 (Solana) works offline today; Bitcoin/Ethereum need one network fetch of `k256`.
2. Is persisting the seed in ReRAM wanted (stage 3), given the write wear and that it is hard to undo, or should stage 1 and 2 stay RAM-only for the hackathon?
3. Ship the wallet as a separate build without games (recommended).
