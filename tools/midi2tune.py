"""midi2tune: turn a MIDI file into a ChocoBao tune (Rust list of (frequency in Hz, duration in ms)).

  python tools/midi2tune.py song.mid                      list the tracks
  python tools/midi2tune.py song.mid --track 1 --name BLINDING_LIGHTS > out.rs
  options: --transpose N (semitones, default 0), --bars A-B (only those bars), --min-hz 400 --max-hz 2600

The buzzer is one voice, so the melody is made monophonic: when notes overlap, the highest one wins.
Notes below --min-hz are moved up an octave (a small buzzer is quiet below ~500 Hz), above --max-hz down.
Download a MIDI from the sheet's source (for example MuseScore: Download -> MIDI), save it, run this, and paste the output
into chocobao/src/music.rs (or tell me the file path and I will do it).
"""
import argparse
import struct
import sys


def vlq(d, i):
    v = 0
    while True:
        b = d[i]
        i += 1
        v = (v << 7) | (b & 0x7F)
        if not b & 0x80:
            return v, i


def parse(path):
    d = open(path, 'rb').read()
    assert d[:4] == b'MThd', 'not a MIDI file'
    _, ntrk, div = struct.unpack('>HHH', d[8:14])
    i = 14
    tracks, tempo_map = [], [(0, 500000)]
    for _ in range(ntrk):
        assert d[i:i + 4] == b'MTrk'
        ln = struct.unpack('>I', d[i + 4:i + 8])[0]
        j, end, t, run = i + 8, i + 8 + ln, 0, 0
        notes, open_n = [], {}
        while j < end:
            dt, j = vlq(d, j)
            t += dt
            st = d[j]
            if st == 0xFF:
                typ = d[j + 1]
                l, j2 = vlq(d, j + 2)
                if typ == 0x51:
                    tempo_map.append((t, int.from_bytes(d[j2:j2 + 3], 'big')))
                j = j2 + l
                continue
            if st in (0xF0, 0xF7):
                l, j = vlq(d, j + 1)
                j += l
                continue
            if st & 0x80:
                run = st
                j += 1
            ch_type = run & 0xF0
            if ch_type in (0x80, 0x90):
                n, v = d[j], d[j + 1]
                j += 2
                if ch_type == 0x90 and v > 0:
                    open_n[(run & 15, n)] = t
                else:
                    s = open_n.pop((run & 15, n), None)
                    if s is not None and (run & 15) != 9:
                        notes.append((s, t, n))
            elif ch_type in (0xA0, 0xB0, 0xE0):
                j += 2
            else:
                j += 1
        tracks.append(sorted(notes))
        i = end
    return div, sorted(tempo_map), tracks


def ticks_to_ms(t, div, tempo_map):
    ms, last_t, tempo = 0.0, 0, 500000
    for tt, tp in tempo_map:
        if tt >= t:
            break
        ms += (tt - last_t) * tempo / div / 1000
        last_t, tempo = tt, tp
    return ms + (t - last_t) * tempo / div / 1000


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('midi')
    ap.add_argument('--track', type=int)
    ap.add_argument('--name', default='TUNE')
    ap.add_argument('--transpose', type=int, default=0)
    ap.add_argument('--bars', help='first-last bar, e.g. 1-16 (assumes 4/4)')
    ap.add_argument('--min-hz', type=float, default=400)
    ap.add_argument('--max-hz', type=float, default=2600)
    a = ap.parse_args()
    div, tempo_map, tracks = parse(a.midi)
    if a.track is None:
        for k, tr in enumerate(tracks):
            if tr:
                print(f'track {k}: {len(tr)} notes, pitches {min(n for _, _, n in tr)}-{max(n for _, _, n in tr)}', file=sys.stderr)
        print('choose one with --track N (the melody is usually the track with the higher pitches)', file=sys.stderr)
        return
    notes = tracks[a.track]
    if a.bars:
        lo, hi = (int(x) for x in a.bars.split('-'))
        notes = [n for n in notes if (lo - 1) * 4 * div <= n[0] < hi * 4 * div]
    # monophonic: highest note wins; each note ends when the next one starts
    notes.sort()
    mono, i = [], 0
    while i < len(notes):
        s, e, n = notes[i]
        best = (s, e, n)
        j = i + 1
        while j < len(notes) and notes[j][0] < e:
            if notes[j][0] == s and notes[j][2] > best[2]:
                best = notes[j]
            j += 1
        mono.append(best)
        i = j if j > i + 1 else i + 1
    out, prev_end = [], None
    for s, e, n in mono:
        n += a.transpose
        hz = 440.0 * 2 ** ((n - 69) / 12)
        while hz < a.min_hz:
            hz *= 2
        while hz > a.max_hz:
            hz /= 2
        t0 = ticks_to_ms(s, div, tempo_map)
        t1 = ticks_to_ms(e, div, tempo_map)
        if prev_end is not None and t0 - prev_end > 30:
            out.append((0, round(t0 - prev_end)))
        out.append((round(hz), max(40, round(t1 - t0))))
        prev_end = t1
    print(f'pub const {a.name}: &[Note] = &[')
    line = '   '
    for hz, ms in out:
        item = f' ({hz}, {ms}),'
        if len(line) + len(item) > 110:
            print(line)
            line = '   '
        line += item
    print(line)
    print('];')
    print(f'// {len(out)} notes, {sum(m for _, m in out) / 1000:.1f} s', file=sys.stderr)


main()
