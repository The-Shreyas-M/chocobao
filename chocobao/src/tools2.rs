// tools2.rs - small self-contained apps: Calculator, Unit converter, Base converter, Metronome, Habit counter,
// Morse trainer, Text presets, Flashlight, Chord synth, Button scope, TRNG health, AM beacon. One `ToolState` holds
// whichever one is open; app.rs only forwards buttons / time to it and applies the returned `Out`.
use std::time::{Duration, Instant};

use bao1x_hal_service::trng::Trng;

use crate::controls::Btn;
use crate::oled::Canvas;
use crate::rng::below;
use crate::sound::{Note, A5, B6, C6, E6, REST};

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Calc,
    Units,
    Base,
    Metro,
    Habit,
    Morse,
    Typer,
    Flash,
    Synth,
    Scope,
    Health,
    Beacon,
    Notary,
}

pub const COUNT: usize = 13;
const KINDS: [Kind; COUNT] = [
    Kind::Calc,
    Kind::Units,
    Kind::Base,
    Kind::Metro,
    Kind::Habit,
    Kind::Morse,
    Kind::Typer,
    Kind::Flash,
    Kind::Synth,
    Kind::Scope,
    Kind::Health,
    Kind::Beacon,
    Kind::Notary,
];

pub fn kind_index(k: Kind) -> usize { KINDS.iter().position(|x| *x == k).unwrap_or(0) }
pub fn kind_of(i: usize) -> Kind { KINDS[i % COUNT] }

/// (title, four help lines) for the first-open screen
pub fn help(k: Kind) -> (&'static str, [&'static str; 4]) {
    match k {
        Kind::Calc => ("Calculator", ["<> ^v pick a key", "OK = press the key", "= shows the answer", "C clears, < deletes"]),
        Kind::Units => ("Unit converter", ["^v  pick conversion", "<>  change the value", "OK  step 0.1/1/10/100", ""]),
        Kind::Base => ("Base converter", ["^v +1 / -1", "<> +16 / -16", "OK  step x1 x16 x256", "shows dec hex bin"]),
        Kind::Metro => ("Metronome", ["^v tempo +/- 5", "<> beats per bar", "OK start / stop", "first beat chirps"]),
        Kind::Habit => ("Habit counter", ["^v pick a habit", "OK +1   LEFT -1", "RIGHT = reset it", "kept until power off"]),
        Kind::Morse => ("Morse trainer", ["<> pick a letter", "OK plays it", "UP = quiz: hidden", "DOWN = reveal it"]),
        Kind::Typer => ("Text presets", ["^v pick a text", "HOLD OK types it into", "the PC (click a box", "first). Edit tools2.rs"]),
        Kind::Flash => ("Flashlight", ["^v brightness", "OK = strobe on/off", "BACK exits", ""]),
        Kind::Synth => ("Chord synth", ["<> root note", "^v chord type", "OK plays 4 voices", "shared on one pin"]),
        Kind::Scope => ("Button scope", ["shows the last 2 s of", "all six button pins", "press buttons to see", "pulses (CPU sampled)"]),
        Kind::Health => ("TRNG health", ["live random bits from", "the hardware TRNG with", "simple quality tests", "OK = restart counts"]),
        Kind::Notary => ("Notary", ["^v<> press = an event", "each one is signed with", "Ed25519 (key in RAM)", "HOLD OK types the proof"]),
        Kind::Beacon => ("AM beacon", ["OK = start / stop", "keys a 250 kHz carrier", "in morse (SHREYAS).", "AM radio near? try it"]),
    }
}

/// What the app must do after a call.
#[derive(Default)]
pub struct Out {
    pub redraw: bool,
    pub play: Option<Vec<Note>>,
    pub tune: Option<Option<&'static [Note]>>,
    pub typ: Option<String>,
    pub toast: Option<String>,
}

impl Out {
    fn redraw() -> Out { Out { redraw: true, ..Default::default() } }
}

