"""ChocoBao desktop helper: one-click Dice, no serial monitor needed.

Double-click ChocoBao.cmd (project root) or run:   python tools/chocobao_gui.py
Test without a window:                              python tools/chocobao_gui.py --cli "dice pw 16 alnum"

It finds the board by its USB ID (VID 1D50, PID 6197), opens the console at 1,000,000 baud 8N1,
sends the `dice ...` commands for you, and shows the answer. Nothing is stored anywhere.
Only ONE program can use the serial port at a time: close any serial monitor first.
"""
import re
import sys
import time

import serial
from serial.tools import list_ports

BAUD = 1_000_000
PROMPT = "[console]"
CHAR_DELAY = 0.01  # seconds between typed characters; 0 corrupts commands (see README)


def find_board():
    """Returns (port, state) with state 'run', 'boot' or None."""
    for p in list_ports.comports():
        if p.vid == 0x1D50 and p.pid == 0x6197:
            return p.device, "run"
        if p.vid == 0x1D50 and p.pid == 0x6196:
            return p.device, "boot"
    return None, None


def parse_answer(cmd, raw):
    """Recognise the answer to `cmd` inside the (garbled, echo-polluted) console text.

    The console echoes typed characters unreliably (repeats, merged lines), but the real answer
    always appears intact somewhere, so we match known formats and check them against the request.
    Returns a list of lines, or None if the answer is not complete yet."""
    t = raw.replace("\r", "")
    words = cmd.split()
    typed = words[-1] == "type"
    if typed:
        m = re.findall(r"(sent \d+ keystrokes[^\n]*|can't type[^\n]*)", t)
        info = re.findall(r"(\d+ (?:chars from a \d+-symbol set|words from a \d+-word list) = [\d.]+ bits)\n", t)
        if m:
            return [info[-1] if info else "typed", m[-1]]
        return None
    if words[:2] == ["dice", "pw"]:
        n = int(words[2]) if len(words) > 2 and words[2].isdigit() else 20
        for info, secret in reversed(re.findall(r"(?m)^(\d+ chars from a \d+-symbol set = [\d.]+ bits)\n(\S+)$", t)):
            if len(secret) == n:
                return [info, secret]
        return None
    if words[:2] == ["dice", "words"]:
        n = int(words[2]) if len(words) > 2 and words[2].isdigit() else 6
        for info, secret in reversed(re.findall(r"(?m)^(\d+ words from a \d+-word list = [\d.]+ bits)\n([a-z]+(?:-[a-z]+)*)$", t)):
            if secret.count("-") + 1 == n:
                return [info, secret]
        return None
    if words[:2] == ["dice", "roll"]:
        spec = words[2] if len(words) > 2 else "1d6"
        m = re.findall(r"(?m)^" + re.escape(spec) + r": (\d+(?: \d+)*)(?:  \(sum (\d+)\))?$", t)
        if m:
            rolls, total = m[-1]
            return [f"{spec}: {rolls}" + (f"  (sum {total})" if total else ""), rolls if not total else total]
        return None
    return None


class Board:
    def __init__(self):
        self.s = None

    def connect(self):
        port, state = find_board()
        if state is None:
            raise RuntimeError("No ChocoBao found. Plug it in, then press PROG (hold about 1 s).")
        if state == "boot":
            raise RuntimeError("Board is in bootloader mode. Press PROG (hold about 1 s) to start the app.")
        try:
            self.s = serial.Serial(port, BAUD, timeout=0.05)
        except serial.SerialException as e:
            raise RuntimeError(f"Can't open {port}: another program (a serial monitor?) is using it. Close it. ({e})")
        self._read(0.2)
        self.s.write(b"\r")
        self._read(0.6)
        return port

    def close(self):
        if self.s:
            try:
                self.s.close()
            except Exception:
                pass
            self.s = None

    def _read(self, seconds, until=None):
        end, out = time.time() + seconds, ""
        while time.time() < end:
            out += self.s.read(max(1, self.s.in_waiting)).decode("utf-8", "replace")
            if until and until(out):
                break
        return out

    def run(self, cmd, timeout=6.0):
        """Send one console command; return the answer as a list of lines."""
        if self.s is None:
            self.connect()
        self._quiet(0.4)  # let any earlier output finish before we send
        self.s.reset_input_buffer()
        # The board's console corrupts characters that arrive in a burst (found 8 Nov), so type slowly.
        for ch in cmd.encode() + b"\r":
            self.s.write(bytes([ch]))
            time.sleep(CHAR_DELAY)
        typed = cmd.endswith(" type")
        t0 = last = time.time()
        raw = ""
        while time.time() - t0 < (12.0 if typed else timeout):
            chunk = self.s.read(max(1, self.s.in_waiting)).decode("utf-8", "replace")
            if chunk:
                raw += chunk
                last = time.time()
                found = parse_answer(cmd, raw)
                if found:
                    return found
            elif cmd.strip() == "dice" and raw and time.time() - last > 0.8:
                break  # the help text: take whatever arrived
        if cmd.strip() == "dice" and raw:
            return [ln.strip() for ln in raw.replace("\r", "").split("\n") if ln.strip() and PROMPT not in ln]
        raise RuntimeError("No usable answer from the board (is the ChocoBao app running?). Heard: " + repr(raw[-120:]))

    def _quiet(self, seconds):
        """Read and discard until nothing has arrived for `seconds`."""
        last = time.time()
        while time.time() - last < seconds:
            if self.s.read(max(1, self.s.in_waiting)):
                last = time.time()


