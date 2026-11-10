"""baoflash: flash a Dabao over the boot1 serial console (no drive copy, no PROG press).

Uses two commands that exist in boot1's console (xous-core/bao1x-boot/boot1/src/repl.rs):
  uf2 <base64 of one 512-byte UF2 block>   writes that block to ReRAM (boot1 checks family + address)
  boot                                     starts the app (what PROG does)

Usage:
  python baoflash.py apps.uf2 [more.uf2 ...] [--boot] [--dry-run] [--port COMx]

Safety (checked here AND by boot1):
  * only UF2 blocks of family 0xA7D76373 inside 0x60060000..0x603DA000 are sent
    (boot0/boot1 are never touched; boot1 also refuses anything outside that range)
  * refuses to run unless the board is in bootloader mode (USB PID 6196)
  * never sends lockdown / self_destruct / paranoid / altboot / bootwait
"""
import argparse
import base64
import re
import struct
import sys
import time

import serial
from serial.tools import list_ports

FAMILY = 0xA7D76373
LO, HI = 0x6006_0000, 0x6000_0000 + 0x3D_A000
M0, M1, MEND = 0x0A324655, 0x9E5D5157, 0x0AB16F30


def parse_uf2(path):
    data = open(path, "rb").read()
    if len(data) % 512:
        sys.exit(f"{path}: size {len(data)} is not a multiple of 512")
    blocks = []
    for i in range(0, len(data), 512):
        b = data[i:i + 512]
        m0, m1, flags, addr, size, no, total, fam = struct.unpack("<8I", b[:32])
        (mend,) = struct.unpack("<I", b[508:])
        if (m0, m1, mend) != (M0, M1, MEND):
            sys.exit(f"{path}: block {i // 512} has bad magic")
        if fam != FAMILY:
            sys.exit(f"{path}: block {i // 512} has family {fam:#x}, expected {FAMILY:#x}")
        if size > 476 or not (LO <= addr and addr + size <= HI):
            sys.exit(f"{path}: block {i // 512} address {addr:#x}+{size} outside {LO:#x}..{HI:#x}")
        blocks.append((addr, size, b))
    return blocks


def find_port(want_pid=0x6196, port=None):
    for p in list_ports.comports():
        if p.vid == 0x1D50 and p.pid in (0x6196, 0x6197):
            if p.pid != want_pid:
                return None, p.pid
            return port or p.device, p.pid
    return None, None


def read_for(s, seconds):
    end, out = time.time() + seconds, b""
    while time.time() < end:
        out += s.read(4096)
    return out.decode("utf-8", "replace")


def send_cmd(s, line, wait_for=None, timeout=3.0):
    s.write(line.encode() + b"\r")
    end, out = time.time() + timeout, ""
    while time.time() < end:
        # read(1) returns as soon as ONE byte arrives; read(4096) would sit out the whole timeout
        out += s.read(max(1, s.in_waiting)).decode("utf-8", "replace")
        if wait_for and wait_for in out:
            return out
    return out


PEEK_LINE = re.compile(r"([0-9a-f]{8}): ((?:[0-9a-f]{8} ?)+)")