// ------------------------------------------------------------------ calculator
const CALC_KEYS: [[&str; 6]; 3] = [["7", "8", "9", "+", "-", "C"], ["4", "5", "6", "*", "/", "<"], ["1", "2", "3", "0", ".", "="]];

fn calc_eval(s: &str) -> Option<f64> {
    // numbers and + - * / with the usual precedence, leading minus allowed
    let mut nums: Vec<f64> = Vec::new();
    let mut ops: Vec<char> = Vec::new();
    let mut cur = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_digit() || ch == '.' || (ch == '-' && (i == 0 || "+-*/".contains(s.chars().nth(i - 1).unwrap_or(' ')))) {
            cur.push(ch);
        } else if "+-*/".contains(ch) {
            nums.push(cur.parse().ok()?);
            cur.clear();
            ops.push(ch);
        } else {
            return None;
        }
    }
    nums.push(cur.parse().ok()?);
    // * and / first
    let (mut n2, mut o2): (Vec<f64>, Vec<char>) = (vec![nums[0]], Vec::new());
    for (k, &op) in ops.iter().enumerate() {
        let b = nums[k + 1];
        match op {
            '*' => *n2.last_mut()? *= b,
            '/' => {
                if b == 0.0 {
                    return None;
                }
                *n2.last_mut()? /= b
            }
            _ => {
                n2.push(b);
                o2.push(op);
            }
        }
    }
    let mut r = n2[0];
    for (k, &op) in o2.iter().enumerate() {
        if op == '+' {
            r += n2[k + 1];
        } else {
            r -= n2[k + 1];
        }
    }
    Some(r)
}

fn fmt_num(v: f64) -> String {
    if v.abs() >= 1e9 || (v != 0.0 && v.abs() < 1e-4) {
        return format!("{:e}", v);
    }
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-" { "0".into() } else { s }
}

// ------------------------------------------------------------------ units
const UNITS: [(&str, &str, &str, f64, f64); 10] = [
    // name, from, to, factor, offset (result = v*factor + offset)
    ("km to miles", "km", "mi", 0.621371, 0.0),
    ("miles to km", "mi", "km", 1.609344, 0.0),
    ("kg to pounds", "kg", "lb", 2.204623, 0.0),
    ("pounds to kg", "lb", "kg", 0.453592, 0.0),
    ("C to F", "C", "F", 1.8, 32.0),
    ("F to C", "F", "C", 0.555556, -17.7778),
    ("cm to inch", "cm", "in", 0.393701, 0.0),
    ("inch to cm", "in", "cm", 2.54, 0.0),
    ("litre to gallon", "L", "gal", 0.264172, 0.0),
    ("gallon to litre", "gal", "L", 3.785412, 0.0),
];
const STEPS: [f64; 5] = [0.1, 1.0, 10.0, 100.0, 1000.0];

// ------------------------------------------------------------------ morse
const MORSE: [&str; 36] = [
    ".-", "-...", "-.-.", "-..", ".", "..-.", "--.", "....", "..", ".---", "-.-", ".-..", "--", "-.", "---", ".--.", "--.-", ".-.",
    "...", "-", "..-", "...-", ".--", "-..-", "-.--", "--..", "-----", ".----", "..---", "...--", "....-", ".....", "-....", "--...",
    "---..", "----.",
];

fn morse_char(i: usize) -> char {
    if i < 26 { (b'A' + i as u8) as char } else { (b'0' + (i - 26) as u8) as char }
}

fn morse_notes(code: &str, dot: u32) -> Vec<Note> {
    let mut v = Vec::new();
    for ch in code.chars() {
        v.push((A5, if ch == '.' { dot } else { dot * 3 }));
        v.push((REST, dot));
    }
    v
}

// ------------------------------------------------------------------ text presets (edit these lines)
const PRESETS: [&str; 5] = ["hello@example.com", "Best regards, Shreyas", "Thanks! I will check it today.", "+91 00000 00000", "My address goes here"];