def run_cmd(board, cmd, timeout=6.0):
    lines = board.run(cmd, timeout)
    if not lines:
        raise RuntimeError("Empty answer.")
    return lines


def main_gui():
    import tkinter as tk
    from tkinter import ttk

    board = Board()
    root = tk.Tk()
    root.title("ChocoBao")
    root.geometry("520x470")
    pad = {"padx": 8, "pady": 4}

    status = tk.StringVar(value="looking for the board...")
    info = tk.StringVar(value="")
    result = tk.StringVar(value="")

    ttk.Label(root, textvariable=status, font=("Segoe UI", 10, "bold")).pack(anchor="w", **pad)

    def show(lines, typed=False):
        info.set(lines[0] if lines else "")
        result.set("" if typed else (lines[1] if len(lines) > 1 else ""))
        if typed:
            result.set("(typed into your PC, not shown here)")

    def do(cmd, typed=False):
        try:
            if typed:
                status.set("click the target field NOW, typing in 3 seconds...")
                root.update()
            lines = run_cmd(board, cmd, timeout=9.0 if typed else 5.0)
            show(lines, typed)
            status.set(f"connected: {board.s.port}")
        except Exception as e:
            board.close()
            status.set(str(e))

    pw = ttk.LabelFrame(root, text="Password")
    pw.pack(fill="x", **pad)
    plen = tk.StringVar(value="20")
    pset = tk.StringVar(value="full")
    ttk.Label(pw, text="length").grid(row=0, column=0, **pad)
    ttk.Spinbox(pw, from_=4, to=128, textvariable=plen, width=5).grid(row=0, column=1, **pad)
    ttk.Label(pw, text="characters").grid(row=0, column=2, **pad)
    ttk.Combobox(pw, textvariable=pset, values=["full", "alnum", "lower", "digits", "hex"], width=8, state="readonly").grid(row=0, column=3, **pad)
    ttk.Button(pw, text="Generate", command=lambda: do(f"dice pw {plen.get()} {pset.get()}")).grid(row=0, column=4, **pad)
    ttk.Button(pw, text="Type into PC", command=lambda: do(f"dice pw {plen.get()} {pset.get()} type", True)).grid(row=0, column=5, **pad)

    pp = ttk.LabelFrame(root, text="Passphrase (BIP-39 words)")
    pp.pack(fill="x", **pad)
    nw = tk.StringVar(value="6")
    ttk.Label(pp, text="words").grid(row=0, column=0, **pad)
    ttk.Spinbox(pp, from_=1, to=24, textvariable=nw, width=5).grid(row=0, column=1, **pad)
    ttk.Button(pp, text="Generate", command=lambda: do(f"dice words {nw.get()}")).grid(row=0, column=2, **pad)
    ttk.Button(pp, text="Type into PC", command=lambda: do(f"dice words {nw.get()} type", True)).grid(row=0, column=3, **pad)

    dc = ttk.LabelFrame(root, text="Dice")
    dc.pack(fill="x", **pad)
    spec = tk.StringVar(value="2d6")
    ttk.Entry(dc, textvariable=spec, width=8).grid(row=0, column=0, **pad)
    ttk.Button(dc, text="Roll", command=lambda: do(f"dice roll {spec.get()}")).grid(row=0, column=1, **pad)

    ttk.Label(root, textvariable=info, wraplength=480).pack(anchor="w", **pad)
    out = ttk.Entry(root, textvariable=result, font=("Consolas", 13), state="readonly")
    out.pack(fill="x", **pad)

    def copy():
        root.clipboard_clear()
        root.clipboard_append(result.get())
        info.set("copied (Windows clipboard history may keep it; clear it if that matters)")

    ttk.Button(root, text="Copy", command=copy).pack(anchor="w", **pad)
    ttk.Label(root, text="Random numbers come from the chip's hardware TRNG. Nothing is stored on the board or in this window.\n'Type into PC' assumes a US keyboard layout.", wraplength=490, foreground="#555").pack(anchor="w", **pad)

    def poll():
        if board.s is None:
            port, state = find_board()
            if state == "run":
                status.set(f"board found on {port} (click a button)")
            elif state == "boot":
                status.set("board is in bootloader mode: press PROG (hold about 1 s)")
            else:
                status.set("no board: plug it in, then press PROG (hold about 1 s)")
        root.after(1500, poll)

    poll()
    root.protocol("WM_DELETE_WINDOW", lambda: (board.close(), root.destroy()))
    root.mainloop()


if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "--cli":
        b = Board()
        try:
            print("port:", b.connect())
            for c in sys.argv[2:]:
                print(f"> {c}")
                for ln in run_cmd(b, c, 9.0 if c.endswith(" type") else 5.0):
                    print("  " + ln)
        except Exception as e:
            print("ERROR:", e)
            sys.exit(1)
        finally:
            b.close()
    else:
        main_gui()