def verify(s, blocks, progress):
    """Read every block back with boot1's `peek` and compare to the UF2 payload. Returns an error string or None.
    Needed because boot1 prints 'Wrote ...' even when the ReRAM write itself reported an error."""
    for n, (addr, size, raw) in enumerate(blocks, 1):
        want = raw[32:32 + size].ljust((size + 3) // 4 * 4, b"\0")
        nwords = len(want) // 4
        s.write(f"peek {addr:x} {nwords}\r".encode())
        words, text, end = {}, "", time.time() + 3
        while len(words) < nwords and time.time() < end:
            text += s.read(max(1, s.in_waiting)).decode("utf-8", "replace")
            if "not recognized" in text:
                return ("this boot1 build has no `peek` command (checked on v0.10.0-61), so readback is impossible; "
                        "verify by booting: the boot chain checks image signatures")
            for m in PEEK_LINE.finditer(text):
                base = int(m.group(1), 16)
                for i, w in enumerate(m.group(2).split()):
                    words[base + 4 * i] = int(w, 16)
        got = b"".join(words.get(addr + 4 * i, 0xDEADBEEF).to_bytes(4, "little") for i in range(nwords))
        if got != want:
            for i in range(nwords):
                if got[4 * i:4 * i + 4] != want[4 * i:4 * i + 4]:
                    return f"MISMATCH in block {n} at {addr + 4 * i:#x}: device {got[4*i:4*i+4].hex()} vs file {want[4*i:4*i+4].hex()}"
        progress(n)
    return None


ACK = re.compile(r"Wrote (\d+) to 0x([0-9a-f]+)")
BAD = ("Invalid write address", "Write error", "Decode error", "malformed", "invalid u2f")


def flash_pipelined(s, blocks, window, progress):
    """Keep up to `window` blocks in flight; every block must be acked with its own address, in order."""
    total, sent, acked, buf, last = len(blocks), 0, 0, "", time.time()
    while acked < total:
        while sent < total and sent - acked < window:
            raw = blocks[sent][2]
            s.write(b"uf2 " + base64.b64encode(raw) + b"\r")
            sent += 1
        chunk = s.read(max(1, s.in_waiting)).decode("utf-8", "replace")
        if chunk:
            buf += chunk
            last = time.time()
        for bad in BAD:
            if bad in buf:
                return f"device reported '{bad}' near block {acked + 1}: {buf[-200:]!r}"
        while True:
            m = ACK.search(buf)
            if not m:
                break
            if int(m.group(2), 16) != blocks[acked][0]:
                return f"ack for {m.group(2)} but expected {blocks[acked][0]:x} (block {acked + 1})"
            acked += 1
            buf = buf[m.end():]
            progress(acked)
        if time.time() - last > 5:
            return f"no response for 5 s at block {acked + 1}/{total} (sent {sent})"
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--boot", action="store_true", help="send 'boot' after flashing")
    ap.add_argument("--dry-run", action="store_true", help="parse and validate only")
    ap.add_argument("--port")
    ap.add_argument("--window", type=int, default=1,
                    help="blocks in flight. Keep at 1: boot1 merges queued lines into one command (tested: 8 and 16 stall)")
    ap.add_argument("--verify", action="store_true", help="read every block back with `peek` and compare")
    ap.add_argument("--verify-only", action="store_true", help="don't flash; only compare device memory to the file")
    a = ap.parse_args()

    blocks = []
    for f in a.files:
        b = parse_uf2(f)
        print(f"{f}: {len(b)} blocks, {b[0][0]:#x}..{b[-1][0] + b[-1][1]:#x} OK")
        blocks += b
    if a.dry_run:
        print("dry run: nothing sent")
        return 0

    port, pid = find_port(0x6196, a.port)
    if not port:
        print("Board is not in bootloader mode (PID 6196)." if pid else "No Dabao found.")
        print("Unplug and replug USB (bootwait is enabled, so it lands in the bootloader).")
        return 1
    s = serial.Serial(port, 1_000_000, timeout=0.02)
    read_for(s, 0.3)
    send_cmd(s, "localecho off", timeout=0.5)
    t0 = time.time()

    def progress(n):
        if n % 100 == 0 or n == len(blocks):
            print(f"\r{n}/{len(blocks)} blocks  {time.time() - t0:.1f}s", end="", flush=True)

    if not a.verify_only:
        err = flash_pipelined(s, blocks, max(1, a.window), progress)
        if err:
            print(f"\nFAILED: {err}")
            return 2
        dt = time.time() - t0
        kb = sum(sz for _, sz, _ in blocks) / 1024
        print(f"\nflashed {len(blocks)} blocks ({kb:.0f} KiB) in {dt:.1f}s = {kb / dt:.1f} KiB/s, window={a.window}")
    if a.verify or a.verify_only:
        t1 = time.time()
        err = verify(s, blocks, lambda n: print(f"\rverify {n}/{len(blocks)}", end="", flush=True) if n % 100 == 0 or n == len(blocks) else None)
        if err:
            print(f"\nVERIFY FAILED: {err}")
            return 3
        print(f"\nverified {len(blocks)} blocks byte-for-byte in {time.time() - t1:.1f}s")
    if a.boot:
        s.write(b"boot\r")
        print("sent: boot")
    s.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