const HABITS: [&str; 4] = ["Water", "Walk", "Read", "Code"];
const CHORDS: [(&str, [i32; 3]); 6] = [("major", [4, 7, 12]), ("minor", [3, 7, 12]), ("7th", [4, 7, 10]), ("dim", [3, 6, 9]), ("sus4", [5, 7, 12]), ("power", [7, 12, 19])];
const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

fn midi_hz(m: i32) -> u32 { (440.0 * 2f64.powf((m as f64 - 69.0) / 12.0)) as u32 }

pub struct ToolState {
    pub kind: Kind,
    // shared small fields
    sel: i32,
    sub: i32,
    step: usize,
    val: f64,
    flag: bool,
    text: String,
    res: String,
    hist: Vec<u8>,
    counts: [i32; 4],
    bits: Vec<u32>,
    ones: u64,
    total: u64,
    runs: u64,
    longest: u32,
    last_bit: u8,
    cur_run: u32,
    next_at: Instant,
    beat: u32,
    t0: Instant,
    notary: Option<Box<Notary>>,
}

/// The signing key lives only here, in RAM, and is never drawn or typed; only signatures and the public key leave.
struct Notary {
    seed: [u8; 32],
    n: u32,
    msg: String,
    sig_hex: String,
    pub_hex: String,
}

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{:02x}", x)).collect() }

impl Notary {
    fn sign(&mut self, evt: &str) {
        use ed25519_dalek::{Signer, SigningKey};
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let ms = START.get_or_init(Instant::now).elapsed().as_millis();
        self.n += 1;
        self.msg = format!("CHOCONOTARY1|{}|{}|{}", self.n, ms, evt);
        let sk = SigningKey::from_bytes(&self.seed);
        let sig = sk.sign(self.msg.as_bytes());
        self.sig_hex = hex(&sig.to_bytes());
        self.pub_hex = hex(&sk.verifying_key().to_bytes());
    }
}

impl ToolState {
    pub fn new(kind: Kind) -> ToolState {
        let mut s = ToolState {
            kind,
            sel: 0,
            sub: 0,
            step: 1,
            val: 0.0,
            flag: false,
            text: String::new(),
            res: String::new(),
            hist: Vec::new(),
            counts: [0; 4],
            bits: Vec::new(),
            ones: 0,
            total: 0,
            runs: 0,
            longest: 0,
            last_bit: 2,
            cur_run: 0,
            next_at: Instant::now(),
            beat: 0,
            t0: Instant::now(),
            notary: None,
        };
        if kind == Kind::Metro {
            s.val = 100.0;
            s.sub = 4; // beats per bar
        }
        if kind == Kind::Flash {
            s.sel = 2;
        }
        if kind == Kind::Units {
            s.val = 10.0;
        }
        s
    }

    /// Where BACK leads: (menu index in the Tools menu, ...) is decided by app.rs through `menu_slot`.
    pub fn menu_slot(&self) -> (u8, usize) {
        // (0 = Tools, 1 = Music, 2 = Lab), item index
        match self.kind {
            Kind::Calc => (0, 10),
            Kind::Units => (0, 11),
            Kind::Base => (0, 12),
            Kind::Metro => (1, 3),
            Kind::Habit => (0, 13),
            Kind::Morse => (0, 14),
            Kind::Typer => (0, 15),
            Kind::Flash => (0, 16),
            Kind::Synth => (1, 2),
            Kind::Scope => (2, 2),
            Kind::Health => (2, 3),
            Kind::Beacon => (2, 4),
            Kind::Notary => (2, 4),
        }
    }

