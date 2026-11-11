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

pub const NAMES: [&str; 16] = [
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
    "Solitaire",
    "Jetpack Joyride",
    "Road Rush",
    "Subway Run",
];

pub const DESCS: [&str; 16] = [
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
    "Klondike, draw one",
    "Fly, dodge, vehicles",
    "3D bike race, nitro",
    "Dodge trains, coins",
];

pub const HELP: [[&str; 4]; 16] = [
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
    ["<> pile  ^v lift more", "OK lift / OK drop", "HOLD OK: send to top", "down red/black, A-K"],
    ["hold OK = fly up", "dodge zaps, missiles", "S shield M magnet", "B bike C crush G grav"],
    ["<> steer, DOWN brake", "OK / UP = nitro boost", "grab cans for nitro", "cars cost a life"],
    ["<> change lane", "UP jump, DOWN slide", "jump low barriers", "slide under beams"],
];

/// 8x8 menu icons, one byte per row, most significant bit = leftmost pixel.
pub const ICONS: [[u8; 8]; 16] = [
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
    [0x7E, 0x81, 0x99, 0xBD, 0xBD, 0x99, 0x81, 0x7E], // Solitaire
    [0x18, 0x3C, 0x24, 0x7E, 0x5A, 0x18, 0x24, 0x66], // Jetpack
    [0x00, 0x18, 0x3C, 0x5A, 0x18, 0x24, 0x42, 0x81], // Road Rush
    [0x00, 0x42, 0x42, 0x5A, 0x5A, 0x5A, 0x42, 0xFF], // Subway Run
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
        12 => Box::new(Solitaire::new(t)),
        13 => Box::new(Jetpack::new()),
        14 => Box::new(RoadRush::new()),
        _ => Box::new(Subway::new()),
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

fn over_box(c: &mut Canvas, msg: &str) { over_box2(c, msg, "OK retry  BACK exit"); }

fn over_box2(c: &mut Canvas, msg: &str, hint: &str) {
    c.fill_rect(6, 16, 116, 42, false);
    c.rect(6, 16, 116, 42);
    c.text_big(12, 19, msg, 2);
    c.text(12, 44, hint);
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
            over_box2(c, "ESCAPED!", if self.level + 1 < MAZE_SIZES.len() { "OK = next maze" } else { "OK = maze 1 again" });
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

// ---------------------------------------------------------------- 12 Solitaire (Klondike, draw one)
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

pub struct Solitaire {
    tab: Vec<Vec<u8>>,
    up: [usize; 7], // index of the first face-up card in each column
    stock: Vec<u8>,
    waste: Vec<u8>,
    found: [u8; 4],
    cur: i32, // 0 stock, 1 waste, 2..=8 columns
    depth: usize, // how many face-up cards of the column are lifted
    hold: Option<(i32, usize)>, // (source cursor position, number of cards) being carried
    ok_down: bool,
    ok_since: Instant,
    ok_fired: bool,
    want_ok: bool,
    dirty: bool,
    moves: u32,
    snd: Option<Snd>,
}

impl Solitaire {
    pub fn new(t: &Trng) -> Self {
        let mut deck: Vec<u8> = (0..52).collect();
        for i in (1..52usize).rev() {
            let j = rnd(t, i as u32 + 1) as usize;
            deck.swap(i, j);
        }
        let mut tab: Vec<Vec<u8>> = Vec::new();
        let mut up = [0usize; 7];
        for col in 0..7 {
            let mut v = Vec::new();
            for _ in 0..=col {
                v.push(deck.pop().unwrap_or(0));
            }
            up[col] = col;
            tab.push(v);
        }
        Solitaire {
            tab,
            up,
            stock: deck,
            waste: Vec::new(),
            found: [0; 4],
            cur: 0,
            depth: 1,
            hold: None,
            ok_down: false,
            ok_since: Instant::now(),
            ok_fired: false,
            want_ok: false,
            dirty: true,
            moves: 0,
            snd: None,
        }
    }
    fn col_of(pos: i32) -> Option<usize> { if (2..=8).contains(&pos) { Some((pos - 2) as usize) } else { None } }
    fn faceup(&self, col: usize) -> usize { self.tab[col].len().saturating_sub(self.up[col]) }
    fn won(&self) -> bool { self.found.iter().map(|&n| n as u32).sum::<u32>() == 52 }
    fn fix(&mut self, col: usize) {
        if !self.tab[col].is_empty() && self.up[col] >= self.tab[col].len() {
            self.up[col] = self.tab[col].len() - 1; // flip the new top card
        }
        if self.tab[col].is_empty() {
            self.up[col] = 0;
        }
    }
    fn can_found(&self, card: u8) -> bool { self.found[(card / 13) as usize % 4] == card % 13 }
    fn can_stack(&self, card: u8, col: usize) -> bool {
        match self.tab[col].last() {
            None => card % 13 == 12,
            Some(&t) => is_red(card) != is_red(t) && card % 13 + 1 == t % 13,
        }
    }
    fn top_of(&self, pos: i32) -> Option<u8> {
        match pos {
            1 => self.waste.last().copied(),
            p => Solitaire::col_of(p).and_then(|c| self.tab[c].last().copied()),
        }
    }
    /// OK released after a short press.
    fn ok_short(&mut self) {
        if let Some((src, n)) = self.hold.take() {
            // drop the carried cards on the column under the cursor
            if let Some(dst) = Solitaire::col_of(self.cur) {
                let cards: Vec<u8> = if src == 1 {
                    self.waste.last().copied().into_iter().collect()
                } else {
                    let sc = (src - 2) as usize;
                    let l = self.tab[sc].len();
                    self.tab[sc][l.saturating_sub(n)..].to_vec()
                };
                if src != self.cur && !cards.is_empty() && self.can_stack(cards[0], dst) {
                    if src == 1 {
                        self.waste.pop();
                    } else {
                        let sc = (src - 2) as usize;
                        let l = self.tab[sc].len();
                        self.tab[sc].truncate(l.saturating_sub(n));
                        self.fix(sc);
                    }
                    self.tab[dst].extend(cards);
                    self.fix(dst);
                    self.moves += 1;
                    self.snd = Some(Snd::Click);
                    self.depth = 1;
                    return;
                }
                if src != self.cur {
                    self.snd = Some(Snd::Crash);
                }
            }
            return;
        }
        match self.cur {
            0 => {
                if let Some(c) = self.stock.pop() {
                    self.waste.push(c);
                } else {
                    while let Some(c) = self.waste.pop() {
                        self.stock.push(c); // recycle the waste face down
                    }
                }
                self.snd = Some(Snd::Tick);
            }
            1 => {
                if !self.waste.is_empty() {
                    self.hold = Some((1, 1));
                    self.snd = Some(Snd::Tick);
                }
            }
            p => {
                if let Some(col) = Solitaire::col_of(p) {
                    let n = self.depth.clamp(1, self.faceup(col).max(1));
                    if !self.tab[col].is_empty() {
                        self.hold = Some((p, n));
                        self.snd = Some(Snd::Tick);
                    }
                }
            }
        }
    }
    /// OK held: send the top card under the cursor to its foundation when that is allowed.
    fn ok_long(&mut self) {
        self.hold = None;
        if let Some(card) = self.top_of(self.cur) {
            if self.can_found(card) {
                self.found[(card / 13) as usize % 4] += 1;
                if self.cur == 1 {
                    self.waste.pop();
                } else if let Some(col) = Solitaire::col_of(self.cur) {
                    self.tab[col].pop();
                    self.fix(col);
                }
                self.moves += 1;
                self.snd = Some(if self.won() { Snd::Coin } else { Snd::Click });
            } else {
                self.snd = Some(Snd::Crash);
            }
        }
    }
}

impl Game for Solitaire {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.won() {
            return false;
        }
        match b {
            Btn::Left | Btn::Right => {
                let dir = if b == Btn::Right { 1 } else { -1 };
                let lo = if self.hold.is_some() { 2 } else { 0 };
                let n = 9 - lo;
                self.cur = lo + (self.cur - lo + dir + n).rem_euclid(n);
                self.depth = 1;
            }
            Btn::Up | Btn::Down => {
                if self.hold.is_none() {
                    if let Some(col) = Solitaire::col_of(self.cur) {
                        let fu = self.faceup(col).max(1);
                        if b == Btn::Up {
                            self.depth = (self.depth + 1).min(fu);
                        } else {
                            self.depth = self.depth.saturating_sub(1).max(1);
                        }
                    }
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
        let ph = 14; // card height
        let slot = |i: i32| 1 + i * 18;
        // stock (face-down pattern), waste (top card), and the four foundations in slots 3..6
        rc(c, slot(0), 0, 16, ph);
        if !self.stock.is_empty() {
            for yy in 1..ph - 1 {
                for xx in 1..15 {
                    if (xx + yy) % 2 == 0 {
                        c.dot(slot(0) + xx, yy, true);
                    }
                }
            }
        } else if !self.waste.is_empty() {
            txt(c, slot(0) + 5, 1, "o");
        }
        if let Some(&w) = self.waste.last() {
            fr(c, slot(1), 0, 16, ph, false);
            rc(c, slot(1), 0, 16, ph);
            card_face(c, slot(1), 0, w);
        }
        for s in 0..4usize {
            let x = slot(3 + s as i32);
            rc(c, x, 0, 16, ph);
            if self.found[s] > 0 {
                card_face(c, x, 0, (s as u8) * 13 + self.found[s] - 1);
            } else {
                for (r, bits) in SUITS[s].iter().enumerate() {
                    for col in 0..5 {
                        if bits & (0b10000 >> col) != 0 && (col + r as i32) % 2 == 0 {
                            c.dot(x + 5 + col, 4 + r as i32, true);
                        }
                    }
                }
            }
        }
        // the tableau
        let top = 16;
        for col in 0..7usize {
            let x = slot(col as i32);
            let n = self.tab[col].len();
            if n == 0 {
                rc(c, x, top, 16, ph);
                continue;
            }
            let upn = self.up[col].min(n);
            let fu = n - upn;
            let fd_step = 3;
            let budget = 64 - top - ph - upn as i32 * fd_step;
            let step = if fu > 1 { (budget / (fu as i32 - 1)).clamp(4, 8) } else { 8 };
            // y position of every card
            let mut ys: Vec<i32> = Vec::with_capacity(n);
            let mut y = top;
            for k in 0..n {
                ys.push(y);
                y += if k < upn { fd_step } else { step };
            }
            for (k, &card) in self.tab[col].iter().enumerate() {
                fr(c, x, ys[k], 16, ph, false);
                rc(c, x, ys[k], 16, ph);
                if k < upn {
                    for xx in 1..15 {
                        if xx % 2 == 0 {
                            c.dot(x + xx, ys[k] + 1, true);
                        }
                    }
                } else {
                    card_face(c, x, ys[k], card);
                }
            }
            // highlight the lifted (or about to be lifted) cards of this column
            let lifted = match self.hold {
                Some((src, cnt)) if src == col as i32 + 2 => Some(cnt),
                None if self.cur == col as i32 + 2 => Some(self.depth.clamp(1, fu.max(1))),
                _ => None,
            };
            if let Some(cnt) = lifted {
                let first = n - cnt.min(n);
                let y0 = ys[first];
                let y1 = (ys[n - 1] + ph).min(64);
                if y1 > y0 {
                    c.invert_rect(x as usize, y0.max(0) as usize, 16, (y1 - y0) as usize);
                }
            }
        }
        // cursor on stock / waste, or the target column while carrying
        if self.cur < 2 {
            rc(c, slot(self.cur) - 1, 0, 18, ph + 1);
        } else if self.hold.is_some() {
            let x = slot(self.cur - 2);
            rc(c, x - 1, top - 2, 18, 3);
        }
        if self.won() {
            over_box(c, "YOU WON!");
        }
    }
    fn score(&self) -> u32 { self.found.iter().map(|&n| n as u32).sum::<u32>() * 10 + if self.won() { 200 } else { 0 } }
    fn over(&self) -> bool { self.won() }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 13 Jetpack Joyride (side scroller: hold OK to fly)
const JP_TOP: i32 = 11;
const JP_BOT: i32 = 62;

/// 8 x 10 pixel hero: flying, and two running frames. Bit 7 is the leftmost pixel.
const HERO: [[u8; 10]; 3] = [
    [0b00011100, 0b00111110, 0b00110100, 0b00011100, 0b11111100, 0b11111100, 0b11111100, 0b00111000, 0b00110100, 0b00100100],
    [0b00011100, 0b00111110, 0b00110100, 0b00011100, 0b11111100, 0b11111100, 0b11111100, 0b00111000, 0b00101000, 0b01000100],
    [0b00011100, 0b00111110, 0b00110100, 0b00011100, 0b11111100, 0b11111100, 0b11111100, 0b00111000, 0b00010000, 0b00111000],
];
/// 5 x 8 scientist in a white coat (two walking frames).
const SCI: [[u8; 8]; 2] = [
    [0b01110, 0b01010, 0b11111, 0b11111, 0b01110, 0b01110, 0b01010, 0b01010],
    [0b01110, 0b01010, 0b11111, 0b11111, 0b01110, 0b01110, 0b00100, 0b01010],
];

fn sprite8(c: &mut Canvas, x: i32, y: i32, rows: &[u8], w: i32) {
    for (r, bits) in rows.iter().enumerate() {
        for col in 0..w {
            if bits & (0x80 >> col) != 0 {
                c.dot(x + col, y + r as i32, true);
            }
        }
    }
}

/// 3 x 5 letters for the pick-up boxes.
fn letter(c: &mut Canvas, x: i32, y: i32, ch: char) {
    let rows: [u8; 5] = match ch {
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        _ => [0b110, 0b001, 0b010, 0b100, 0b111], // 2
    };
    for (r, bits) in rows.iter().enumerate() {
        for col in 0..3 {
            if bits & (0b100 >> col) != 0 {
                c.dot(x + col, y + r as i32, true);
            }
        }
    }
}

struct JObj {
    kind: u8, // 0 coin, 1 zapper, 2 missile warning, 3 shield, 4 double coins, 5 missile, 6 vehicle box, 7 magnet, 8 scientist
    sub: u8,  // vehicle box: 1 bike, 2 crusher, 3 gravity suit
    x: f32,
    y: f32,
    w: i32,
    h: i32,
    t: f32,
}

pub struct Jetpack {
    y: f32,
    vy: f32,
    thrust: bool,
    was_thrust: bool,
    objs: Vec<JObj>,
    dist: f32,
    coins: u32,
    shield: f32,
    dbl: f32,
    magnet: f32,
    veh: u8, // 0 on foot (jetpack), 1 Bad As Hog bike, 2 Crusher, 3 Gravity suit
    veh_t: f32,
    grav_up: bool,
    inv: f32,
    sci: u32,
    gap: f32,
    alive: bool,
    paused: bool,
    last: Instant,
    clock: u32,
    snd: Option<Snd>,
}

impl Jetpack {
    pub fn new() -> Self {
        Jetpack {
            y: 30.0,
            vy: 0.0,
            thrust: false,
            was_thrust: false,
            objs: Vec::new(),
            dist: 0.0,
            coins: 0,
            shield: 0.0,
            dbl: 0.0,
            magnet: 0.0,
            veh: 0,
            veh_t: 0.0,
            grav_up: false,
            inv: 0.0,
            sci: 0,
            gap: 40.0,
            alive: true,
            paused: false,
            last: Instant::now(),
            clock: 0,
            snd: None,
        }
    }
    fn speed(&self) -> f32 { (55.0 + self.dist / 45.0).min(125.0) }
    /// (width, height) of the player's body in the current mode.
    fn size(&self) -> (i32, i32) {
        match self.veh {
            1 => (14, 12),
            2 => (14, 15),
            _ => (6, 10),
        }
    }
    fn on_floor_mode(&self) -> bool { self.veh == 1 || self.veh == 2 }
    fn spawn(&mut self, t: &Trng) {
        let x0 = 134.0;
        match rnd(t, 10) {
            0..=3 => {
                // a row of coins, sometimes a wave
                let n = 4 + rnd(t, 4) as i32;
                let base = 18 + rnd(t, 34) as i32;
                let wave = rnd(t, 2) == 0;
                for i in 0..n {
                    let dy = if wave { ((i as f32 * 0.9).sin() * 7.0) as i32 } else { 0 };
                    self.objs.push(JObj { kind: 0, sub: 0, x: x0 + i as f32 * 9.0, y: (base + dy) as f32, w: 5, h: 5, t: 0.0 });
                }
                self.gap = 70.0 + n as f32 * 9.0;
            }
            4..=6 => {
                if rnd(t, 2) == 0 {
                    let len = 18 + rnd(t, 16) as i32;
                    self.objs.push(JObj { kind: 1, sub: 0, x: x0, y: (14 + rnd(t, 36)) as f32, w: len, h: 3, t: 0.0 });
                } else {
                    let len = 16 + rnd(t, 18) as i32;
                    let y = JP_TOP + 2 + rnd(t, (JP_BOT - JP_TOP - len - 4).max(1) as u32) as i32;
                    self.objs.push(JObj { kind: 1, sub: 0, x: x0, y: y as f32, w: 3, h: len, t: 0.0 });
                }
                self.gap = 60.0 + rnd(t, 50) as f32;
            }
            7 | 8 => {
                // a missile: a warning shows first, then it flies in fast
                self.objs.push(JObj { kind: 2, sub: 0, x: 119.0, y: (14 + rnd(t, 40)) as f32, w: 7, h: 9, t: 1.0 });
                self.gap = 80.0;
            }
            _ => {
                let (kind, sub) = match rnd(t, 7) {
                    0 => (3, 0),
                    1 => (4, 0),
                    2 => (7, 0),
                    3 | 4 => (6, 1),
                    5 => (6, 2),
                    _ => (6, 3),
                };
                // vehicles need room to land: bike and crusher boxes float low
                let y = if kind == 6 && sub != 3 { (JP_BOT - 28) as f32 } else { (16 + rnd(t, 34)) as f32 };
                self.objs.push(JObj { kind, sub, x: x0, y, w: 9, h: 9, t: 0.0 });
                self.gap = 90.0;
            }
        }
        // scientists walk about on the floor
        if rnd(t, 4) == 0 {
            let n = 2 + rnd(t, 2) as i32;
            for i in 0..n {
                self.objs.push(JObj { kind: 8, sub: 0, x: x0 + 30.0 + i as f32 * 11.0, y: (JP_BOT - 7) as f32, w: 5, h: 8, t: 0.0 });
            }
        }
    }
    fn mount(&mut self, v: u8) {
        self.veh = v;
        self.veh_t = 14.0;
        self.inv = 0.6;
        self.vy = 0.0;
        self.grav_up = false;
        self.snd = Some(Snd::Click);
        let (_, h) = self.size();
        if self.on_floor_mode() {
            self.y = (JP_BOT - h + 1) as f32;
        }
    }
    fn dismount(&mut self) {
        self.veh = 0;
        self.inv = 1.2;
        self.vy = -40.0;
        self.snd = Some(Snd::Crash);
    }
}

impl Game for Jetpack {
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
        if self.veh == 3 && (b == Btn::Ok || b == Btn::Up) {
            self.grav_up = !self.grav_up; // gravity suit: every press flips gravity
            self.vy = 0.0;
            self.snd = Some(Snd::Tick);
            return true;
        }
        false
    }
    fn held(&mut self, down: &[bool; 6]) { self.thrust = down[0] || down[1]; }
    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = (dt_ms(&mut self.last) as f32 / 1000.0).min(0.06);
        self.clock += (dt * 1000.0) as u32;
        let sp = self.speed();
        self.dist += sp * dt;
        let (pw, ph) = self.size();
        let (lo, hi) = ((JP_TOP + 1) as f32, (JP_BOT - ph + 1) as f32);
        match self.veh {
            1 | 2 => {
                // ground vehicles: heavy gravity, OK hops
                self.vy += 420.0 * dt;
                if self.thrust && !self.was_thrust && self.y >= hi - 0.5 {
                    self.vy = if self.veh == 1 { -150.0 } else { -125.0 };
                    self.snd = Some(Snd::Tick);
                }
            }
            3 => {
                self.vy += if self.grav_up { -260.0 } else { 260.0 } * dt;
            }
            _ => {
                // jetpack: hold to climb, let go to fall
                if self.thrust {
                    self.vy -= 340.0 * dt;
                } else {
                    self.vy += 180.0 * dt;
                }
            }
        }
        self.was_thrust = self.thrust;
        self.vy = self.vy.clamp(-150.0, 150.0);
        self.y += self.vy * dt;
        if self.y < lo {
            self.y = lo;
            self.vy = self.vy.max(0.0);
        }
        if self.y > hi {
            self.y = hi;
            self.vy = self.vy.min(0.0);
        }
        if self.veh != 0 {
            self.veh_t -= dt;
            if self.veh_t <= 0.0 {
                self.veh = 0;
                self.inv = 0.8;
            }
        }
        self.shield = (self.shield - dt).max(0.0);
        self.dbl = (self.dbl - dt).max(0.0);
        self.magnet = (self.magnet - dt).max(0.0);
        self.inv = (self.inv - dt).max(0.0);
        self.gap -= sp * dt;
        if self.gap <= 0.0 && self.objs.len() < 26 {
            self.spawn(t);
        }
        let (px, py) = (if self.on_floor_mode() { 14 } else { 20 }, self.y as i32);
        let mut dead = false;
        let mut i = 0;
        let mut mount: Option<u8> = None;
        while i < self.objs.len() {
            let magnet = self.magnet > 0.0;
            let o = &mut self.objs[i];
            match o.kind {
                2 => {
                    o.t -= dt;
                    if o.t <= 0.0 {
                        o.kind = 5;
                        o.x = 128.0;
                        o.w = 9;
                        o.h = 3;
                        o.y += 3.0;
                    }
                }
                5 => o.x -= (sp + 110.0) * dt,
                8 => o.x -= (sp - 14.0) * dt, // scientists stroll along with the floor
                0 if magnet && o.x < 70.0 => {
                    // the magnet pulls coins in
                    let (dx, dy) = ((px + 3) as f32 - o.x, (py + 4) as f32 - o.y);
                    let d = (dx * dx + dy * dy).sqrt().max(1.0);
                    o.x += dx / d * 150.0 * dt - sp * dt;
                    o.y += dy / d * 150.0 * dt;
                }
                _ => o.x -= sp * dt,
            }
            let (ox, oy) = (o.x as i32, o.y as i32);
            let hit = ox < px + pw && ox + o.w > px && oy < py + ph && oy + o.h > py;
            let mut remove = o.x + (o.w as f32) < -2.0;
            if hit {
                match o.kind {
                    0 => {
                        self.coins += if self.dbl > 0.0 { 2 } else { 1 };
                        self.snd = Some(Snd::Coin);
                        remove = true;
                    }
                    3 => {
                        self.shield = 9.0;
                        self.snd = Some(Snd::Click);
                        remove = true;
                    }
                    4 => {
                        self.dbl = 10.0;
                        self.snd = Some(Snd::Click);
                        remove = true;
                    }
                    7 => {
                        self.magnet = 10.0;
                        self.snd = Some(Snd::Click);
                        remove = true;
                    }
                    6 => {
                        mount = Some(o.sub);
                        remove = true;
                    }
                    8 => {
                        self.sci += 1;
                        self.coins += 2;
                        self.snd = Some(Snd::Tick);
                        remove = true;
                    }
                    1 | 5 => {
                        if self.veh == 2 {
                            self.coins += 3; // the crusher smashes it
                            self.snd = Some(Snd::Crash);
                            remove = true;
                        } else if self.inv > 0.0 {
                            // blinking after a crash: you fly through
                        } else if self.veh != 0 {
                            remove = true;
                            self.dismount(); // the vehicle takes the hit and breaks
                        } else if self.shield > 0.0 {
                            self.shield = 0.0;
                            self.inv = 0.6;
                            self.snd = Some(Snd::Crash);
                            remove = true;
                        } else {
                            dead = true;
                        }
                    }
                    _ => {}
                }
            }
            if remove {
                self.objs.swap_remove(i);
            } else {
                i += 1;
            }
        }
        if let Some(v) = mount {
            self.mount(v);
        }
        if dead {
            self.alive = false;
            self.snd = Some(Snd::Crash);
        }
        true
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("{}m {}c", (self.dist / 10.0) as u32, self.coins));
        let label = if self.veh != 0 {
            ["", "BIKE", "CRUSHER", "GRAVITY"][self.veh as usize]
        } else if self.shield > 0.0 {
            "SHIELD"
        } else if self.magnet > 0.0 {
            "MAGNET"
        } else if self.dbl > 0.0 {
            "x2 COINS"
        } else {
            ""
        };
        txt(c, 80, 0, label);
        fr(c, 0, JP_TOP - 1, 128, 1, true);
        fr(c, 0, JP_BOT + 1, 128, 1, true);
        // lab windows and far dots slide by: you can feel the speed
        for i in 0..5i32 {
            let x = 127 - ((self.dist as i32 * 2 / 3 + i * 31) % 128);
            rc(c, x, JP_TOP + 6 + (i * 13) % 22, 6, 4);
        }
        for i in 0..6i32 {
            c.dot(127 - ((self.dist as i32 + i * 47) % 128), JP_TOP + 3 + (i * 19) % 44, true);
        }
        for o in &self.objs {
            let (x, y) = (o.x as i32, o.y as i32);
            match o.kind {
                0 => {
                    rc(c, x, y, 5, 5);
                    c.dot(x + 2, y + 2, true);
                }
                1 => {
                    let ph = (self.clock / 70) as i32;
                    if o.w > o.h {
                        fr(c, x, y, 3, 3, true);
                        fr(c, x + o.w - 3, y, 3, 3, true);
                        let mut xx = x + 3;
                        while xx < x + o.w - 3 {
                            c.dot(xx, y + (xx / 2 + ph) % 3, true);
                            xx += 1;
                        }
                    } else {
                        fr(c, x, y, 3, 3, true);
                        fr(c, x, y + o.h - 3, 3, 3, true);
                        let mut yy = y + 3;
                        while yy < y + o.h - 3 {
                            c.dot(x + (yy / 2 + ph) % 3, yy, true);
                            yy += 1;
                        }
                    }
                }
                2 => {
                    if (self.clock / 120) % 2 == 0 {
                        txt(c, x, y - 2, "!");
                        c.dot(x + 6, y + 4, true);
                    }
                }
                3 | 4 | 6 | 7 => {
                    fr(c, x, y, 9, 9, false);
                    rc(c, x, y, 9, 9);
                    let ch = match (o.kind, o.sub) {
                        (3, _) => 'S',
                        (4, _) => '2',
                        (7, _) => 'M',
                        (_, 1) => 'B',
                        (_, 2) => 'C',
                        _ => 'G',
                    };
                    letter(c, x + 3, y + 2, ch);
                    if o.kind == 6 && (self.clock / 200) % 2 == 0 {
                        c.dot(x - 1, y - 1, true);
                        c.dot(x + 9, y + 9, true); // vehicle boxes sparkle
                    }
                }
                8 => {
                    let f = ((self.clock / 140) % 2) as usize;
                    fr(c, x, y, 5, 8, false);
                    sprite8(c, x, y, &SCI[f], 5);
                    c.dot(x + 1, y + 1, false); // glasses
                }
                _ => {
                    fr(c, x, y, 7, 3, true);
                    c.dot(x - 1, y + 1, true);
                    fr(c, x + 7, y - 1 + (self.clock as i32 / 50) % 2, 3, 2, true);
                }
            }
        }
        // the player: hero on a jetpack, bike, crusher or gravity suit
        let py = self.y as i32;
        let blink = self.inv > 0.0 && (self.clock / 80) % 2 == 0;
        if !blink {
            match self.veh {
                1 => {
                    // Bad As Hog: rider on a chopper with two wheels
                    let spin = (self.clock / 60) as i32 % 2;
                    rc(c, 14, py + 7, 5, 5);
                    rc(c, 25, py + 7, 5, 5);
                    c.dot(16 + spin, py + 9, true);
                    c.dot(27 - spin, py + 9, true);
                    c.line(16, py + 8, 22, py + 5);
                    c.line(22, py + 5, 27, py + 8);
                    fr(c, 20, py + 4, 5, 2, true);
                    sprite8(c, 20, py - 5, &HERO[0][..8], 8);
                }
                2 => {
                    // Crusher: a walking mech with a fist
                    let st = (self.clock / 110) as i32 % 2;
                    fr(c, 14, py, 14, 9, true);
                    fr(c, 16, py + 2, 4, 3, false);
                    fr(c, 22, py + 2, 4, 3, false);
                    c.dot(18, py + 3, true);
                    c.dot(24, py + 3, true);
                    fr(c, 15, py + 9, 4, 6 - st, true);
                    fr(c, 23, py + 9, 4, 5 + st, true);
                    fr(c, 28, py + 3, 3, 4, true);
                    fr(c, 12, py + 4, 2, 3, true);
                }
                _ => {
                    let f = if self.thrust || self.veh == 3 || py < JP_BOT - 10 { 0 } else { 1 + ((self.clock / 110) % 2) as usize };
                    sprite8(c, 19, py, &HERO[f], 8);
                    if self.thrust && self.veh == 0 {
                        fr(c, 18, py + 7, 2, 2 + (self.clock as i32 / 60) % 3, true);
                        c.dot(19, py + 11, true);
                    }
                    if self.veh == 3 {
                        rc(c, 16, py - 2, 14, 14);
                        // arrow: where gravity pulls
                        let (ay, d) = if self.grav_up { (py - 5, -1) } else { (py + 14, 1) };
                        c.line(22, ay, 24, ay + d * 2);
                        c.line(26, ay, 24, ay + d * 2);
                    }
                }
            }
        }
        if self.shield > 0.0 && (self.shield > 2.0 || (self.clock / 120) % 2 == 0) {
            let (w, h) = self.size();
            rc(c, 15, py - 2, w + 8, h + 4);
        }
        if !self.alive {
            over_box(c, "CRASHED");
        }
    }
    fn score(&self) -> u32 { (self.dist / 10.0) as u32 + self.coins * 5 + self.sci * 10 }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 14 Road Rush (pseudo 3D bike race)
const RR_HY: i32 = 22; // horizon row
const RR_NEAR: i32 = 63; // bottom row of the road

struct Car {
    d: f32, // metres ahead of you
    lane: f32, // -0.8 ..= 0.8 across the road
    kind: u8, // 0 car, 1 nitro can
}

pub struct RoadRush {
    px: f32,       // your sideways position, -1 ..= 1 is the road
    speed: f32,    // metres per second
    pos: f32,      // distance driven
    curve: f32,
    target: f32,
    until: f32,    // seconds until the next bend is chosen
    nitro: f32,    // seconds of boost left
    lives: i32,
    cars: Vec<Car>,
    steer: i32,
    boost: bool,
    brake: bool,
    inv: f32,
    spawn_in: f32,
    alive: bool,
    paused: bool,
    last: Instant,
    clock: u32,
    snd: Option<Snd>,
}

impl RoadRush {
    pub fn new() -> Self {
        RoadRush {
            px: 0.0,
            speed: 20.0,
            pos: 0.0,
            curve: 0.0,
            target: 0.0,
            until: 3.0,
            nitro: 2.0,
            lives: 3,
            cars: Vec::new(),
            steer: 0,
            boost: false,
            brake: false,
            inv: 0.0,
            spawn_in: 1.0,
            alive: true,
            paused: false,
            last: Instant::now(),
            clock: 0,
            snd: None,
        }
    }
    /// Screen row for a distance ahead (0 = at your wheels).
    fn row(d: f32) -> f32 { RR_HY as f32 + (RR_NEAR - RR_HY) as f32 / (1.0 + d / 12.0) }
    fn t_of(d: f32) -> f32 { 1.0 / (1.0 + d / 12.0) }
    fn centre(&self, t: f32) -> f32 { 64.0 + self.curve * 46.0 * (1.0 - t) * (1.0 - t) - self.px * (4.0 + 52.0 * t) }
}

impl Game for RoadRush {
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
        false
    }
    fn held(&mut self, down: &[bool; 6]) {
        self.steer = (down[4] as i32) - (down[3] as i32);
        self.boost = down[0] || down[1];
        self.brake = down[2];
    }
    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = (dt_ms(&mut self.last) as f32 / 1000.0).min(0.06);
        self.clock += (dt * 1000.0) as u32;
        // speed: builds up by itself, nitro adds a lot, brake slows, grass slows
        let nitro_on = self.boost && self.nitro > 0.0;
        let top = if nitro_on { 80.0 } else { 52.0 + (self.pos / 400.0).min(14.0) };
        let off = self.px.abs() > 1.05;
        let top = if off { top.min(24.0) } else { top };
        let acc = if self.brake { -40.0 } else if self.speed < top { 26.0 } else { -22.0 };
        self.speed = (self.speed + acc * dt).clamp(8.0, 90.0);
        if nitro_on {
            self.nitro = (self.nitro - dt).max(0.0);
        }
        self.pos += self.speed * dt;
        // bends
        self.until -= dt;
        if self.until <= 0.0 {
            self.until = 3.0 + rnd(t, 40) as f32 / 10.0;
            self.target = (rnd(t, 21) as f32 - 10.0) / 10.0;
        }
        self.curve += (self.target - self.curve) * dt * 0.7;
        // steering; bends push you outwards
        self.px += self.steer as f32 * 1.5 * dt * (0.5 + self.speed / 80.0) + self.curve * self.speed * dt * 0.009;
        self.px = self.px.clamp(-1.4, 1.4);
        self.inv = (self.inv - dt).max(0.0);
        // traffic and nitro cans
        self.spawn_in -= dt;
        if self.spawn_in <= 0.0 && self.cars.len() < 10 {
            let kind = if rnd(t, 7) == 0 { 1 } else { 0 };
            self.cars.push(Car { d: 150.0, lane: (rnd(t, 17) as f32 - 8.0) / 10.0, kind });
            self.spawn_in = 0.5 + rnd(t, 12) as f32 / 10.0 * (60.0 / self.speed.max(30.0));
        }
        let mut crashed = false;
        let mut i = 0;
        while i < self.cars.len() {
            let c = &mut self.cars[i];
            // cars drive away at 20 m/s, so you close in at (speed - 20); cans stand still
            c.d -= (self.speed - if c.kind == 0 { 20.0 } else { 0.0 }) * dt;
            let mut remove = c.d < -6.0;
            if c.d < 1.6 && c.d > -1.0 && (c.lane - self.px).abs() < 0.3 {
                if c.kind == 1 {
                    self.nitro = (self.nitro + 1.5).min(4.0);
                    self.snd = Some(Snd::Coin);
                    remove = true;
                } else if self.inv <= 0.0 {
                    crashed = true;
                    remove = true;
                }
            }
            if remove {
                self.cars.swap_remove(i);
            } else {
                i += 1;
            }
        }
        if crashed {
            self.lives -= 1;
            self.speed = 12.0;
            self.inv = 1.6;
            self.snd = Some(Snd::Crash);
            if self.lives <= 0 {
                self.alive = false;
            }
        }
        true
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("{}m {}km/h", (self.pos / 1.0) as u32, (self.speed * 3.0) as u32));
        for l in 0..self.lives.max(0) {
            fr(c, 120 - l * 7, 1, 5, 5, true);
        }
        // nitro meter
        rc(c, 0, 12, 22, 5);
        fr(c, 1, 13, (20.0 * self.nitro / 4.0) as i32, 3, true);
        // horizon with mountains that slide with the bends
        fr(c, 0, RR_HY, 128, 1, true);
        let off = (self.curve * 30.0) as i32;
        let mut x = 0;
        while x < 128 {
            let h = 2 + (((x + off + 200) / 6) * 7 % 5);
            c.dot(x, RR_HY - h, true);
            c.line(x, RR_HY - h, x + 3, RR_HY - 2 - ((x + off + 203) / 6 * 7 % 5));
            x += 3;
        }
        // the road, row by row
        for y in (RR_HY + 1)..=RR_NEAR {
            let tt = (y - RR_HY) as f32 / (RR_NEAR - RR_HY) as f32; // 0 far .. 1 near
            let d = 12.0 * (1.0 / tt - 1.0);
            let hw = 4.0 + 52.0 * tt;
            let cx = self.centre(tt);
            let band = (((self.pos + d) / 8.0) as i32) & 1;
            let (l, r) = ((cx - hw) as i32, (cx + hw) as i32);
            // kerbs: striped red-white look as long and short dashes
            if band == 0 {
                fr(c, l - 2, y, 3, 1, true);
                fr(c, r, y, 3, 1, true);
            } else {
                c.dot(l, y, true);
                c.dot(r, y, true);
            }
            if band == 0 && tt > 0.08 {
                c.dot(cx as i32, y, true); // lane marking
            }
            // grass: sparse dots, denser close to you
            if (y + band as i32) % 3 == 0 {
                let mut gx = (l - 4).min(l - 2);
                while gx > 0 {
                    c.dot(gx, y, true);
                    gx -= 9;
                }
                let mut gx = r + 4;
                while gx < 128 {
                    c.dot(gx, y, true);
                    gx += 9;
                }
            }
        }
        // traffic and cans, far ones first
        let mut order: Vec<usize> = (0..self.cars.len()).collect();
        order.sort_by(|a, b| self.cars[*b].d.partial_cmp(&self.cars[*a].d).unwrap_or(std::cmp::Ordering::Equal));
        for i in order {
            let car = &self.cars[i];
            if car.d < 0.0 {
                continue;
            }
            let tt = RoadRush::t_of(car.d);
            let y = RoadRush::row(car.d) as i32;
            let cx = self.centre(tt) + car.lane * (4.0 + 52.0 * tt);
            let w = ((14.0 * tt) as i32).max(2);
            let h = ((9.0 * tt) as i32).max(2);
            let x = cx as i32 - w / 2;
            if car.kind == 0 {
                fr(c, x, y - h, w, h, false);
                rc(c, x, y - h, w, h);
                if w >= 6 {
                    fr(c, x + 1, y - h + 1, w - 2, (h / 3).max(1), true); // rear window
                    fr(c, x, y - 1, 1, 1, true);
                }
            } else {
                rc(c, x, y - h, w.max(4), h.max(4));
                c.dot(x + w.max(4) / 2, y - h + h.max(4) / 2, true);
            }
        }
        // your bike
        let blink = self.inv > 0.0 && (self.clock / 90) % 2 == 0;
        if !blink {
            let lean = self.steer * 2;
            let (bx, by) = (62 + lean, 50);
            fr(c, bx + 1, by, 4, 6, true); // rider
            fr(c, bx, by + 6, 6, 3, true); // body
            fr(c, bx + 2, by + 9, 2, 4, true); // rear wheel
            if self.boost && self.nitro > 0.0 {
                fr(c, bx + 2, by + 13, 2, 2 + (self.clock as i32 / 50) % 3, true);
            }
        }
        if !self.alive {
            over_box(c, "WRECKED");
        }
    }
    fn score(&self) -> u32 { (self.pos / 1.0) as u32 }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- 15 Subway Run (three lanes, jump and slide)
const SW_HY: i32 = 20; // horizon row
const SW_NEAR: i32 = 63;

struct SObj {
    d: f32,    // metres ahead (front of the object)
    lane: i32, // -1, 0, 1
    kind: u8,  // 0 low barrier (jump), 1 overhead beam (slide), 2 train (change lane), 3 coin
}

pub struct Subway {
    lane: i32,
    lane_x: f32, // smooth sideways position, -1 ..= 1
    jump_t: f32, // seconds into a jump, < 0 = not jumping
    slide_t: f32,
    pos: f32,
    speed: f32,
    objs: Vec<SObj>,
    coins: u32,
    lives: i32,
    inv: f32,
    spawn_in: f32,
    alive: bool,
    paused: bool,
    last: Instant,
    clock: u32,
    snd: Option<Snd>,
}

const SW_JUMP_S: f32 = 0.7;

impl Subway {
    pub fn new() -> Self {
        Subway {
            lane: 0,
            lane_x: 0.0,
            jump_t: -1.0,
            slide_t: 0.0,
            pos: 0.0,
            speed: 24.0,
            objs: Vec::new(),
            coins: 0,
            lives: 2,
            inv: 0.0,
            spawn_in: 0.5,
            alive: true,
            paused: false,
            last: Instant::now(),
            clock: 0,
            snd: None,
        }
    }
    fn t_of(d: f32) -> f32 { 1.0 / (1.0 + d / 10.0) }
    fn row(d: f32) -> f32 { SW_HY as f32 + (SW_NEAR - SW_HY) as f32 / (1.0 + d / 10.0) }
    fn x_of(lane: f32, tt: f32) -> f32 { 64.0 + lane * (3.0 + 56.0 * tt) * 0.62 }
    fn height(&self) -> f32 { if self.jump_t >= 0.0 { (self.jump_t / SW_JUMP_S * std::f32::consts::PI).sin().max(0.0) } else { 0.0 } }
}

impl Game for Subway {
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
        match b {
            Btn::Left => self.lane = (self.lane - 1).max(-1),
            Btn::Right => self.lane = (self.lane + 1).min(1),
            Btn::Up | Btn::Ok => {
                if self.jump_t < 0.0 {
                    self.jump_t = 0.0;
                    self.slide_t = 0.0;
                    self.snd = Some(Snd::Tick);
                }
            }
            Btn::Down => {
                self.slide_t = 0.6;
                self.jump_t = -1.0; // sliding also cancels a jump: you drop and slide
            }
            _ => return false,
        }
        true
    }
    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let dt = (dt_ms(&mut self.last) as f32 / 1000.0).min(0.06);
        self.clock += (dt * 1000.0) as u32;
        self.speed = (24.0 + self.pos / 40.0).min(48.0);
        self.pos += self.speed * dt;
        // glide to the chosen lane
        let target = self.lane as f32;
        self.lane_x += (target - self.lane_x) * (dt * 14.0).min(1.0);
        if self.jump_t >= 0.0 {
            self.jump_t += dt;
            if self.jump_t >= SW_JUMP_S {
                self.jump_t = -1.0;
            }
        }
        self.slide_t = (self.slide_t - dt).max(0.0);
        self.inv = (self.inv - dt).max(0.0);
        // spawn: obstacles with a free lane, and coin rows
        self.spawn_in -= dt * self.speed / 24.0;
        if self.spawn_in <= 0.0 && self.objs.len() < 14 {
            self.spawn_in = 0.85 + rnd(t, 8) as f32 / 10.0;
            let free = rnd(t, 3) as i32 - 1; // this lane stays open
            match rnd(t, 6) {
                0..=1 => {
                    // barriers or beams in the two other lanes
                    for l in -1..=1 {
                        if l != free {
                            let kind = if rnd(t, 2) == 0 { 0 } else { 1 };
                            self.objs.push(SObj { d: 110.0, lane: l, kind });
                        }
                    }
                }
                2 => {
                    // a train in one lane, a barrier in another
                    let other = (free + 1 + rnd(t, 2) as i32 + 1) % 3 - 1;
                    let train_lane = (free + 2 + 1) % 3 - 1;
                    self.objs.push(SObj { d: 110.0, lane: train_lane, kind: 2 });
                    if other != train_lane && other != free {
                        self.objs.push(SObj { d: 110.0, lane: other, kind: 0 });
                    }
                }
                _ => {
                    // a row of coins in a lane
                    for i in 0..5 {
                        self.objs.push(SObj { d: 110.0 + i as f32 * 4.0, lane: free, kind: 3 });
                    }
                }
            }
        }
        let h = self.height();
        let mut hit = false;
        let mut i = 0;
        while i < self.objs.len() {
            let o = &mut self.objs[i];
            o.d -= self.speed * dt;
            let len = if o.kind == 2 { 10.0 } else { 1.0 };
            let mut remove = o.d + len < -3.0;
            let same_lane = (self.lane_x - o.lane as f32).abs() < 0.55;
            if same_lane && o.d < 1.0 && o.d + len > -0.6 {
                match o.kind {
                    3 => {
                        self.coins += 1;
                        self.snd = Some(Snd::Coin);
                        remove = true;
                    }
                    0 => {
                        if h < 0.4 && self.inv <= 0.0 {
                            hit = true;
                            remove = true;
                        }
                    }
                    1 => {
                        if self.slide_t <= 0.0 && self.inv <= 0.0 {
                            hit = true;
                            remove = true;
                        }
                    }
                    _ => {
                        if self.inv <= 0.0 && h < 0.9 {
                            hit = true;
                        }
                    }
                }
            }
            if remove {
                self.objs.swap_remove(i);
            } else {
                i += 1;
            }
        }
        if hit {
            self.lives -= 1;
            self.speed *= 0.6;
            self.inv = 1.5;
            self.snd = Some(Snd::Crash);
            if self.lives <= 0 {
                self.alive = false;
            }
        }
        true
    }
    fn draw(&self, c: &mut Canvas, _best: u32) {
        txt(c, 0, 0, &format!("{}m {}c", self.pos as u32, self.coins));
        for l in 0..self.lives.max(0) {
            fr(c, 120 - l * 7, 1, 5, 5, true);
        }
        // sky line and the tunnel walls
        fr(c, 0, SW_HY, 128, 1, true);
        for y in (SW_HY + 1)..=SW_NEAR {
            let tt = (y - SW_HY) as f32 / (SW_NEAR - SW_HY) as f32;
            let d = 10.0 * (1.0 / tt - 1.0);
            let band = (((self.pos + d) / 5.0) as i32) & 1;
            let hw = 3.0 + 56.0 * tt;
            let (l, r) = ((64.0 - hw) as i32, (64.0 + hw) as i32);
            c.dot(l, y, true);
            c.dot(r, y, true);
            // the three tracks: two rails each; sleepers on every other band
            for lane in -1..=1 {
                let cx = 64.0 + lane as f32 * hw * 0.62;
                let rail = hw * 0.12;
                c.dot((cx - rail) as i32, y, true);
                c.dot((cx + rail) as i32, y, true);
                if band == 0 {
                    let (a, b) = ((cx - rail) as i32, (cx + rail) as i32);
                    let mut x = a;
                    while x <= b {
                        c.dot(x, y, true);
                        x += 2;
                    }
                }
            }
        }
        // objects, far ones first
        let mut order: Vec<usize> = (0..self.objs.len()).collect();
        order.sort_by(|a, b| self.objs[*b].d.partial_cmp(&self.objs[*a].d).unwrap_or(std::cmp::Ordering::Equal));
        for i in order {
            let o = &self.objs[i];
            if o.d < -1.0 {
                continue;
            }
            let dd = o.d.max(0.0);
            let tt = Subway::t_of(dd);
            let y = Subway::row(dd) as i32;
            let cx = Subway::x_of(o.lane as f32, tt) as i32;
            let w = ((3.0 + 56.0 * tt) * 0.5) as i32;
            match o.kind {
                3 => {
                    let r = ((4.0 * tt) as i32).max(1);
                    rc(c, cx - r, y - 2 * r - 1, 2 * r + 1, 2 * r + 1);
                }
                0 => {
                    let hh = ((9.0 * tt) as i32).max(2);
                    fr(c, cx - w / 2, y - hh, w, hh, false);
                    rc(c, cx - w / 2, y - hh, w, hh);
                    fr(c, cx - w / 2, y - hh / 2, w, 1, true);
                }
                1 => {
                    let (top, hh) = (y - ((26.0 * tt) as i32).max(4), ((4.0 * tt) as i32).max(2));
                    fr(c, cx - w / 2, top, w, hh, true);
                    fr(c, cx - w / 2, top, 1, ((26.0 * tt) as i32).max(4), true); // posts
                    fr(c, cx + w / 2 - 1, top, 1, ((26.0 * tt) as i32).max(4), true);
                }
                _ => {
                    // a train: front face, roof lines running back
                    let hh = ((30.0 * tt) as i32).max(4);
                    let (back_t, back_y) = (Subway::t_of(dd + 10.0), Subway::row(dd + 10.0) as i32);
                    let (bw, bh) = (((3.0 + 56.0 * back_t) * 0.5) as i32, ((30.0 * back_t) as i32).max(2));
                    let bcx = Subway::x_of(o.lane as f32, back_t) as i32;
                    c.line(cx - w / 2, y - hh, bcx - bw / 2, back_y - bh);
                    c.line(cx + w / 2, y - hh, bcx + bw / 2, back_y - bh);
                    fr(c, cx - w / 2, y - hh, w, hh, false);
                    rc(c, cx - w / 2, y - hh, w, hh);
                    fr(c, cx - w / 2 + 2, y - hh + 2, (w - 4).max(1), (hh / 3).max(1), true); // windscreen
                }
            }
        }
        // the runner
        let blink = self.inv > 0.0 && (self.clock / 90) % 2 == 0;
        if !blink {
            let x = Subway::x_of(self.lane_x, 1.0) as i32;
            let h = (self.height() * 16.0) as i32;
            let base = 62 - h;
            if self.slide_t > 0.0 {
                fr(c, x - 5, base - 4, 10, 3, true); // lying down
                fr(c, x + 4, base - 6, 3, 3, true);
            } else {
                let step = ((self.clock / 100) % 2) as i32;
                fr(c, x - 1, base - 14, 4, 4, true); // head
                fr(c, x - 2, base - 10, 6, 6, true); // body
                if h > 0 {
                    c.line(x - 2, base - 8, x - 5, base - 12);
                    c.line(x + 3, base - 8, x + 6, base - 12);
                    fr(c, x - 2, base - 4, 2, 3, true);
                    fr(c, x + 2, base - 4, 2, 3, true);
                } else {
                    fr(c, x - 2, base - 4, 2, 4 - step, true);
                    fr(c, x + 2, base - 4, 2, 3 + step, true);
                    c.dot(x - 3 - step, base - 8, true);
                    c.dot(x + 4 + step, base - 7, true);
                }
            }
            if h > 0 {
                c.dot(x, 63, true); // shadow
                c.dot(x + 1, 63, true);
            }
        }
        // the guard who chases you after the first mistake
        if self.lives == 1 {
            fr(c, 2, 50, 4, 4, true);
            fr(c, 1, 54, 6, 6, true);
            fr(c, 1, 60 - (self.clock as i32 / 100) % 2, 2, 3, true);
            fr(c, 5, 60 + (self.clock as i32 / 100) % 2, 2, 3, true);
        }
        if !self.alive {
            over_box(c, "CAUGHT!");
        }
    }
    fn score(&self) -> u32 { self.pos as u32 / 2 + self.coins * 5 }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}
