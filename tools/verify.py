"""verify.py: check a ChocoBao Notary proof on the PC. No extra packages needed (pure Python Ed25519, RFC 8032).

On the device: Lab -> Notary, press UP/DOWN/LEFT/RIGHT to create signed events, click a text box on the PC, HOLD OK.
The device types one line:   CHOCONOTARY1|<count>|<ms since the app started>|<event>|<signature hex>|<public key hex>
Save one or more such lines in a text file and run:

    python tools/verify.py proofs.txt                 verify every line
    python tools/verify.py proofs.txt --pub <hex>     also require this public key (write the key down at the first proof)

Checks: the Ed25519 signature is valid for the first four fields, the counters never go backwards and every event is
signed by the same key. What it proves: the event text was signed by whoever holds that key. It does NOT prove the
timestamp is true (the chip's clock is its own) or that a clone with another key is not the original: pin the public key.
"""
import hashlib
import sys

P = 2 ** 255 - 19
L = 2 ** 252 + 27742317777372353535851937790883648493
D = -121665 * pow(121666, P - 2, P) % P
I = pow(2, (P - 1) // 4, P)


def inv(x):
    return pow(x, P - 2, P)


def recover_x(y, sign):
    xx = (y * y - 1) * inv(D * y * y + 1) % P
    x = pow(xx, (P + 3) // 8, P)
    if (x * x - xx) % P != 0:
        x = x * I % P
    if (x * x - xx) % P != 0:
        return None
    if x & 1 != sign:
        x = P - x
    return x


BY = 4 * inv(5) % P
BX = recover_x(BY, 0)
B = (BX, BY, 1, BX * BY % P)


def add(p, q):
    a = (p[1] - p[0]) * (q[1] - q[0]) % P
    b = (p[1] + p[0]) * (q[1] + q[0]) % P
    c = 2 * p[3] * q[3] * D % P
    d = 2 * p[2] * q[2] % P
    e, f, g, h = b - a, d - c, d + c, b + a
    return (e * f % P, g * h % P, f * g % P, e * h % P)


def mul(s, p):
    q = (0, 1, 1, 0)
    while s:
        if s & 1:
            q = add(q, p)
        p = add(p, p)
        s >>= 1
    return q


def encode(p):
    zi = inv(p[2])
    x, y = p[0] * zi % P, p[1] * zi % P
    return (y | ((x & 1) << 255)).to_bytes(32, "little")


def decode(b):
    y = int.from_bytes(b, "little")
    sign = y >> 255
    y &= (1 << 255) - 1
    x = recover_x(y, sign)
    return None if x is None else (x, y, 1, x * y % P)


def verify(pub, msg, sig):
    if len(sig) != 64 or len(pub) != 32:
        return False
    a = decode(pub)
    r = decode(sig[:32])
    s = int.from_bytes(sig[32:], "little")
    if a is None or r is None or s >= L:
        return False
    h = int.from_bytes(hashlib.sha512(sig[:32] + pub + msg).digest(), "little") % L
    return encode(mul(s, B)) == encode(add(r, mul(h, a)))


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    want_pub = None
    if "--pub" in sys.argv:
        want_pub = sys.argv[sys.argv.index("--pub") + 1].lower()
    ok_all, last_n, key = True, 0, None
    for ln in open(sys.argv[1], encoding="utf-8"):
        ln = ln.strip()
        if not ln:
            continue
        f = ln.split("|")
        if len(f) != 6 or f[0] != "CHOCONOTARY1":
            print("SKIP (not a proof line):", ln[:60])
            continue
        msg = "|".join(f[:4]).encode()
        good = verify(bytes.fromhex(f[5]), msg, bytes.fromhex(f[4]))
        n = int(f[1])
        if key is None:
            key = f[5]
        notes = []
        if f[5] != key:
            notes.append("DIFFERENT KEY")
        if want_pub and f[5].lower() != want_pub:
            notes.append("NOT THE EXPECTED KEY")
        if n <= last_n:
            notes.append("COUNTER NOT INCREASING")
        last_n = max(last_n, n)
        status = "VALID  " if good and not notes else "INVALID"
        ok_all &= good and not notes
        print(f"{status} #{n} {f[3]} at {f[2]} ms {' '.join(notes)}")
    print("ALL GOOD" if ok_all else "PROBLEMS FOUND")
    return 0 if ok_all else 1


if __name__ == "__main__":
    sys.exit(main())