    pub fn press(&mut self, b: Btn, trng: &Trng) -> Out {
        let mut o = Out::redraw();
        match self.kind {
            Kind::Calc => match b {
                Btn::Left => self.sel = (self.sel + 17) % 18,
                Btn::Right => self.sel = (self.sel + 1) % 18,
                Btn::Up => self.sel = (self.sel + 12) % 18,
                Btn::Down => self.sel = (self.sel + 6) % 18,
                Btn::Ok => {
                    let key = CALC_KEYS[(self.sel / 6) as usize][(self.sel % 6) as usize];
                    match key {
                        "C" => {
                            self.text.clear();
                            self.res.clear();
                        }
                        "<" => {
                            self.text.pop();
                        }
                        "=" => {
                            self.res = match calc_eval(&self.text) {
                                Some(v) => fmt_num(v),
                                None => "error".into(),
                            };
                        }
                        k => {
                            if self.text.len() < 20 {
                                self.text.push_str(k);
                            }
                        }
                    }
                    o.play = Some(vec![(1800, 12)]);
                }
                _ => o.redraw = false,
            },
            Kind::Units => match b {
                Btn::Up => self.sel = (self.sel + UNITS.len() as i32 - 1) % UNITS.len() as i32,
                Btn::Down => self.sel = (self.sel + 1) % UNITS.len() as i32,
                Btn::Left => self.val -= STEPS[self.step],
                Btn::Right => self.val += STEPS[self.step],
                Btn::Ok => self.step = (self.step + 1) % STEPS.len(),
                _ => o.redraw = false,
            },
            Kind::Base => {
                let m = [1.0, 16.0, 256.0][self.step % 3];
                match b {
                    Btn::Up => self.val += m,
                    Btn::Down => self.val -= m,
                    Btn::Right => self.val += 16.0 * m,
                    Btn::Left => self.val -= 16.0 * m,
                    Btn::Ok => self.step = (self.step + 1) % 3,
                    _ => o.redraw = false,
                }
                self.val = self.val.clamp(0.0, 65535.0);
            }
            Kind::Metro => match b {
                Btn::Left => self.sub = (self.sub - 1).max(2),
                Btn::Right => self.sub = (self.sub + 1).min(8),
                Btn::Up => self.val += 5.0,
                Btn::Down => self.val -= 5.0,
                Btn::Ok => {
                    self.flag = !self.flag;
                    self.beat = 0;
                    self.next_at = Instant::now();
                }
                _ => o.redraw = false,
            },
            Kind::Habit => match b {
                Btn::Up => self.sel = (self.sel + 3) % 4,
                Btn::Down => self.sel = (self.sel + 1) % 4,
                Btn::Ok => {
                    self.counts[self.sel as usize] += 1;
                    o.play = Some(vec![(C6, 40)]);
                }
                Btn::Left => self.counts[self.sel as usize] = (self.counts[self.sel as usize] - 1).max(0),
                Btn::Right => self.counts[self.sel as usize] = 0,
                _ => o.redraw = false,
            },
            Kind::Morse => match b {
                Btn::Left => {
                    self.sel = (self.sel + 35) % 36;
                    self.flag = false;
                }
                Btn::Right => {
                    self.sel = (self.sel + 1) % 36;
                    self.flag = false;
                }
                Btn::Up => {
                    self.sel = below(trng, 36).unwrap_or(0) as i32;
                    self.flag = true; // hidden quiz
                    o.play = Some(morse_notes(MORSE[self.sel as usize], 110));
                }
                Btn::Down => self.flag = false,
                Btn::Ok => o.play = Some(morse_notes(MORSE[self.sel as usize], 110)),
                _ => o.redraw = false,
            },
            Kind::Typer => match b {
                Btn::Up => self.sel = (self.sel + PRESETS.len() as i32 - 1) % PRESETS.len() as i32,
                Btn::Down => self.sel = (self.sel + 1) % PRESETS.len() as i32,
                _ => o.redraw = false,
            },
            Kind::Flash => match b {
                Btn::Up => self.sel = (self.sel + 1).min(3),
                Btn::Down => self.sel = (self.sel - 1).max(0),
                Btn::Ok => self.flag = !self.flag,
                _ => o.redraw = false,
            },
            Kind::Synth => {
                match b {
                    Btn::Left => self.sel = (self.sel + 11) % 12,
                    Btn::Right => self.sel = (self.sel + 1) % 12,
                    Btn::Up => self.sub = (self.sub + CHORDS.len() as i32 - 1) % CHORDS.len() as i32,
                    Btn::Down => self.sub = (self.sub + 1) % CHORDS.len() as i32,
                    Btn::Ok => {}
                    _ => o.redraw = false,
                }
                // every change sounds the chord: four voices share the single pin, switched very fast (an arpeggio blur)
                let root = 72 + self.sel; // C5 + root
                let iv = CHORDS[self.sub as usize].1;
                let voices = [midi_hz(root), midi_hz(root + iv[0]), midi_hz(root + iv[1]), midi_hz(root + iv[2])];
                let mut v = Vec::new();
                let rounds = if b == Btn::Ok { 30 } else { 8 };
                for _ in 0..rounds {
                    for hz in voices {
                        v.push((hz, 22));
                    }
                }
                o.play = Some(v);
                self.beat = 40;
                self.t0 = Instant::now();
            }
            Kind::Beacon => {
                if b == Btn::Ok {
                    self.flag = !self.flag;
                    o.tune = Some(if self.flag { Some(beacon_notes()) } else { None });
                    o.toast = Some(if self.flag { "carrier ON".into() } else { "carrier off".into() });
                } else {
                    o.redraw = false;
                }
            }
            Kind::Health => {
                if b == Btn::Ok {
                    self.ones = 0;
                    self.total = 0;
                    self.runs = 0;
                    self.longest = 0;
                    self.last_bit = 2;
                    self.cur_run = 0;
                } else {
                    o.redraw = false;
                }
            }
            Kind::Scope => o.redraw = false,
            Kind::Notary => {
                let evt = match b {
                    Btn::Up => "UP",
                    Btn::Down => "DOWN",
                    Btn::Left => "LEFT",
                    Btn::Right => "RIGHT",
                    _ => "",
                };
                if evt.is_empty() {
                    o.redraw = false;
                } else {
                    if self.notary.is_none() {
                        let mut seed = [0u8; 32];
                        for ch in seed.chunks_mut(4) {
                            let x = trng.get_u32().unwrap_or(0).to_le_bytes();
                            ch.copy_from_slice(&x);
                        }
                        self.notary = Some(Box::new(Notary { seed, n: 0, msg: String::new(), sig_hex: String::new(), pub_hex: String::new() }));
                    }
                    if let Some(n) = self.notary.as_mut() {
                        n.sign(evt);
                    }
                    o.play = Some(vec![(C6, 25)]);
                }
            }
        }
        o
    }

