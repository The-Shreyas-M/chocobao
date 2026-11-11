// games2.rs - eleven more small games for the 128x64 OLED, all using the `Game` trait from games.rs.
//   0 Breakout   1 Block Drop   2 2048   3 Tic-Tac-Toe   4 Connect 4   5 Memory Pairs
//   6 Whack-a-Mole   7 Minesweeper   8 Space Shooter   9 Lunar Lander   10 Maze Runner
// Rules of the house: no panics (everything is bounds-checked), only non-negative sizes ever reach
// `Canvas::fill_rect` (the helpers below clip), randomness comes from the hardware TRNG, BACK is never used here
// (the app pauses / quits), and real-time games measure time with `Instant` so the speed does not depend on how often
// `update` is called.
use std::time::{Duration, Instant};

use bao1x_hal_service::trng::Trng;

use crate::controls::Btn;
use crate::games::{Game, Snd};
use crate::oled::Canvas;
use crate::rng::below;

pub const NAMES: [&str; 13] = [
    "Breakout",
    "Tetris",
    "2048",
    "Tic-Tac-Toe",
    "Connect 4",
    "Memory",
    "Whack-a-Mole",
    "Minesweeper",
    "Space Invaders",
    "Lunar Lander",
    "Maze",
    "Tower Blocks",
    "FreeCell",
];

pub const DESCS: [&str; 13] = [
    "Smash all the bricks",
    "Stack falling blocks",
    "Merge tiles to 2048",
    "Beat the computer",
    "Four in a row vs CPU",
    "Find the 6 pairs",
    "Hit moles in 30 s",
    "Hold OK to flag mines",
    "Shoot the invaders",
    "Land softly on the pad",
    "Find the exit, 3 levels",
    "Stack the city blocks",
    "All cards face up",
];

pub const HELP: [[&str; 4]; 13] = [
    ["<> move paddle", "OK launch the ball", "clear every brick", "3 lives"],
    ["<> move  ^ rotate", "v soft drop", "OK = hard drop", "clear full lines"],
    ["^v<> slide tiles", "equal tiles merge", "reach 2048", "no moves = game over"],
    ["^v<> move cursor", "OK place your X", "computer plays O", "3 in a row wins"],
    ["<> pick a column", "OK or v drops a disc", "you = solid disc", "get 4 in a row"],
    ["^v<> move cursor", "OK flips a card", "find the 6 pairs", "fewer moves = more"],
    ["^v<> move cursor", "OK hits the mole", "30 seconds", "faster as you score"],
    ["^v<> move cursor", "OK reveals a cell", "HOLD OK = flag", "clear all safe cells"],
    ["<> move the ship", "OK shoots", "clear the waves", "3 lives"],
    ["OK = main thrust", "<> side thrusters", "land on the pad", "slow and level"],
    ["^v<> walk", "find the exit", "at the far corner", "3 levels, bigger each"],
    ["OK drops the block", "land it on the tower", "overhang is cut off", "perfect = bonus"],
    ["<> pick a spot  ^v", "OK lift, OK drop", "HOLD OK = foundation", "4 cells hold 1 card"],
];

/// 8x8 menu icons, one byte per row, most significant bit = leftmost pixel.
pub const ICONS: [[u8; 8]; 13] = [
    [0xFF, 0x00, 0xDB, 0x00, 0x00, 0x10, 0x00, 0x7E], // Breakout
    [0x00, 0x18, 0x18, 0x7E, 0x00, 0x6C, 0x6C, 0x00], // Block Drop
    [0x7E, 0x81, 0xB9, 0xA1, 0xB9, 0x85, 0xB9, 0x7E], // 2048
    [0x24, 0x24, 0xFF, 0x24, 0x24, 0xFF, 0x24, 0x24], // Tic-Tac-Toe
    [0x00, 0x3C, 0x7E, 0x7E, 0x7E, 0x7E, 0x3C, 0x00], // Connect 4
    [0x7E, 0x42, 0x5A, 0x4A, 0x4A, 0x42, 0x7E, 0x00], // Memory
    [0x00, 0x18, 0x3C, 0x7E, 0x66, 0x00, 0xFF, 0x7E], // Whack-a-Mole
    [0x08, 0x2A, 0x1C, 0x7F, 0x1C, 0x2A, 0x08, 0x00], // Minesweeper
    [0x10, 0x10, 0x38, 0x38, 0x7C, 0xFE, 0xFE, 0x00], // Space Shooter
    [0x18, 0x24, 0x7E, 0x42, 0x24, 0x42, 0x81, 0xFF], // Lunar Lander
    [0xFF, 0x81, 0xBD, 0xA1, 0xAD, 0xA5, 0xBD, 0x81], // Maze
    [0x18, 0x18, 0x3C, 0x3C, 0x7E, 0x7E, 0xFF, 0xFF], // Tower
    [0x7E, 0x81, 0x99, 0xBD, 0xBD, 0x99, 0x81, 0x7E], // FreeCell
];

pub fn make(i: usize, t: &Trng) -> Box<dyn Game> {
    match i {
        0 => Box::new(Breakout::new()),
        1 => Box::new(Tetris::new(t)),
        2 => Box::new(G2048::new(t)),
        3 => Box::new(TicTacToe::new()),
        4 => Box::new(Connect4::new()),
        5 => Box::new(Memory::new(t)),
        6 => Box::new(Whack::new(t)),
        7 => Box::new(Mines::new()),
        8 => Box::new(Shooter::new()),
        9 => Box::new(Lander::new(t)),
        10 => Box::new(Maze::new(t)),
        11 => Box::new(Tower::new()),
        _ => Box::new(FreeCell::new(t)),
    }
}

// ---------------------------------------------------------------- helpers
fn rnd(t: &Trng, n: u32) -> u32 { below(t, n).unwrap_or(0) }

/// Clipped filled rectangle (negative or oversized values can never reach `Canvas::fill_rect`).
fn fr(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, on: bool) {
    let (x0, y0, x1, y1) = (x.max(0), y.max(0), (x + w).min(128), (y + h).min(64));
    if x1 > x0 && y1 > y0 {
        c.fill_rect(x0 as usize, y0 as usize, (x1 - x0) as usize, (y1 - y0) as usize, on);
    }
}

fn rc(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    if w < 2 || h < 2 {
        return;
    }
    fr(c, x, y, w, 1, true);
    fr(c, x, y + h - 1, w, 1, true);
    fr(c, x, y, 1, h, true);
    fr(c, x + w - 1, y, 1, h, true);
}

fn txt(c: &mut Canvas, x: i32, y: i32, s: &str) { c.text(x.clamp(0, 127) as usize, y.clamp(0, 63) as usize, s); }

fn disc(c: &mut Canvas, cx: i32, cy: i32, r: i32) {
    for dy in -r..=r {
        let w = Canvas::dome_h(r, r, dy);
        fr(c, cx - w, cy + dy, 2 * w + 1, 1, true);
    }
}

fn ring(c: &mut Canvas, cx: i32, cy: i32, r: i32) {
    for d in -r..=r {
        let h = Canvas::dome_h(r, r, d);
        c.dot(cx + d, cy - h, true);
        c.dot(cx + d, cy + h, true);
        c.dot(cx - h, cy + d, true);
        c.dot(cx + h, cy + d, true);
    }
}

fn over_box(c: &mut Canvas, msg: &str) {
    c.fill_rect(6, 16, 116, 42, false);
    c.rect(6, 16, 116, 42);
    c.text_big(12, 19, msg, 2);
    c.text(12, 44, "OK retry  BACK exit");
}

/// Milliseconds since the last call (capped, so a long stall never makes things jump).
fn dt_ms(last: &mut Instant) -> u32 {
    let d = last.elapsed().as_millis().min(100) as u32;
    *last = Instant::now();
    d
}

/// Adds `dt` to the accumulator and returns how many whole `period`s have passed (at most 4).
fn steps(acc: &mut u32, dt: u32, period: u32) -> u32 {
    *acc += dt;
    let n = *acc / period.max(1);
    *acc %= period.max(1);
    n.min(4)
}

// ---------------------------------------------------------------- 0 Breakout
pub struct Breakout {
    br: [[bool; 10]; 4],
    left: i32,
    px: i32,
    bx: i32,
    by: i32,
    vx: i32,
    vy: i32,
    stuck: bool,
    lives: i32,
    score: u32,
    level: u32,
    alive: bool,
    paused: bool,
    dir: i32,
    last: Instant,
    acc: u32,
    snd: Option<Snd>,
}

impl Breakout {
    pub fn new() -> Self {
        Breakout {
            br: [[true; 10]; 4],
            left: 40,
            px: 56,
            bx: 63,
            by: 57,
            vx: 1,
            vy: -1,
            stuck: true,
            lives: 3,
            score: 0,
            level: 1,
            alive: true,
            paused: false,
            dir: 0,
            last: Instant::now(),
            acc: 0,
            snd: None,
        }
    }
    fn speed(&self) -> i32 { if self.level >= 2 { 2 } else { 1 } }
    fn step(&mut self) {
        self.px = (self.px + self.dir * 3).clamp(0, 112);
        if self.stuck {
            self.bx = self.px + 7;
            self.by = 57;
            return;
        }
        self.bx += self.vx;
        self.by += self.vy;
        if self.bx < 0 {
            self.bx = 0;
            self.vx = self.vx.abs();
        }
        if self.bx > 125 {
            self.bx = 125;
            self.vx = -self.vx.abs();
        }
        if self.by < 13 {
            self.by = 13;
            self.vy = self.vy.abs();
        }
        let (cx, cy) = (self.bx + 1, self.by + 1);
        if (13..29).contains(&cy) && cx >= 4 {
            let (r, c) = (((cy - 13) / 4) as usize, ((cx - 4) / 12) as usize);
            if r < 4 && c < 10 && self.br[r][c] {
                self.br[r][c] = false;
                self.left -= 1;
                self.score += 10;
                self.vy = -self.vy;
                self.snd = Some(Snd::Coin);
            }
        }
        if self.vy > 0 && self.by + 3 >= 60 && self.by <= 62 && self.bx + 3 >= self.px && self.bx <= self.px + 16 {
            self.by = 57;
            let rel = self.bx + 1 - self.px;
            let mut vx = ((rel - 8) / 3).clamp(-2, 2);
            if vx == 0 {
                vx = if rel < 8 { -1 } else { 1 };
            }
            self.vx = vx;
            self.vy = -self.speed();
            self.snd = Some(Snd::Tick);
        }
        if self.by >= 64 {
            self.lives -= 1;
            self.stuck = true;
            self.snd = Some(Snd::Crash);
            if self.lives <= 0 {
                self.alive = false;
            }
        }
        if self.left <= 0 {
            self.level += 1;
            self.br = [[true; 10]; 4];
            self.left = 40;
            self.stuck = true;
            self.snd = Some(Snd::Coin);
        }
    }
}