    /// Called when OK is released (`held` ms): the text presets type on a long press.
    pub fn ok_release(&mut self, held: u32) -> Out {
        let mut o = Out::default();
        if self.kind == Kind::Notary && held >= 800 {
            if let Some(n) = &self.notary {
                if n.n > 0 {
                    o.typ = Some(format!("{}|{}|{}", n.msg, n.sig_hex, n.pub_hex));
                    o.toast = Some("proof typed".into());
                    o.redraw = true;
                }
            }
        }
        if self.kind == Kind::Typer && held >= 800 {
            o.typ = Some(PRESETS[self.sel as usize].to_string());
            o.toast = Some("typed into the PC".into());
            o.play = Some(vec![(C6, 60)]);
            o.redraw = true;
        }
        o
    }

    /// Called every app tick (16 ms) with the six button states.
    pub fn tick(&mut self, trng: &Trng, now: Instant, down: &[bool; 6]) -> Out {
        let mut o = Out::default();
        match self.kind {
            Kind::Metro => {
                if self.flag && now >= self.next_at {
                    let bpm = self.val.clamp(30.0, 260.0);
                    self.next_at = now + Duration::from_millis((60_000.0 / bpm) as u64);
                    let beats = self.sub.clamp(2, 8) as u32;
                    // the first beat of each bar is a longer two-note chirp, the others one short low tick
                    o.play = Some(if self.beat % beats == 0 { vec![(E6, 40), (B6, 70)] } else { vec![(A5, 18)] });
                    self.beat += 1;
                    o.redraw = true;
                }
            }
            Kind::Scope => {
                // bit i = button i is down (order OK UP DOWN LEFT RIGHT BACK)
                let mut v = 0u8;
                for (i, d) in down.iter().enumerate() {
                    if *d {
                        v |= 1 << i;
                    }
                }
                self.hist.push(v);
                if self.hist.len() > 128 {
                    self.hist.remove(0);
                }
                o.redraw = self.hist.len() % 2 == 0;
            }
            Kind::Health => {
                self.bits.clear();
                for _ in 0..32 {
                    if let Ok(x) = trng.get_u32() {
                        self.bits.push(x);
                        self.ones += x.count_ones() as u64;
                        self.total += 32;
                        for k in 0..32 {
                            let bit = ((x >> k) & 1) as u8;
                            if bit == self.last_bit {
                                self.cur_run += 1;
                            } else {
                                self.runs += 1;
                                self.cur_run = 1;
                                self.last_bit = bit;
                            }
                            self.longest = self.longest.max(self.cur_run);
                        }
                    }
                }
                o.redraw = true;
            }
            Kind::Flash => {
                o.redraw = self.flag;
            }
            Kind::Synth => {
                o.redraw = self.beat > 0 && now.duration_since(self.t0) < Duration::from_millis(1500);
            }
            Kind::Beacon => {
                o.redraw = self.flag;
            }
            _ => {}
        }
        o
    }

    pub fn draw(&self, c: &mut Canvas, ms: u32) {
        match self.kind {
            Kind::Calc => {
                c.inset(0, 0, 128, 22);
                c.text(3, 2, &self.text);
                let r = if self.res.is_empty() { String::new() } else { format!("= {}", self.res) };
                c.text((125usize).saturating_sub(6 * r.len()), 12, &r);
                for (ri, row) in CALC_KEYS.iter().enumerate() {
                    for (ci, k) in row.iter().enumerate() {
                        let (x, y) = (1 + ci * 21, 25 + ri * 13);
                        c.rect(x, y, 19, 12);
                        c.text(x + 7, y + 2, k);
                        if (ri * 6 + ci) as i32 == self.sel {
                            c.invert_rect(x, y, 19, 12);
                        }
                    }
                }
            }
            Kind::Units => {
                let u = UNITS[self.sel as usize];
                c.card(0, 0, 128, 13);
                c.arrow(3, 2, 0);
                c.arrow(3, 7, 1);
                c.text(12, 2, u.0);
                let out = self.val * u.3 + u.4;
                c.text_big(2, 18, &format!("{}", fmt_num(self.val)), 2);
                c.text(2, 34, u.1);
                c.text_big(2, 42, &format!("{}", fmt_num(out)), 2);
                c.text(100, 56, &format!("{} {}", u.2, ""));
                c.text(60, 34, &format!("step {}", STEPS[self.step]));
            }
            Kind::Base => {
                let v = self.val as u32;
                c.text(0, 0, &format!("DEC {}", v));
                c.text(0, 11, &format!("HEX {:04X}", v));
                c.text(0, 22, &format!("BIN {:08b}", v >> 8));
                c.text(24, 32, &format!("{:08b}", v & 255));
                let ch = if (32..127).contains(&v) { (v as u8 as char).to_string() } else { "-".into() };
                c.text(0, 44, &format!("CHAR {}", ch));
                c.text(0, 55, &format!("OK step x{}", [1, 16, 256][self.step % 3]));
            }
            Kind::Metro => {
                c.text_big(4, 4, &format!("{}", self.val as i32), 3);
                c.text(80, 8, "BPM");
                c.text(80, 20, if self.flag { "playing" } else { "stopped" });
                if self.flag {
                    let beats = self.sub.clamp(2, 8) as u32;
                    let on = (self.beat + beats - 1) % beats;
                    let w = (120 / beats as usize).min(24);
                    for i in 0..beats {
                        let (x, y) = (4 + i as usize * (w + 1), 40);
                        c.rect(x, y, w - 2, 14);
                        if i == on {
                            c.fill_rect(x + 1, y + 1, w - 4, 12, true);
                        }
                    }
                } else {
                    c.text(4, 44, "OK = start");
                }
                c.text(80, 30, &format!("{} beats", self.sub.clamp(2, 8)));
            }
            Kind::Habit => {
                for (i, h) in HABITS.iter().enumerate() {
                    let y = i * 15;
                    c.card(0, y, 128, 14);
                    c.text(4, y + 3, h);
                    c.text(60, y + 3, &format!("{}", self.counts[i]));
                    for k in 0..self.counts[i].min(8) as usize {
                        c.fill_rect(80 + k * 5, y + 4, 4, 6, true);
                    }
                    if i as i32 == self.sel {
                        c.invert_rect(0, y, 128, 14);
                    }
                }
            }
            Kind::Morse => {
                let code = MORSE[self.sel as usize];
                c.text_big(4, 4, &morse_char(self.sel as usize).to_string(), 4);
                if self.flag {
                    c.text(50, 10, "quiz: what was it?");
                    c.text(50, 22, "DOWN = reveal");
                } else {
                    let mut x = 50;
                    for ch in code.chars() {
                        if ch == '.' {
                            c.fill_rect(x, 14, 4, 4, true);
                            x += 8;
                        } else {
                            c.fill_rect(x, 14, 12, 4, true);
                            x += 16;
                        }
                    }
                }
                c.text(0, 44, "<> letter  OK play");
                c.text(0, 55, "UP quiz  DOWN reveal");
            }
            Kind::Typer => {
                c.text(0, 0, "Text presets");
                // same behaviour as the menus: the highlight moves, the list scrolls only at the edges
                let sel = self.sel as usize;
                let start = if sel >= 3 { sel - 2 } else { 0 };
                for k in 0..3 {
                    let i = start + k;
                    if i >= PRESETS.len() {
                        break;
                    }
                    let y = 13 + 13 * k;
                    let t = PRESETS[i];
                    if i != sel {
                        c.card(1, y, 124, 12);
                    }
                    c.text(4, y, if t.len() > 19 { &t[..19] } else { t });
                    if i == sel {
                        c.invert_rect(1, y, 124, 12);
                    }
                }
                let n = PRESETS.len();
                if n > 3 {
                    let h = 39 * 3 / n;
                    c.fill_rect(126, 13 + (39 - h) * sel / (n - 1), 2, h, true);
                }
                c.text(0, 52, "Click a box, HOLD OK");
            }
            Kind::Flash => {
                let strobe = self.flag && (ms / 90) % 2 == 0;
                if !strobe {
                    for y in 0..64usize {
                        for x in 0..128usize {
                            let on = match self.sel {
                                3 => true,
                                2 => (x + y) % 4 != 0,
                                1 => (x + y) % 2 == 0,
                                _ => x % 4 == 0 && y % 4 == 0,
                            };
                            if on {
                                c.dot(x as i32, y as i32, true);
                            }
                        }
                    }
                }
            }
            Kind::Synth => {
                let ch = CHORDS[self.sub as usize];
                c.text(0, 0, &format!("{} {}", NOTE_NAMES[self.sel as usize], ch.0));
                c.text(0, 12, "<> root  ^v type");
                let active = self.beat > 0 && Instant::now().duration_since(self.t0) < Duration::from_millis(1500);
                for v in 0..4usize {
                    let h = if active { 4 + (((ms / 60) as usize * 7 + v * 13) % 28) } else { 4 };
                    c.fill_rect(14 + v * 30, 56 - h, 20, h, true);
                }
                c.text(0, 56, "");
            }
            Kind::Scope => {
                const NAMES: [&str; 6] = ["OK", "UP", "DN", "LT", "RT", "BK"];
                for i in 0..6usize {
                    let y = i * 10 + 2;
                    c.text(0, y, NAMES[i]);
                    let mut prev = false;
                    for (x, s) in self.hist.iter().enumerate() {
                        let on = s & (1 << i) != 0;
                        let yy = if on { y } else { y + 7 } as i32;
                        c.dot(14 + x as i32 * 8 / 9, yy, true);
                        if on != prev && x > 0 {
                            c.line(14 + x as i32 * 8 / 9, y as i32, 14 + x as i32 * 8 / 9, y as i32 + 7);
                        }
                        prev = on;
                    }
                }
            }
            Kind::Health => {
                // the newest 32 random words as a 64x32 pixel noise picture
                for (i, w) in self.bits.iter().enumerate() {
                    for k in 0..32usize {
                        if (w >> k) & 1 == 1 {
                            c.dot((k + (i % 2) * 32) as i32, 20 + (i / 2) as i32, true);
                        }
                    }
                }
                let pct = if self.total > 0 { self.ones as f64 * 100.0 / self.total as f64 } else { 50.0 };
                c.text(70, 0, "TRNG");
                c.text(66, 12, &format!("{:.2}%", pct));
                c.text(66, 24, &format!("run {}", self.longest));
                let exp_runs = self.total / 2;
                let dev = if exp_runs > 0 { (self.runs as f64 - exp_runs as f64).abs() * 100.0 / exp_runs as f64 } else { 0.0 };
                c.text(66, 36, &format!("dev {:.1}%", dev));
                let good = self.total < 20_000 || ((pct - 50.0).abs() < 1.0 && dev < 2.0 && self.longest < 40);
                c.text(66, 48, if good { "looks OK" } else { "CHECK!" });
                c.text(0, 0, &format!("{}k", self.total / 1000));
            }
            Kind::Notary => {
                c.card(0, 0, 128, 13);
                c.text(4, 0, "Notary (Ed25519)");
                match &self.notary {
                    Some(n) if n.n > 0 => {
                        c.text(0, 15, &format!("#{} {}", n.n, n.msg.rsplit('|').next().unwrap_or("")));
                        c.text(0, 26, &format!("sig {}..", &n.sig_hex[..12]));
                        c.text(0, 37, &format!("pub {}..", &n.pub_hex[..12]));
                    }
                    _ => c.text(0, 15, "press ^ v < > = event"),
                }
                c.text(0, 53, "HOLD OK: type proof");
            }
            Kind::Beacon => {
                c.text(0, 0, "AM beacon (250 kHz)");
                c.text(0, 13, if self.flag { "ON: keying SHREYAS" } else { "OFF" });
                c.text(0, 26, "tune an AM radio near");
                c.text(0, 36, "the board: 750 or 1250");
                c.text(0, 46, "kHz (harmonics)");
                if self.flag {
                    for i in 0..6 {
                        let r = ((ms / 120) % 5 + i * 2) as i32;
                        for dx in -r..=r {
                            c.dot(118 + dx / 2, 30 - r + dx.abs() / 2, true);
                        }
                    }
                }
            }
        }
    }
}

/// Morse "SHREYAS" as carrier on/off: the tone kernel runs at 250 kHz when the frequency is 250000.
fn beacon_notes() -> &'static [Note] {
    static BEACON: std::sync::OnceLock<Vec<Note>> = std::sync::OnceLock::new();
    BEACON.get_or_init(|| {
        let mut v: Vec<Note> = Vec::new();
        for ch in "SHREYAS".chars() {
            let idx = (ch as u8 - b'A') as usize;
            for m in MORSE[idx].chars() {
                v.push((250_000, if m == '.' { 110 } else { 300 }));
                v.push((REST, 110));
            }
            v.push((REST, 250));
        }
        v.push((REST, 800));
        v
    })
}