impl Game for Breakout {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.alive {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        if b == Btn::Ok && self.stuck && self.alive {
            self.stuck = false;
            self.vx = if self.px < 56 { 1 } else { -1 };
            self.vy = -self.speed();
            return true;
        }
        false
    }
    fn held(&mut self, down: &[bool; 6]) { self.dir = (down[4] as i32) - (down[3] as i32); }
    fn update(&mut self, _t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = dt_ms(&mut self.last);
        let n = steps(&mut self.acc, dt, 25);
        for _ in 0..n {
            self.step();
        }
        n > 0
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("L{} {}", self.lives.max(0), self.score));
        for r in 0..4usize {
            for col in 0..10usize {
                if self.br[r][col] {
                    fr(c, 4 + col as i32 * 12, 13 + r as i32 * 4, 11, 3, true);
                }
            }
        }
        fr(c, self.px, 60, 16, 3, true);
        fr(c, self.bx, self.by, 3, 3, true);
        if self.stuck && self.alive {
            txt(c, 30, 36, "OK = launch");
        }
        if !self.alive {
            over_box(c, "GAME OVER");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 1 Block Drop (falling blocks)
const TW: usize = 10;
const TH: usize = 20;
const SHAPES: [[(i32, i32); 4]; 7] = [
    [(-1, 0), (0, 0), (1, 0), (2, 0)],  // I
    [(0, 0), (1, 0), (0, 1), (1, 1)],   // O
    [(-1, 0), (0, 0), (1, 0), (0, 1)],  // T
    [(0, 0), (1, 0), (-1, 1), (0, 1)],  // S
    [(-1, 0), (0, 0), (0, 1), (1, 1)],  // Z
    [(-1, 0), (0, 0), (1, 0), (-1, 1)], // J
    [(-1, 0), (0, 0), (1, 0), (1, 1)],  // L
];

pub struct Tetris {
    b: [[bool; TW]; TH],
    cur: [(i32, i32); 4],
    kind: usize,
    cx: i32,
    cy: i32,
    next: usize,
    score: u32,
    lines: u32,
    alive: bool,
    paused: bool,
    soft: bool,
    last: Instant,
    acc: u32,
    rep_at: Instant,
    snd: Option<Snd>,
}

impl Tetris {
    pub fn new(t: &Trng) -> Self {
        let mut g = Tetris {
            b: [[false; TW]; TH],
            cur: SHAPES[0],
            kind: 0,
            cx: 4,
            cy: 0,
            next: rnd(t, 7) as usize,
            score: 0,
            lines: 0,
            alive: true,
            paused: false,
            soft: false,
            last: Instant::now(),
            acc: 0,
            rep_at: Instant::now(),
            snd: None,
        };
        g.spawn(t);
        g
    }
    fn fits(&self, cells: &[(i32, i32); 4], cx: i32, cy: i32) -> bool {
        cells.iter().all(|&(dx, dy)| {
            let (x, y) = (cx + dx, cy + dy);
            x >= 0 && x < TW as i32 && y < TH as i32 && (y < 0 || !self.b[y as usize][x as usize])
        })
    }
    fn spawn(&mut self, t: &Trng) {
        self.kind = self.next.min(6);
        self.next = rnd(t, 7) as usize;
        self.cur = SHAPES[self.kind];
        self.cx = 4;
        self.cy = 0;
        if !self.fits(&self.cur, self.cx, self.cy) {
            self.alive = false;
            self.snd = Some(Snd::Crash);
        }
    }
    fn mv(&mut self, dx: i32) -> bool {
        if self.fits(&self.cur, self.cx + dx, self.cy) {
            self.cx += dx;
            true
        } else {
            false
        }
    }
    fn rotate(&mut self) -> bool {
        if self.kind == 1 {
            return false;
        }
        let mut n = self.cur;
        for p in n.iter_mut() {
            *p = (-p.1, p.0);
        }
        for k in [0, -1, 1, -2, 2] {
            if self.fits(&n, self.cx + k, self.cy) {
                self.cur = n;
                self.cx += k;
                return true;
            }
        }
        false
    }
    fn lock(&mut self, t: &Trng) {
        for &(dx, dy) in self.cur.iter() {
            let (x, y) = (self.cx + dx, self.cy + dy);
            if x >= 0 && y >= 0 && (x as usize) < TW && (y as usize) < TH {
                self.b[y as usize][x as usize] = true;
            }
        }
        let mut cleared = 0u32;
        let mut y = TH;
        while y > 0 {
            y -= 1;
            if self.b[y].iter().all(|&v| v) {
                for yy in (1..=y).rev() {
                    self.b[yy] = self.b[yy - 1];
                }
                self.b[0] = [false; TW];
                cleared += 1;
                y += 1; // look at this row again
            }
        }
        if cleared > 0 {
            let table = [0u32, 100, 300, 500, 800];
            self.score += table[(cleared as usize).min(4)] * (self.lines / 10 + 1);
            self.lines += cleared;
            self.snd = Some(Snd::Coin);
        } else {
            self.snd = Some(Snd::Tick);
        }
        self.spawn(t);
    }
    fn interval(&self) -> u32 {
        let base = 700u32.saturating_sub((self.lines / 10) * 60).max(120);
        if self.soft { 60.min(base) } else { base }
    }
}

impl Game for Tetris {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.alive {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        if !self.alive {
            return false;
        }
        self.rep_at = Instant::now() + Duration::from_millis(200);
        match b {
            Btn::Left => {
                self.mv(-1);
            }
            Btn::Right => {
                self.mv(1);
            }
            Btn::Up => {
                self.rotate();
            }
            Btn::Down => {
                if self.fits(&self.cur, self.cx, self.cy + 1) {
                    self.cy += 1;
                } else {
                    self.lock(t);
                }
            }
            Btn::Ok => {
                while self.fits(&self.cur, self.cx, self.cy + 1) {
                    self.cy += 1;
                    self.score += 2;
                }
                self.lock(t);
            }
            _ => return false,
        }
        true
    }
    fn held(&mut self, down: &[bool; 6]) {
        self.soft = down[2];
        let (l, r) = (down[3], down[4]);
        if self.alive && !self.paused && l != r && Instant::now() >= self.rep_at {
            self.mv(if l { -1 } else { 1 });
            self.rep_at = Instant::now() + Duration::from_millis(90);
        }
    }
    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = dt_ms(&mut self.last);
        let iv = self.interval();
        let n = steps(&mut self.acc, dt, iv);
        for _ in 0..n {
            if self.fits(&self.cur, self.cx, self.cy + 1) {
                self.cy += 1;
            } else {
                self.lock(t);
            }
            if !self.alive {
                break;
            }
        }
        // the falling block moves, so keep redrawing at the gravity rate; n == 0 means nothing changed
        n > 0 || self.soft
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        rc(c, 38, 1, 34, 62);
        for y in 0..TH {
            for x in 0..TW {
                if self.b[y][x] {
                    let (px, py) = (40 + x as i32 * 3, 2 + y as i32 * 3);
                    fr(c, px, py, 3, 3, true);
                    c.dot(px + 1, py + 1, false);
                }
            }
        }
        for &(dx, dy) in self.cur.iter() {
            let (x, y) = (self.cx + dx, self.cy + dy);
            if y >= 0 {
                fr(c, 40 + x * 3, 2 + y * 3, 3, 3, true);
            }
        }
        txt(c, 2, 2, "NEXT");
        for &(dx, dy) in SHAPES[self.next.min(6)].iter() {
            fr(c, 12 + dx * 3, 20 + dy * 3, 3, 3, true);
        }
        txt(c, 78, 2, "SCORE");
        txt(c, 78, 14, &format!("{}", self.score));
        txt(c, 78, 30, "LINES");
        txt(c, 78, 42, &format!("{}", self.lines));
        if !self.alive {
            over_box(c, "GAME OVER");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 2 2048
pub struct G2048 {
    b: [[u8; 4]; 4], // exponent: 0 empty, k = tile 2^k
    score: u32,
    alive: bool,
    dirty: bool,
    snd: Option<Snd>,
}

/// Slide one line towards index 0 and merge equal neighbours once. Returns (line, points, moved).
fn slide(line: [u8; 4]) -> ([u8; 4], u32, bool) {
    let mut out = [0u8; 4];
    let (mut n, mut pts) = (0usize, 0u32);
    let mut last_merged = false;
    for &v in line.iter().filter(|&&v| v != 0) {
        if n > 0 && !last_merged && out[n - 1] == v {
            out[n - 1] = v + 1;
            pts += 1u32 << (v as u32 + 1).min(20);
            last_merged = true;
        } else {
            out[n] = v;
            n += 1;
            last_merged = false;
        }
    }
    (out, pts, out != line)
}

impl G2048 {
    pub fn new(t: &Trng) -> Self {
        let mut g = G2048 { b: [[0; 4]; 4], score: 0, alive: true, dirty: true, snd: None };
        g.add(t);
        g.add(t);
        g
    }
    fn add(&mut self, t: &Trng) {
        let empty: Vec<(usize, usize)> =
            (0..4).flat_map(|r| (0..4).map(move |c| (r, c))).filter(|&(r, c)| self.b[r][c] == 0).collect();
        if empty.is_empty() {
            return;
        }
        let (r, c) = empty[rnd(t, empty.len() as u32) as usize];
        self.b[r][c] = if rnd(t, 10) == 0 { 2 } else { 1 };
    }
    fn can_move(&self) -> bool {
        for r in 0..4 {
            for c in 0..4 {
                if self.b[r][c] == 0 {
                    return true;
                }
                if c + 1 < 4 && self.b[r][c] == self.b[r][c + 1] {
                    return true;
                }
                if r + 1 < 4 && self.b[r][c] == self.b[r + 1][c] {
                    return true;
                }
            }
        }
        false
    }
}

impl Game for G2048 {
    fn press(&mut self, b: Btn, t: &Trng) -> bool {
        if !self.alive {
            return false;
        }
        // map every direction onto "slide towards index 0" of a line
        let idx = |r: usize, c: usize, k: usize| -> (usize, usize) {
            match b {
                Btn::Left => (r, k),
                Btn::Right => (r, 3 - k),
                Btn::Up => (k, r),
                _ => (3 - k, r),
            }
        };
        if !matches!(b, Btn::Left | Btn::Right | Btn::Up | Btn::Down) {
            return false;
        }
        let (mut moved, mut gained) = (false, 0u32);
        let mut nb = self.b;
        for r in 0..4 {
            let mut line = [0u8; 4];
            for k in 0..4 {
                let (rr, cc) = idx(r, 0, k);
                line[k] = self.b[rr][cc];
            }
            let (out, pts, m) = slide(line);
            moved |= m;
            gained += pts;
            for k in 0..4 {
                let (rr, cc) = idx(r, 0, k);
                nb[rr][cc] = out[k];
            }
        }
        if !moved {
            return false;
        }
        self.b = nb;
        self.score += gained;
        self.add(t);
        self.snd = Some(if gained > 0 { Snd::Coin } else { Snd::Tick });
        if !self.can_move() {
            self.alive = false;
            self.snd = Some(Snd::Crash);
        }
        self.dirty = true;
        true
    }
    fn update(&mut self, _t: &Trng) -> bool {
        let d = self.dirty;
        self.dirty = false;
        d
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        txt(c, 0, 0, &format!("{} best {}", self.score, best));
        for r in 0..4usize {
            for col in 0..4usize {
                let (x, y) = (col as i32 * 25, 11 + r as i32 * 13);
                let v = self.b[r][col];
                if v == 0 {
                    c.dot(x + 12, y + 6, true);
                } else {
                    rc(c, x, y, 25, 13);
                    let s = format!("{}", 1u32 << (v as u32).min(20));
                    txt(c, x + (25 - 6 * s.len() as i32) / 2, y, &s);
                    if v >= 7 {
                        c.invert_rect((x + 1) as usize, (y + 1) as usize, 23, 11);
                    }
                }
            }
        }
        txt(c, 104, 14, "^v");
        txt(c, 104, 26, "<>");
        if !self.alive {
            over_box(c, "NO MOVES");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 3 Tic-Tac-Toe
const LINES3: [(usize, usize, usize); 8] =
    [(0, 1, 2), (3, 4, 5), (6, 7, 8), (0, 3, 6), (1, 4, 7), (2, 5, 8), (0, 4, 8), (2, 4, 6)];

fn ttt_winner(b: &[u8; 9]) -> u8 {
    for &(x, y, z) in LINES3.iter() {
        if b[x] != 0 && b[x] == b[y] && b[y] == b[z] {
            return b[x];
        }
    }
    0
}

/// Minimax from the computer's (2) point of view.
fn ttt_mm(b: &mut [u8; 9], turn: u8, depth: i32) -> i32 {
    match ttt_winner(b) {
        2 => return 10 - depth,
        1 => return depth - 10,
        _ => {}
    }
    if b.iter().all(|&v| v != 0) {
        return 0;
    }
    let mut best = if turn == 2 { -100 } else { 100 };
    for i in 0..9 {
        if b[i] == 0 {
            b[i] = turn;
            let s = ttt_mm(b, 3 - turn, depth + 1);
            b[i] = 0;
            best = if turn == 2 { best.max(s) } else { best.min(s) };
        }
    }
    best
}

pub struct TicTacToe {
    b: [u8; 9],
    cur: usize,
    result: u8, // 0 playing, 1 you, 2 computer, 3 draw
    snd: Option<Snd>,
}

impl TicTacToe {
    pub fn new() -> Self { TicTacToe { b: [0; 9], cur: 4, result: 0, snd: None } }
    fn settle(&mut self) {
        let w = ttt_winner(&self.b);
        self.result = if w != 0 {
            w
        } else if self.b.iter().all(|&v| v != 0) {
            3
        } else {
            0
        };
        if self.result == 1 {
            self.snd = Some(Snd::Coin);
        } else if self.result == 2 {
            self.snd = Some(Snd::Crash);
        }
    }
}

impl Game for TicTacToe {
    fn press(&mut self, b: Btn, t: &Trng) -> bool {
        if self.result != 0 {
            return false;
        }
        let (r, c) = (self.cur / 3, self.cur % 3);
        match b {
            Btn::Left => self.cur = r * 3 + (c + 2) % 3,
            Btn::Right => self.cur = r * 3 + (c + 1) % 3,
            Btn::Up => self.cur = ((r + 2) % 3) * 3 + c,
            Btn::Down => self.cur = ((r + 1) % 3) * 3 + c,
            Btn::Ok => {
                if self.b[self.cur] != 0 {
                    return false;
                }
                self.b[self.cur] = 1;
                self.snd = Some(Snd::Click);
                self.settle();
                if self.result == 0 {
                    let free: Vec<usize> = (0..9).filter(|&i| self.b[i] == 0).collect();
                    let mv = if rnd(t, 100) < 12 {
                        free[rnd(t, free.len() as u32) as usize] // the computer sometimes slips
                    } else {
                        let (mut best, mut bi) = (-1000, free[0]);
                        for &i in &free {
                            self.b[i] = 2;
                            let s = ttt_mm(&mut self.b, 1, 1);
                            self.b[i] = 0;
                            if s > best {
                                best = s;
                                bi = i;
                            }
                        }
                        bi
                    };
                    self.b[mv] = 2;
                    self.settle();
                }
            }
            _ => return false,
        }
        true
    }
    fn update(&mut self, _t: &Trng) -> bool { false }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 14, "You X");
        txt(c, 0, 28, "CPU O");
        let (x0, y0) = (40, 13);
        fr(c, x0 + 16, y0, 1, 48, true);
        fr(c, x0 + 32, y0, 1, 48, true);
        fr(c, x0, y0 + 16, 48, 1, true);
        fr(c, x0, y0 + 32, 48, 1, true);
        for i in 0..9usize {
            let (cx, cy) = (x0 + (i as i32 % 3) * 16 + 8, y0 + (i as i32 / 3) * 16 + 8);
            match self.b[i] {
                1 => {
                    c.line(cx - 5, cy - 5, cx + 5, cy + 5);
                    c.line(cx - 5, cy + 5, cx + 5, cy - 5);
                }
                2 => ring(c, cx, cy, 5),
                _ => {}
            }
            if i == self.cur && self.result == 0 {
                rc(c, cx - 7, cy - 7, 15, 15);
            }
        }
        match self.result {
            1 => over_box(c, "YOU WIN"),
            2 => over_box(c, "YOU LOSE"),
            3 => over_box(c, "DRAW"),
            _ => {}
        }
    }
    fn score(&self) -> u32 {
        match self.result {
            1 => 100,
            3 => 40,
            _ => 0,
        }
    }
    fn over(&self) -> bool { self.result != 0 }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 4 Connect 4
type Bd = [[u8; 7]; 6]; // row 0 = top; 1 = you, 2 = computer

fn c4_four(b: &Bd, p: u8) -> bool {
    for r in 0..6usize {
        for c in 0..7usize {
            if b[r][c] != p {
                continue;
            }
            for (dr, dc) in [(0i32, 1i32), (1, 0), (1, 1), (1, -1)] {
                let mut k = 1;
                while k < 4 {
                    let (rr, cc) = (r as i32 + dr * k, c as i32 + dc * k);
                    if rr < 0 || rr >= 6 || cc < 0 || cc >= 7 || b[rr as usize][cc as usize] != p {
                        break;
                    }
                    k += 1;
                }
                if k == 4 {
                    return true;
                }
            }
        }
    }
    false
}

fn c4_drop(b: &mut Bd, col: usize, p: u8) -> Option<usize> {
    if col >= 7 {
        return None;
    }
    for r in (0..6).rev() {
        if b[r][col] == 0 {
            b[r][col] = p;
            return Some(r);
        }
    }
    None
}

fn c4_eval(b: &Bd) -> i32 {
    let mut s = 0;
    for r in 0..6i32 {
        for c in 0..7i32 {
            for (dr, dc) in [(0i32, 1i32), (1, 0), (1, 1), (1, -1)] {
                let (er, ec) = (r + dr * 3, c + dc * 3);
                if er < 0 || er >= 6 || ec < 0 || ec >= 7 {
                    continue;
                }
                let (mut a, mut h) = (0, 0);
                for k in 0..4 {
                    match b[(r + dr * k) as usize][(c + dc * k) as usize] {
                        2 => a += 1,
                        1 => h += 1,
                        _ => {}
                    }
                }
                if h == 0 {
                    s += [0, 0, 2, 6, 1000][a as usize];
                }
                if a == 0 {
                    s -= [0, 0, 2, 8, 1000][h as usize];
                }
            }
        }
    }
    for r in 0..6 {
        match b[r][3] {
            2 => s += 3,
            1 => s -= 3,
            _ => {}
        }
    }
    s
}

fn c4_search(b: &mut Bd, depth: i32, mut alpha: i32, mut beta: i32, ai: bool) -> i32 {
    if c4_four(b, 2) {
        return 10000 + depth;
    }
    if c4_four(b, 1) {
        return -10000 - depth;
    }
    if depth == 0 {
        return c4_eval(b);
    }
    let order = [3usize, 2, 4, 1, 5, 0, 6];
    let mut any = false;
    let mut best = if ai { -100_000 } else { 100_000 };
    for &col in order.iter() {
        if let Some(r) = c4_drop(b, col, if ai { 2 } else { 1 }) {
            any = true;
            let v = c4_search(b, depth - 1, alpha, beta, !ai);
            b[r][col] = 0;
            if ai {
                best = best.max(v);
                alpha = alpha.max(best);
            } else {
                best = best.min(v);
                beta = beta.min(best);
            }
            if alpha >= beta {
                break;
            }
        }
    }
    if any { best } else { 0 }
}

pub struct Connect4 {
    b: Bd,
    col: usize,
    result: u8, // 0 playing, 1 you, 2 computer, 3 draw
    ai_at: Option<Instant>,
    dirty: bool,
    snd: Option<Snd>,
}

impl Connect4 {
    pub fn new() -> Self { Connect4 { b: [[0; 7]; 6], col: 3, result: 0, ai_at: None, dirty: true, snd: None } }
    fn settle(&mut self) {
        self.result = if c4_four(&self.b, 1) {
            1
        } else if c4_four(&self.b, 2) {
            2
        } else if self.b[0].iter().all(|&v| v != 0) {
            3
        } else {
            0
        };
        match self.result {
            1 => self.snd = Some(Snd::Coin),
            2 => self.snd = Some(Snd::Crash),
            _ => {}
        }
    }
}

impl Game for Connect4 {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.result != 0 || self.ai_at.is_some() {
            return false;
        }
        match b {
            Btn::Left => self.col = (self.col + 6) % 7,
            Btn::Right => self.col = (self.col + 1) % 7,
            Btn::Ok | Btn::Down => {
                if c4_drop(&mut self.b, self.col, 1).is_none() {
                    return false;
                }
                self.snd = Some(Snd::Click);
                self.settle();
                if self.result == 0 {
                    self.ai_at = Some(Instant::now() + Duration::from_millis(300));
                }
            }
            _ => return false,
        }
        true
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if let Some(at) = self.ai_at {
            if Instant::now() >= at {
                self.ai_at = None;
                let order = [3usize, 2, 4, 1, 5, 0, 6];
                let mut best = (-1_000_000, 3usize);
                for &col in order.iter() {
                    if let Some(r) = c4_drop(&mut self.b, col, 2) {
                        let v = c4_search(&mut self.b, 3, -100_000, 100_000, false);
                        self.b[r][col] = 0;
                        if v > best.0 {
                            best = (v, col);
                        }
                    }
                }
                c4_drop(&mut self.b, best.1, 2);
                self.snd = Some(Snd::Tick);
                self.settle();
                return true;
            }
        }
        false
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        let (x0, y0) = (36, 14);
        rc(c, x0 - 1, y0 - 1, 58, 50);
        for r in 0..6usize {
            for col in 0..7usize {
                let (cx, cy) = (x0 + col as i32 * 8 + 4, y0 + r as i32 * 8 + 4);
                match self.b[r][col] {
                    1 => disc(c, cx, cy, 3),
                    2 => ring(c, cx, cy, 3),
                    _ => c.dot(cx, cy, true),
                }
            }
        }
        if self.result == 0 {
            let cx = x0 + self.col as i32 * 8 + 4;
            fr(c, cx - 2, 9, 5, 1, true);
            fr(c, cx - 1, 10, 3, 1, true);
            fr(c, cx, 11, 1, 1, true);
        }
        txt(c, 0, 14, "You");
        disc(c, 9, 34, 3);
        txt(c, 0, 40, "CPU");
        ring(c, 9, 60, 3);
        match self.result {
            1 => over_box(c, "YOU WIN"),
            2 => over_box(c, "YOU LOSE"),
            3 => over_box(c, "DRAW"),
            _ => {}
        }
    }
    fn score(&self) -> u32 {
        match self.result {
            1 => 100,
            3 => 40,
            _ => 0,
        }
    }
    fn over(&self) -> bool { self.result != 0 }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 5 Memory Pairs (4x3 cards, 6 pairs)
pub struct Memory {
    cards: [u8; 12],
    state: [u8; 12], // 0 hidden, 1 face up, 2 matched
    cur: usize,
    first: Option<usize>,
    hide_at: Option<Instant>,
    moves: u32,
    pairs: u32,
    snd: Option<Snd>,
    dirty: bool,
}

impl Memory {
    pub fn new(t: &Trng) -> Self {
        let mut cards = [0u8; 12];
        for i in 0..12 {
            cards[i] = (i / 2) as u8;
        }
        for i in (1..12).rev() {
            cards.swap(i, rnd(t, i as u32 + 1) as usize);
        }
        Memory { cards, state: [0; 12], cur: 0, first: None, hide_at: None, moves: 0, pairs: 0, snd: None, dirty: true }
    }
}

impl Game for Memory {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.pairs >= 6 {
            return false;
        }
        let (r, c) = (self.cur / 4, self.cur % 4);
        match b {
            Btn::Left => self.cur = r * 4 + (c + 3) % 4,
            Btn::Right => self.cur = r * 4 + (c + 1) % 4,
            Btn::Up => self.cur = ((r + 2) % 3) * 4 + c,
            Btn::Down => self.cur = ((r + 1) % 3) * 4 + c,
            Btn::Ok => {
                if self.hide_at.is_some() || self.state[self.cur] != 0 {
                    return false;
                }
                self.state[self.cur] = 1;
                match self.first {
                    None => {
                        self.first = Some(self.cur);
                        self.snd = Some(Snd::Click);
                    }
                    Some(f) => {
                        self.moves += 1;
                        self.first = None;
                        if self.cards[f] == self.cards[self.cur] {
                            self.state[f] = 2;
                            self.state[self.cur] = 2;
                            self.pairs += 1;
                            self.snd = Some(Snd::Coin);
                        } else {
                            self.hide_at = Some(Instant::now() + Duration::from_millis(800));
                            self.snd = Some(Snd::Tick);
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if let Some(at) = self.hide_at {
            if Instant::now() >= at {
                self.hide_at = None;
                for s in self.state.iter_mut() {
                    if *s == 1 {
                        *s = 0;
                    }
                }
                return true;
            }
        }
        let d = self.dirty;
        self.dirty = false;
        d
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("moves {} pairs {}/6", self.moves, self.pairs));
        for i in 0..12usize {
            let (x, y) = (4 + (i as i32 % 4) * 30, 14 + (i as i32 / 4) * 16);
            rc(c, x, y, 28, 14);
            if self.state[i] == 0 {
                for yy in 0..12 {
                    for xx in 0..26 {
                        if (xx + yy) % 3 == 0 {
                            c.dot(x + 1 + xx, y + 1 + yy, true);
                        }
                    }
                }
            } else {
                let s = ((b'A' + self.cards[i].min(25)) as char).to_string();
                txt(c, x + 11, y + 1, &s);
                if self.state[i] == 2 {
                    c.invert_rect((x + 1) as usize, (y + 1) as usize, 26, 12);
                }
            }
            if i == self.cur {
                rc(c, x - 1, y - 1, 30, 16);
            }
        }
        if self.pairs >= 6 {
            over_box(c, "ALL FOUND");
        }
    }
    fn score(&self) -> u32 { if self.pairs >= 6 { (200u32.saturating_sub(self.moves * 8)).max(20) } else { self.pairs * 10 } }
    fn over(&self) -> bool { self.pairs >= 6 }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 6 Whack-a-Mole
pub struct Whack {
    cur: usize,
    mole: Option<(usize, i32)>, // hole, ms left up
    spawn_in: i32,
    time_left: i32,
    score: u32,
    misses: u32,
    alive: bool,
    paused: bool,
    last: Instant,
    last_sec: i32,
    snd: Option<Snd>,
    dirty: bool,
}

impl Whack {
    pub fn new(_t: &Trng) -> Self {
        Whack {
            cur: 4,
            mole: None,
            spawn_in: 700,
            time_left: 30_000,
            score: 0,
            misses: 0,
            alive: true,
            paused: false,
            last: Instant::now(),
            last_sec: 30,
            snd: None,
            dirty: true,
        }
    }
}

impl Game for Whack {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.alive {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        if !self.alive {
            return false;
        }
        let (r, c) = (self.cur / 3, self.cur % 3);
        match b {
            Btn::Left => self.cur = r * 3 + (c + 2) % 3,
            Btn::Right => self.cur = r * 3 + (c + 1) % 3,
            Btn::Up => self.cur = ((r + 2) % 3) * 3 + c,
            Btn::Down => self.cur = ((r + 1) % 3) * 3 + c,
            Btn::Ok => {
                match self.mole {
                    Some((h, _)) if h == self.cur => {
                        self.score += 1;
                        self.mole = None;
                        self.spawn_in = 150 + rnd(t, 350) as i32;
                        self.snd = Some(Snd::Coin);
                    }
                    _ => {
                        self.misses += 1;
                        self.snd = Some(Snd::Tick);
                    }
                }
            }
            _ => return false,
        }
        true
    }
    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = dt_ms(&mut self.last) as i32;
        self.time_left -= dt;
        let mut changed = false;
        if self.time_left <= 0 {
            self.time_left = 0;
            self.alive = false;
            self.snd = Some(Snd::Crash);
            return true;
        }
        let sec = (self.time_left + 999) / 1000;
        if sec != self.last_sec {
            self.last_sec = sec;
            changed = true;
        }
        match self.mole {
            Some((h, left)) => {
                let left = left - dt;
                if left <= 0 {
                    self.mole = None;
                    self.spawn_in = 150 + rnd(t, 350) as i32;
                } else {
                    self.mole = Some((h, left));
                }
                changed |= left <= 0;
            }
            None => {
                self.spawn_in -= dt;
                if self.spawn_in <= 0 {
                    let up = (1000 - self.score as i32 * 25).max(450);
                    self.mole = Some((rnd(t, 9) as usize, up));
                    changed = true;
                }
            }
        }
        changed
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("{} hit  {}s", self.score, (self.time_left + 999) / 1000));
        for i in 0..9usize {
            let (x, y) = (34 + (i as i32 % 3) * 22, 14 + (i as i32 / 3) * 16);
            // the hole
            for dx in -8..=8i32 {
                let h = Canvas::dome_h(8, 3, dx);
                c.dot(x + 10 + dx, y + 13 - h, true);
                c.dot(x + 10 + dx, y + 13 + h.min(2), true);
            }
            if let Some((h, _)) = self.mole {
                if h == i {
                    disc(c, x + 10, y + 7, 5);
                    c.dot(x + 8, y + 6, false);
                    c.dot(x + 12, y + 6, false);
                    fr(c, x + 9, y + 9, 3, 1, false);
                }
            }
            if i == self.cur {
                rc(c, x, y - 1, 21, 15);
            }
        }
        if !self.alive {
            over_box(c, "TIME UP");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 7 Minesweeper (12x6, 10 mines)
const MW: i32 = 12;
const MH: i32 = 6;
const MINES: usize = 10;
const DIGITS: [[u8; 5]; 9] = [
    [0, 0, 0, 0, 0],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
];

pub struct Mines {
    mine: [bool; 72],
    near: [u8; 72],
    st: [u8; 72], // 0 covered, 1 open, 2 flag
    placed: bool,
    cx: i32,
    cy: i32,
    ok_down: bool,
    ok_since: Instant,
    ok_fired: bool,
    want_reveal: bool,
    result: u8, // 0 playing, 1 won, 2 lost
    revealed: u32,
    dirty: bool,
    snd: Option<Snd>,
}

impl Mines {
    pub fn new() -> Self {
        Mines {
            mine: [false; 72],
            near: [0; 72],
            st: [0; 72],
            placed: false,
            cx: 5,
            cy: 2,
            ok_down: false,
            ok_since: Instant::now(),
            ok_fired: false,
            want_reveal: false,
            result: 0,
            revealed: 0,
            dirty: true,
            snd: None,
        }
    }
    fn idx(x: i32, y: i32) -> Option<usize> {
        if x >= 0 && x < MW && y >= 0 && y < MH { Some((y * MW + x) as usize) } else { None }
    }
    fn place(&mut self, t: &Trng) {
        let mut n = 0;
        let mut tries = 0;
        while n < MINES && tries < 5000 {
            tries += 1;
            let i = rnd(t, 72) as usize;
            let (x, y) = (i as i32 % MW, i as i32 / MW);
            if self.mine[i] || ((x - self.cx).abs() <= 1 && (y - self.cy).abs() <= 1) {
                continue;
            }
            self.mine[i] = true;
            n += 1;
        }
        for y in 0..MH {
            for x in 0..MW {
                let mut k = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if let Some(j) = Mines::idx(x + dx, y + dy) {
                            if self.mine[j] {
                                k += 1;
                            }
                        }
                    }
                }
                if let Some(i) = Mines::idx(x, y) {
                    self.near[i] = k;
                }
            }
        }
        self.placed = true;
    }
    fn reveal(&mut self, t: &Trng) {
        if !self.placed {
            self.place(t);
        }
        let Some(start) = Mines::idx(self.cx, self.cy) else { return };
        if self.st[start] != 0 {
            return;
        }
        if self.mine[start] {
            self.result = 2;
            self.snd = Some(Snd::Crash);
            return;
        }
        let mut stack = vec![(self.cx, self.cy)];
        while let Some((x, y)) = stack.pop() {
            let Some(i) = Mines::idx(x, y) else { continue };
            if self.st[i] != 0 || self.mine[i] {
                continue;
            }
            self.st[i] = 1;
            self.revealed += 1;
            if self.near[i] == 0 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        stack.push((x + dx, y + dy));
                    }
                }
            }
        }
        self.snd = Some(Snd::Click);
        if self.revealed as usize >= 72 - MINES {
            self.result = 1;
            self.snd = Some(Snd::Coin);
        }
    }
    fn flag(&mut self) {
        if let Some(i) = Mines::idx(self.cx, self.cy) {
            match self.st[i] {
                0 => self.st[i] = 2,
                2 => self.st[i] = 0,
                _ => {}
            }
            self.snd = Some(Snd::Tick);
        }
    }
}

impl Game for Mines {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.result != 0 {
            return false;
        }
        match b {
            Btn::Left => self.cx = (self.cx + MW - 1) % MW,
            Btn::Right => self.cx = (self.cx + 1) % MW,
            Btn::Up => self.cy = (self.cy + MH - 1) % MH,
            Btn::Down => self.cy = (self.cy + 1) % MH,
            Btn::Ok => {
                // reveal happens when OK is released; holding it for 0.45 s flags the cell instead
                self.ok_down = true;
                self.ok_since = Instant::now();
                self.ok_fired = false;
                return false;
            }
            _ => return false,
        }
        true
    }
    fn held(&mut self, down: &[bool; 6]) {
        if !self.ok_down || self.result != 0 {
            return;
        }
        if !down[0] {
            self.ok_down = false;
            if !self.ok_fired {
                // the actual reveal needs the TRNG for the first click: it is done in update()
                self.ok_fired = true;
                self.want_reveal = true;
            }
        } else if !self.ok_fired && self.ok_since.elapsed() >= Duration::from_millis(450) {
            self.ok_fired = true;
            self.flag();
            self.dirty = true;
        }
    }
    fn update(&mut self, t: &Trng) -> bool {
        if self.want_reveal {
            self.want_reveal = false;
            self.reveal(t);
            self.dirty = true;
        }
        let d = self.dirty;
        self.dirty = false;
        d
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        let flags = self.st.iter().filter(|&&s| s == 2).count() as i32;
        txt(c, 0, 0, &format!("mines {}", MINES as i32 - flags));
        let (x0, y0) = (4, 14);
        for y in 0..MH {
            for x in 0..MW {
                let Some(i) = Mines::idx(x, y) else { continue };
                let (px, py) = (x0 + x * 10, y0 + y * 8);
                let lost_mine = self.result == 2 && self.mine[i];
                match self.st[i] {
                    1 => {
                        let d = DIGITS[(self.near[i] as usize).min(8)];
                        for (r, bits) in d.iter().enumerate() {
                            for col in 0..3 {
                                if bits & (0b100 >> col) != 0 {
                                    c.dot(px + 3 + col, py + 1 + r as i32, true);
                                }
                            }
                        }
                    }
                    2 => {
                        fr(c, px + 4, py + 1, 1, 5, true);
                        fr(c, px + 5, py + 1, 2, 2, true);
                    }
                    _ => {
                        if lost_mine {
                            fr(c, px + 3, py + 2, 4, 4, true);
                            c.dot(px + 5, py + 1, true);
                            c.dot(px + 5, py + 6, true);
                        } else if !(x == self.cx && y == self.cy && self.result == 0) {
                            for yy in 0..7 {
                                for xx in 0..9 {
                                    if (xx + yy) % 2 == 0 {
                                        c.dot(px + xx, py + yy, true);
                                    }
                                }
                            }
                        }
                    }
                }
                if x == self.cx && y == self.cy && self.result == 0 {
                    // the selected cell becomes a solid light block (content in dark) with a frame around it
                    c.invert_rect(px as usize, py as usize, 9, 7);
                    rc(c, px - 1, py - 1, 11, 9);
                }
            }
        }
        match self.result {
            1 => over_box(c, "CLEARED"),
            2 => {
                c.fill_rect(30, 0, 98, 12, false);
                txt(c, 30, 0, "BOOM!  OK = retry");
            }
            _ => {}
        }
    }
    fn score(&self) -> u32 { self.revealed * 5 + if self.result == 1 { 200 } else { 0 } }
    fn over(&self) -> bool { self.result != 0 }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 8 Space Shooter
const E_ROWS: usize = 3;
const E_COLS: usize = 6;

pub struct Shooter {
    x: i32,
    shots: Vec<(i32, i32)>,
    bombs: Vec<(i32, i32)>,
    alive_e: [bool; E_ROWS * E_COLS],
    fx: i32,
    fy: i32,
    fdir: i32,
    mv: u32,
    cd: u32,
    lives: i32,
    score: u32,
    wave: u32,
    alive: bool,
    paused: bool,
    dir: i32,
    last: Instant,
    acc: u32,
    snd: Option<Snd>,
}

impl Shooter {
    pub fn new() -> Self {
        Shooter {
            x: 64,
            shots: Vec::new(),
            bombs: Vec::new(),
            alive_e: [true; E_ROWS * E_COLS],
            fx: 10,
            fy: 14,
            fdir: 1,
            mv: 0,
            cd: 0,
            lives: 3,
            score: 0,
            wave: 1,
            alive: true,
            paused: false,
            dir: 0,
            last: Instant::now(),
            acc: 0,
            snd: None,
        }
    }
    fn epos(&self, i: usize) -> (i32, i32) { (self.fx + (i % E_COLS) as i32 * 17, self.fy + (i / E_COLS) as i32 * 9) }
    fn step(&mut self, t: &Trng) {
        self.x = (self.x + self.dir * 2).clamp(4, 123);
        self.cd = self.cd.saturating_sub(1);
        for s in self.shots.iter_mut() {
            s.1 -= 3;
        }
        self.shots.retain(|s| s.1 > 11);
        for b in self.bombs.iter_mut() {
            b.1 += 2;
        }
        self.bombs.retain(|b| b.1 < 64);
        // the formation marches sideways and steps down at the edges
        self.mv += 1;
        if self.mv >= 6u32.saturating_sub(self.wave).max(2) {
            self.mv = 0;
            self.fx += self.fdir * 2;
            if self.fx < 0 || self.fx > 128 - (E_COLS as i32 * 17) + 6 {
                self.fdir = -self.fdir;
                self.fx = self.fx.clamp(0, 128 - (E_COLS as i32 * 17) + 6);
                self.fy += 4;
            }
        }
        // enemy bombs
        let alive_ids: Vec<usize> = (0..E_ROWS * E_COLS).filter(|&i| self.alive_e[i]).collect();
        if !alive_ids.is_empty() && rnd(t, (45 - self.wave.min(30) * 3).max(12)) == 0 {
            let i = alive_ids[rnd(t, alive_ids.len() as u32) as usize];
            let (ex, ey) = self.epos(i);
            self.bombs.push((ex + 3, ey + 7));
        }
        // player shots vs enemies
        let mut shots = std::mem::take(&mut self.shots);
        shots.retain(|&(sx, sy)| {
            for i in 0..E_ROWS * E_COLS {
                if self.alive_e[i] {
                    let (ex, ey) = self.epos(i);
                    if sx >= ex && sx <= ex + 7 && sy >= ey && sy <= ey + 6 {
                        self.alive_e[i] = false;
                        self.score += 10;
                        self.snd = Some(Snd::Coin);
                        return false;
                    }
                }
            }
            true
        });
        self.shots = shots;
        // bombs vs ship
        let (x, mut hit) = (self.x, false);
        self.bombs.retain(|&(bx, by)| {
            if by >= 56 && by <= 62 && bx >= x - 3 && bx <= x + 3 {
                hit = true;
                false
            } else {
                true
            }
        });
        if hit {
            self.lives -= 1;
            self.bombs.clear();
            self.snd = Some(Snd::Crash);
            if self.lives <= 0 {
                self.alive = false;
            }
        }
        // invaders reaching the ground, or a cleared wave
        if (0..E_ROWS * E_COLS).any(|i| self.alive_e[i] && self.epos(i).1 + 6 >= 55) {
            self.alive = false;
            self.snd = Some(Snd::Crash);
        }
        if alive_ids.iter().all(|&i| !self.alive_e[i]) {
            self.wave += 1;
            self.alive_e = [true; E_ROWS * E_COLS];
            self.fx = 10;
            self.fy = 14;
            self.shots.clear();
            self.bombs.clear();
            self.snd = Some(Snd::Coin);
        }
    }
}

impl Game for Shooter {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.alive {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        if b == Btn::Ok && self.alive && self.cd == 0 && self.shots.len() < 4 {
            self.shots.push((self.x, 55));
            self.cd = 6;
            self.snd = Some(Snd::Tick);
            return true;
        }
        false
    }
    fn held(&mut self, down: &[bool; 6]) { self.dir = (down[4] as i32) - (down[3] as i32); }
    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = dt_ms(&mut self.last);
        let n = steps(&mut self.acc, dt, 30);
        for _ in 0..n {
            self.step(t);
            if !self.alive {
                break;
            }
        }
        n > 0
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("S{} L{} W{}", self.score, self.lives.max(0), self.wave));
        for i in 0..E_ROWS * E_COLS {
            if self.alive_e[i] {
                let (ex, ey) = self.epos(i);
                fr(c, ex + 1, ey + 1, 5, 3, true);
                fr(c, ex, ey + 3, 7, 1, true);
                let legs = ((ex / 2) & 1) == 0;
                fr(c, ex + if legs { 0 } else { 1 }, ey + 4, 1, 2, true);
                fr(c, ex + if legs { 6 } else { 5 }, ey + 4, 1, 2, true);
                c.dot(ex + 2, ey + 2, false);
                c.dot(ex + 4, ey + 2, false);
            }
        }
        for &(sx, sy) in &self.shots {
            fr(c, sx, sy, 1, 3, true);
        }
        for &(bx, by) in &self.bombs {
            fr(c, bx, by, 1, 3, true);
            c.dot(bx - 1, by + 1, true);
            c.dot(bx + 1, by + 1, true);
        }
        fr(c, self.x - 3, 60, 7, 3, true);
        fr(c, self.x - 1, 57, 3, 3, true);
        c.dot(self.x, 56, true);
        if !self.alive {
            over_box(c, "GAME OVER");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 9 Lunar Lander
pub struct Lander {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    fuel: f32,
    ground: [i32; 17], // terrain height (y) at x = 0, 8, 16 ... 128
    pad: usize,        // first pad segment, it spans 3 segments
    result: u8,        // 0 flying, 1 landed, 2 crashed
    paused: bool,
    thrust: bool,
    left: bool,
    right: bool,
    last: Instant,
    acc: u32,
    snd: Option<Snd>,
}

impl Lander {
    pub fn new(t: &Trng) -> Self {
        let pad = 1 + rnd(t, 12) as usize; // 1..=12, spans pad..pad+2 (<= 14)
        let mut ground = [58i32; 17];
        for (i, g) in ground.iter_mut().enumerate() {
            *g = 56 - rnd(t, 7) as i32;
            if i >= pad && i <= pad + 3 {
                *g = 58;
            }
        }
        Lander {
            x: 20.0 + rnd(t, 88) as f32,
            y: 16.0,
            vx: rnd(t, 21) as f32 - 10.0,
            vy: 0.0,
            fuel: 100.0,
            ground,
            pad,
            result: 0,
            paused: false,
            thrust: false,
            left: false,
            right: false,
            last: Instant::now(),
            acc: 0,
            snd: None,
        }
    }
    fn ground_at(&self, x: f32) -> i32 {
        let i = ((x.max(0.0) / 8.0) as usize).min(15);
        let f = (x - i as f32 * 8.0).clamp(0.0, 8.0) / 8.0;
        (self.ground[i] as f32 * (1.0 - f) + self.ground[i + 1] as f32 * f) as i32
    }
}

impl Game for Lander {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.result == 0 {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        false
    }
    fn held(&mut self, down: &[bool; 6]) {
        self.thrust = down[0];
        self.left = down[3];
        self.right = down[4];
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if self.result != 0 || self.paused {
            return false;
        }
        let dt = dt_ms(&mut self.last);
        let n = steps(&mut self.acc, dt, 20);
        for _ in 0..n {
            let s = 0.02f32;
            let mut ay = 9.0f32; // gravity
            let mut ax = 0.0f32;
            if self.fuel > 0.0 {
                if self.thrust {
                    ay -= 24.0;
                    self.fuel -= 22.0 * s;
                }
                if self.left {
                    ax -= 14.0;
                    self.fuel -= 6.0 * s;
                }
                if self.right {
                    ax += 14.0;
                    self.fuel -= 6.0 * s;
                }
            }
            self.fuel = self.fuel.max(0.0);
            self.vx += ax * s;
            self.vy += ay * s;
            self.x += self.vx * s;
            self.y += self.vy * s;
            if self.x < 3.0 {
                self.x = 3.0;
                self.vx = 0.0;
            }
            if self.x > 124.0 {
                self.x = 124.0;
                self.vx = 0.0;
            }
            if self.y < 13.0 {
                self.y = 13.0;
                self.vy = self.vy.max(0.0);
            }
            let gy = self.ground_at(self.x);
            if self.y + 3.0 >= gy as f32 {
                let on_pad = self.x >= (self.pad * 8 + 2) as f32 && self.x <= ((self.pad + 3) * 8 - 2) as f32;
                if on_pad && self.vy <= 22.0 && self.vx.abs() <= 10.0 {
                    self.result = 1;
                    self.snd = Some(Snd::Coin);
                } else {
                    self.result = 2;
                    self.snd = Some(Snd::Crash);
                }
                self.y = gy as f32 - 3.0;
                break;
            }
        }
        n > 0
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("F{} vy{} vx{}", self.fuel as i32, self.vy as i32, self.vx as i32));
        for i in 0..16usize {
            c.line(i as i32 * 8, self.ground[i], (i as i32 + 1) * 8, self.ground[i + 1]);
        }
        let px = self.pad as i32 * 8;
        fr(c, px, 58, 24, 2, true);
        c.dot(px, 56, true);
        c.dot(px, 55, true);
        c.dot(px + 24, 56, true);
        c.dot(px + 24, 55, true);
        let (x, y) = (self.x as i32, self.y as i32);
        fr(c, x - 3, y - 3, 7, 4, true);
        c.dot(x - 1, y - 2, false);
        c.line(x - 3, y + 1, x - 5, y + 3);
        c.line(x + 3, y + 1, x + 5, y + 3);
        if self.result == 0 && self.fuel > 0.0 {
            if self.thrust {
                fr(c, x - 1, y + 1, 3, 4, true);
            }
            if self.left {
                c.dot(x + 4, y - 1, true);
            }
            if self.right {
                c.dot(x - 4, y - 1, true);
            }
        }
        match self.result {
            1 => over_box(c, "LANDED!"),
            2 => over_box(c, "CRASHED"),
            _ => {}
        }
    }
    fn score(&self) -> u32 { if self.result == 1 { 100 + self.fuel as u32 * 2 } else { 0 } }
    fn over(&self) -> bool { self.result != 0 }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 10 Maze (three levels, each bigger)
static MAZE_LEVEL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// (columns, rows, pixels per cell) per level; all fit inside 128 x 52.
const MAZE_SIZES: [(usize, usize, usize); 3] = [(31, 13, 4), (41, 17, 3), (61, 25, 2)];

pub struct Maze {
    wall: Vec<bool>,
    w: usize,
    h: usize,
    cs: i32,
    level: usize,
    px: i32,
    py: i32,
    elapsed: u32, // ms
    won: bool,
    paused: bool,
    last: Instant,
    last_sec: u32,
    rep_at: Instant,
    dirty: bool,
    snd: Option<Snd>,
}

impl Maze {
    pub fn new(t: &Trng) -> Self {
        let level = MAZE_LEVEL.load(std::sync::atomic::Ordering::Relaxed) % MAZE_SIZES.len();
        let (w, h, cs) = MAZE_SIZES[level];
        let mut wall = vec![true; w * h];
        // iterative depth-first search over the odd cells
        let mut stack: Vec<(usize, usize)> = vec![(1, 1)];
        wall[w + 1] = false;
        while let Some(&(x, y)) = stack.last() {
            let mut opts: Vec<(usize, usize, usize, usize)> = Vec::new();
            for (dx, dy) in [(2i32, 0i32), (-2, 0), (0, 2), (0, -2)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx > 0 && ny > 0 && (nx as usize) < w - 1 && (ny as usize) < h - 1 && wall[ny as usize * w + nx as usize] {
                    opts.push((nx as usize, ny as usize, (x as i32 + dx / 2) as usize, (y as i32 + dy / 2) as usize));
                }
            }
            if opts.is_empty() {
                stack.pop();
            } else {
                let (nx, ny, wx, wy) = opts[rnd(t, opts.len() as u32) as usize];
                wall[wy * w + wx] = false;
                wall[ny * w + nx] = false;
                stack.push((nx, ny));
            }
        }
        Maze {
            wall,
            w,
            h,
            cs: cs as i32,
            level,
            px: 1,
            py: 1,
            elapsed: 0,
            won: false,
            paused: false,
            last: Instant::now(),
            last_sec: 0,
            rep_at: Instant::now(),
            dirty: true,
            snd: None,
        }
    }
    fn walk(&mut self, dx: i32, dy: i32) -> bool {
        let (nx, ny) = (self.px + dx, self.py + dy);
        if nx < 0 || ny < 0 || nx >= self.w as i32 || ny >= self.h as i32 || self.wall[ny as usize * self.w + nx as usize] {
            return false;
        }
        self.px = nx;
        self.py = ny;
        if nx == self.w as i32 - 2 && ny == self.h as i32 - 2 {
            self.won = true;
            self.snd = Some(Snd::Coin);
            // OK after winning starts the next, bigger maze (after the last one it starts over with the first)
            MAZE_LEVEL.store((self.level + 1) % MAZE_SIZES.len(), std::sync::atomic::Ordering::Relaxed);
        }
        true
    }
}

impl Game for Maze {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if !self.won {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        if self.won {
            return false;
        }
        self.rep_at = Instant::now() + Duration::from_millis(220);
        match b {
            Btn::Left => self.walk(-1, 0),
            Btn::Right => self.walk(1, 0),
            Btn::Up => self.walk(0, -1),
            Btn::Down => self.walk(0, 1),
            _ => false,
        }
    }
    fn held(&mut self, down: &[bool; 6]) {
        if self.won || self.paused || Instant::now() < self.rep_at {
            return;
        }
        let (dx, dy) = ((down[4] as i32) - (down[3] as i32), (down[2] as i32) - (down[1] as i32));
        if (dx != 0) != (dy != 0) {
            if self.walk(dx, dy) {
                self.dirty = true;
            }
            self.rep_at = Instant::now() + Duration::from_millis(if self.level == 2 { 70 } else { 110 });
        }
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if self.won || self.paused {
            return false;
        }
        self.elapsed += dt_ms(&mut self.last);
        let sec = self.elapsed / 1000;
        let mut changed = self.dirty || self.elapsed / 250 != (self.elapsed.saturating_sub(20)) / 250;
        self.dirty = false;
        if sec != self.last_sec {
            self.last_sec = sec;
            changed = true;
        }
        changed
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("Maze {}/3  {}s", self.level + 1, self.elapsed / 1000));
        let cs = self.cs;
        for y in 0..self.h {
            for x in 0..self.w {
                if self.wall[y * self.w + x] {
                    fr(c, 2 + x as i32 * cs, 12 + y as i32 * cs, cs, cs, true);
                }
            }
        }
        // the exit blinks, the runner is a small square
        let (gx, gy) = (2 + (self.w as i32 - 2) * cs, 12 + (self.h as i32 - 2) * cs);
        if (self.elapsed / 300) % 2 == 0 {
            if cs >= 4 { rc(c, gx, gy, cs, cs) } else { fr(c, gx, gy, cs, cs, true) }
        }
        if cs >= 4 {
            fr(c, 2 + self.px * cs + 1, 12 + self.py * cs + 1, cs - 2, cs - 2, true);
        } else if (self.elapsed / 150) % 2 == 0 {
            fr(c, 2 + self.px * cs, 12 + self.py * cs, cs, cs, true);
        }
        if self.won {
            over_box(c, "ESCAPED!");
        }
    }
    fn score(&self) -> u32 { if self.won { (600u32 + self.level as u32 * 300).saturating_sub((self.elapsed / 1000) * 5).max(10) } else { 0 } }
    fn over(&self) -> bool { self.won }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 11 Tower (stack city blocks, like Tower Bloxx)
const TOWER_BH: i32 = 6; // block height in pixels
const TOWER_TOP: i32 = 40; // screen y of the top of the highest block
const TOWER_SWING_Y: i32 = 18;

pub struct Tower {
    blocks: Vec<(i32, i32)>, // (left x, width) of the newest blocks, oldest first
    w: i32,
    t: u32, // ms of swinging, drives the crane
    phase: u8, // 0 swinging, 1 falling, 2 toppled
    fx: i32,
    fy: i32,
    fy_acc: u32,
    stacked: u32,
    combo: u32,
    score: u32,
    paused: bool,
    last: Instant,
    snd: Option<Snd>,
}

impl Tower {
    pub fn new() -> Self {
        Tower {
            blocks: vec![(44, 40)],
            w: 40,
            t: 0,
            phase: 0,
            fx: 0,
            fy: 0,
            fy_acc: 0,
            stacked: 0,
            combo: 0,
            score: 0,
            paused: false,
            last: Instant::now(),
            snd: None,
        }
    }
    fn swing_x(&self) -> i32 {
        let span = (128 - self.w).max(1) as u32;
        let ms_per_px = (9u32.saturating_sub(self.stacked / 4)).max(3); // faster as the tower grows
        let p = (self.t / ms_per_px) % (2 * span);
        (if p < span { p } else { 2 * span - p }) as i32
    }
    fn land(&mut self) {
        let (tx, tw) = *self.blocks.last().unwrap_or(&(44, 40));
        let overlap = (self.fx + self.w).min(tx + tw) - self.fx.max(tx);
        if overlap <= 0 {
            self.phase = 2;
            self.snd = Some(Snd::Crash);
            return;
        }
        let perfect = (self.fx - tx).abs() <= 2;
        if perfect {
            self.combo += 1;
            self.score += 2 + self.combo;
            self.fx = tx; // snaps into place, keeps its full width
            self.snd = Some(Snd::Coin);
        } else {
            self.combo = 0;
            self.score += 1;
            self.w = overlap;
            self.fx = self.fx.max(tx);
            self.snd = Some(Snd::Tick);
        }
        self.blocks.push((self.fx, self.w));
        if self.blocks.len() > 8 {
            self.blocks.remove(0);
        }
        self.stacked += 1;
        self.phase = 0;
    }
}

fn building(c: &mut Canvas, x: i32, y: i32, w: i32) {
    fr(c, x, y, w, TOWER_BH, false);
    rc(c, x, y, w, TOWER_BH);
    let mut wx = x + 2;
    while wx + 1 < x + w - 1 {
        c.dot(wx, y + 2, true);
        c.dot(wx + 1, y + 2, true);
        c.dot(wx, y + 3, true);
        c.dot(wx + 1, y + 3, true);
        wx += 4;
    }
}

impl Game for Tower {
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.phase != 2 {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.set_paused(false);
            }
            return true;
        }
        if b == Btn::Ok && self.phase == 0 {
            self.fx = self.swing_x();
            self.fy = TOWER_SWING_Y;
            self.fy_acc = 0;
            self.phase = 1;
            self.snd = Some(Snd::Click);
            return true;
        }
        false
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if self.phase == 2 || self.paused {
            return false;
        }
        let dt = dt_ms(&mut self.last);
        if self.phase == 0 {
            self.t += dt;
            return true;
        }
        // falling: 1 px per 8 ms
        self.fy_acc += dt;
        let step = (self.fy_acc / 8) as i32;
        self.fy_acc %= 8;
        self.fy += step;
        if self.fy >= TOWER_TOP - TOWER_BH {
            self.fy = TOWER_TOP - TOWER_BH;
            self.land();
        }
        true
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("{} blocks", self.stacked));
        if self.combo > 1 {
            txt(c, 80, 0, &format!("x{} perfect", self.combo));
        }
        // the tower: newest block on top, older ones below, ground at the bottom
        let n = self.blocks.len();
        for k in 0..4usize {
            if k < n {
                let (x, w) = self.blocks[n - 1 - k];
                building(c, x, TOWER_TOP + k as i32 * TOWER_BH, w);
            }
        }
        fr(c, 0, 63, 128, 1, true);
        match self.phase {
            0 => {
                let x = self.swing_x();
                fr(c, x + self.w / 2, 11, 1, TOWER_SWING_Y - 11, true); // the crane rope
                building(c, x, TOWER_SWING_Y, self.w);
            }
            1 => building(c, self.fx, self.fy, self.w),
            _ => {
                building(c, self.fx, self.fy.min(TOWER_TOP + 2), self.w);
                over_box(c, "TOPPLED");
            }
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { self.phase == 2 }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 12 FreeCell
// cards: 0..52, rank = c % 13 (0 = ace .. 12 = king), suit = c / 13 (0 spade, 1 heart, 2 diamond, 3 club)
const GLYPH: [[u8; 5]; 13] = [
    [0b010, 0b101, 0b111, 0b101, 0b101], // A
    [0b110, 0b001, 0b010, 0b100, 0b111], // 2
    [0b110, 0b001, 0b010, 0b001, 0b110], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b110, 0b001, 0b110], // 5
    [0b011, 0b100, 0b110, 0b101, 0b010], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b010, 0b101, 0b010, 0b101, 0b010], // 8
    [0b010, 0b101, 0b011, 0b001, 0b110], // 9
    [0b111, 0b010, 0b010, 0b010, 0b010], // T (10)
    [0b001, 0b001, 0b001, 0b101, 0b010], // J
    [0b010, 0b101, 0b101, 0b111, 0b011], // Q
    [0b101, 0b101, 0b110, 0b101, 0b101], // K
];
const SUITS: [[u8; 5]; 4] = [
    [0b00100, 0b01110, 0b11111, 0b00100, 0b01110], // spade: point, wide body, stem and foot
    [0b01010, 0b11111, 0b11111, 0b01110, 0b00100], // heart: two bumps, point down
    [0b00100, 0b01010, 0b10001, 0b01010, 0b00100], // diamond: hollow
    [0b01110, 0b01110, 0b10101, 0b11111, 0b00100], // club: three lobes, stem
];

fn is_red(c: u8) -> bool { matches!(c / 13, 1 | 2) }

fn card_face(c: &mut Canvas, x: i32, y: i32, card: u8) {
    for (r, bits) in GLYPH[(card % 13) as usize].iter().enumerate() {
        for col in 0..3 {
            if bits & (0b100 >> col) != 0 {
                c.dot(x + 1 + col, y + 1 + r as i32, true);
            }
        }
    }
    for (r, bits) in SUITS[(card / 13) as usize % 4].iter().enumerate() {
        for col in 0..5 {
            if bits & (0b10000 >> col) != 0 {
                c.dot(x + 6 + col, y + 1 + r as i32, true);
            }
        }
    }
}

pub struct FreeCell {
    col: Vec<Vec<u8>>, // 8 columns, the last card is the top; all cards are face up
    cells: [Option<u8>; 4],
    found: [u8; 4], // cards placed on each suit's foundation
    cur: i32, // 0..=3 free cells, 4..=11 columns
    depth: usize, // how many cards of the column are lifted
    hold: Option<(i32, usize)>, // (where the carried cards come from, how many)
    ok_down: bool,
    ok_since: Instant,
    ok_fired: bool,
    want_ok: bool,
    dirty: bool,
    snd: Option<Snd>,
}

const FC_PH: i32 = 12; // card height
const FC_TOP: i32 = 14; // y where the columns start

impl FreeCell {
    pub fn new(t: &Trng) -> Self {
        let mut deck: Vec<u8> = (0..52).collect();
        for i in (1..52usize).rev() {
            let j = rnd(t, i as u32 + 1) as usize;
            deck.swap(i, j);
        }
        let mut col: Vec<Vec<u8>> = vec![Vec::new(); 8];
        for (i, c) in deck.into_iter().enumerate() {
            col[i % 8].push(c);
        }
        FreeCell {
            col,
            cells: [None; 4],
            found: [0; 4],
            cur: 4,
            depth: 1,
            hold: None,
            ok_down: false,
            ok_since: Instant::now(),
            ok_fired: false,
            want_ok: false,
            dirty: true,
            snd: None,
        }
    }
    fn won(&self) -> bool { self.found.iter().map(|&n| n as u32).sum::<u32>() == 52 }
    fn free_cells(&self) -> usize { self.cells.iter().filter(|c| c.is_none()).count() }
    fn empty_cols(&self) -> usize { self.col.iter().filter(|c| c.is_empty()).count() }
    /// How many cards may be moved at once (free cells and empty columns are used as temporary space).
    fn max_move(&self, dest_empty: bool) -> usize {
        let empties = self.empty_cols().saturating_sub(if dest_empty { 1 } else { 0 });
        (self.free_cells() + 1) << empties.min(6)
    }
    /// Length of the ordered run (alternating colours, descending) at the bottom of a column.
    fn run_len(&self, c: usize) -> usize {
        let v = &self.col[c];
        let mut n = if v.is_empty() { 0 } else { 1 };
        while n < v.len() {
            let (lo, hi) = (v[v.len() - n], v[v.len() - n - 1]);
            if is_red(lo) != is_red(hi) && lo % 13 + 1 == hi % 13 {
                n += 1;
            } else {
                break;
            }
        }
        n
    }
    fn max_lift(&self, c: usize) -> usize { self.run_len(c).min(self.max_move(false)).max(1) }
    fn can_stack(&self, card: u8, c: usize) -> bool {
        match self.col[c].last() {
            None => true,
            Some(&t) => is_red(card) != is_red(t) && card % 13 + 1 == t % 13,
        }
    }
    fn top_card(&self, pos: i32) -> Option<u8> {
        if pos < 4 { self.cells[pos as usize] } else { self.col[(pos - 4) as usize].last().copied() }
    }
    fn ok_short(&mut self) {
        if let Some((src, n)) = self.hold.take() {
            let dst = self.cur;
            if dst == src {
                return; // put back
            }
            let cards: Vec<u8> = if src < 4 {
                self.cells[src as usize].into_iter().collect()
            } else {
                let sc = (src - 4) as usize;
                let l = self.col[sc].len();
                self.col[sc][l.saturating_sub(n)..].to_vec()
            };
            let ok = if cards.is_empty() {
                false
            } else if dst < 4 {
                n == 1 && self.cells[dst as usize].is_none()
            } else {
                let dc = (dst - 4) as usize;
                self.can_stack(cards[0], dc) && n <= self.max_move(self.col[dc].is_empty())
            };
            if !ok {
                self.snd = Some(Snd::Crash);
                return;
            }
            if src < 4 {
                self.cells[src as usize] = None;
            } else {
                let sc = (src - 4) as usize;
                let l = self.col[sc].len();
                self.col[sc].truncate(l.saturating_sub(n));
            }
            if dst < 4 {
                self.cells[dst as usize] = cards.first().copied();
            } else {
                self.col[(dst - 4) as usize].extend(cards);
            }
            self.depth = 1;
            self.snd = Some(Snd::Click);
            return;
        }
        if self.cur < 4 {
            if self.cells[self.cur as usize].is_some() {
                self.hold = Some((self.cur, 1));
                self.snd = Some(Snd::Tick);
            }
        } else {
            let c = (self.cur - 4) as usize;
            if !self.col[c].is_empty() {
                self.hold = Some((self.cur, self.depth.clamp(1, self.max_lift(c))));
                self.snd = Some(Snd::Tick);
            }
        }
    }
    /// OK held: the top card under the cursor goes to its foundation if it is the next one needed.
    fn ok_long(&mut self) {
        self.hold = None;
        if let Some(card) = self.top_card(self.cur) {
            let s = (card / 13) as usize % 4;
            if self.found[s] == card % 13 {
                self.found[s] += 1;
                if self.cur < 4 {
                    self.cells[self.cur as usize] = None;
                } else {
                    self.col[(self.cur - 4) as usize].pop();
                }
                self.snd = Some(if self.won() { Snd::Coin } else { Snd::Click });
                return;
            }
            self.snd = Some(Snd::Crash);
        }
    }
}

impl Game for FreeCell {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.won() {
            return false;
        }
        match b {
            Btn::Left => {
                self.cur = (self.cur + 11) % 12;
                self.depth = 1;
            }
            Btn::Right => {
                self.cur = (self.cur + 1) % 12;
                self.depth = 1;
            }
            Btn::Up | Btn::Down => {
                // jump between the free cells and the columns, or (on a column, not carrying) lift more / fewer cards
                if self.hold.is_none() && self.cur >= 4 {
                    let c = (self.cur - 4) as usize;
                    if b == Btn::Up {
                        if self.depth >= self.max_lift(c) {
                            self.cur = ((self.cur - 4).min(3)).max(0); // past the top: go to the free cell above
                            self.depth = 1;
                        } else {
                            self.depth += 1;
                        }
                    } else {
                        self.depth = self.depth.saturating_sub(1).max(1);
                    }
                } else if self.cur < 4 && b == Btn::Down {
                    self.cur += 4;
                    self.depth = 1;
                } else if self.hold.is_some() && b == Btn::Up && self.cur >= 4 {
                    self.cur = (self.cur - 4).min(3);
                }
            }
            Btn::Ok => {
                self.ok_down = true;
                self.ok_since = Instant::now();
                self.ok_fired = false;
                return false;
            }
            _ => return false,
        }
        true
    }
    fn held(&mut self, down: &[bool; 6]) {
        if !self.ok_down || self.won() {
            return;
        }
        if !down[0] {
            self.ok_down = false;
            if !self.ok_fired {
                self.ok_fired = true;
                self.want_ok = true;
            }
        } else if !self.ok_fired && self.ok_since.elapsed() >= Duration::from_millis(500) {
            self.ok_fired = true;
            self.ok_long();
            self.dirty = true;
        }
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if self.want_ok {
            self.want_ok = false;
            self.ok_short();
            self.dirty = true;
        }
        let d = self.dirty;
        self.dirty = false;
        d
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        let slot = |i: i32| i * 16;
        // free cells (slots 0..3) and foundations (slots 4..7)
        for i in 0..4i32 {
            let x = slot(i);
            fr(c, x, 0, 15, FC_PH, false);
            rc(c, x, 0, 15, FC_PH);
            if let Some(card) = self.cells[i as usize] {
                card_face(c, x, 0, card);
            }
            if self.cur == i {
                c.invert_rect(x as usize, 0, 15, FC_PH as usize);
            }
        }
        for s in 0..4usize {
            let x = slot(4 + s as i32);
            rc(c, x, 0, 15, FC_PH);
            if self.found[s] > 0 {
                card_face(c, x, 0, (s as u8) * 13 + self.found[s] - 1);
            } else {
                for (r, bits) in SUITS[s].iter().enumerate() {
                    for k in 0..5 {
                        if bits & (0b10000 >> k) != 0 && (k + r as i32) % 2 == 0 {
                            c.dot(x + 5 + k, 3 + r as i32, true);
                        }
                    }
                }
            }
        }
        // the eight columns
        for ci in 0..8usize {
            let x = slot(ci as i32);
            let n = self.col[ci].len();
            if n == 0 {
                rc(c, x, FC_TOP, 15, FC_PH);
                if self.cur == 4 + ci as i32 {
                    c.invert_rect(x as usize, FC_TOP as usize, 15, FC_PH as usize);
                }
                continue;
            }
            let step = if n > 1 { ((64 - FC_TOP - FC_PH) / (n as i32 - 1)).clamp(3, 8) } else { 8 };
            for (k, &card) in self.col[ci].iter().enumerate() {
                let y = FC_TOP + k as i32 * step;
                fr(c, x, y, 15, FC_PH, false);
                rc(c, x, y, 15, FC_PH);
                card_face(c, x, y, card);
            }
            let lifted = match self.hold {
                Some((src, cnt)) if src == 4 + ci as i32 => Some(cnt),
                None if self.cur == 4 + ci as i32 => Some(self.depth.clamp(1, self.max_lift(ci))),
                _ => None,
            };
            if let Some(cnt) = lifted {
                let first = n - cnt.min(n);
                let y0 = FC_TOP + first as i32 * step;
                let y1 = (FC_TOP + (n as i32 - 1) * step + FC_PH).min(64);
                if y1 > y0 {
                    c.invert_rect(x as usize, y0.max(0) as usize, 15, (y1 - y0) as usize);
                }
            }
            // target marker while carrying: a small arrow above the column
            if self.hold.is_some() && self.cur == 4 + ci as i32 {
                fr(c, x + 4, FC_TOP - 2, 7, 1, true);
                fr(c, x + 5, FC_TOP - 1, 5, 1, true);
            }
        }
        if self.hold.is_some() {
            if self.cur >= 4 && self.col[(self.cur - 4) as usize].is_empty() {
                let x = slot(self.cur - 4);
                fr(c, x + 4, FC_TOP - 2, 7, 1, true);
            } else if self.cur < 4 {
                rc(c, slot(self.cur) - 1, -1, 17, FC_PH + 2);
            }
        }
        if self.won() {
            over_box(c, "YOU WON!");
        }
    }
    fn score(&self) -> u32 { self.found.iter().map(|&n| n as u32).sum::<u32>() * 10 + if self.won() { 200 } else { 0 } }
    fn over(&self) -> bool { self.won() }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}
