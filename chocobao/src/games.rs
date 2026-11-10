// games.rs - small games for the 128x64 OLED. All randomness comes from the hardware TRNG.
//
// Every game implements `Game`, so the menu code does not care which one is running:
//   press()   a button went down (OK included)         update()  advance time, returns true if redraw needed
//   held()    called every tick with the button state  draw()    paint the whole frame
//   over()    game finished (OK then restarts it)      take_sound() one sound event for the buzzer
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use bao1x_hal_service::trng::Trng;

use crate::controls::Btn;
use crate::oled::Canvas;
use crate::rng::below;

#[derive(Clone, Copy, PartialEq)]
pub enum Snd {
    Click,
    Coin,
    Crash,
    Tick,
}

pub trait Game {
    fn press(&mut self, b: Btn, trng: &Trng) -> bool;
    fn held(&mut self, _down: &[bool; 6]) {}
    fn update(&mut self, trng: &Trng) -> bool;
    fn draw(&self, c: &mut Canvas, best: u32);
    fn score(&self) -> u32;
    fn over(&self) -> bool;
    fn take_sound(&mut self) -> Option<Snd>;
    /// Reaction test: a smaller score is better.
    fn lower_is_better(&self) -> bool { false }
}

pub struct Entry {
    pub name: &'static str,
    pub make: fn(&Trng) -> Box<dyn Game>,
}

pub const GAMES: [Entry; 7] = [
    Entry { name: "Mempool Dodger", make: |_| Box::new(Dodger::new()) },
    Entry { name: "Snake", make: |t| Box::new(Snake::new(t)) },
    Entry { name: "Pong", make: |t| Box::new(Pong::new(t)) },
    Entry { name: "Flappy Bao", make: |t| Box::new(Flappy::new(t)) },
    Entry { name: "Simon", make: |t| Box::new(Simon::new(t)) },
    Entry { name: "Reaction", make: |t| Box::new(Reaction::new(t)) },
    Entry { name: "ChocoDoom", make: |t| Box::new(crate::doom::Doom::new(t)) },
];

fn game_over_box(c: &mut Canvas, msg: &str) {
    c.fill_rect(6, 16, 116, 42, false);
    c.rect(6, 16, 116, 42);
    c.text_big(12, 19, msg, 2);
    c.text(12, 44, "OK retry  BACK exit");
}

// ---------------------------------------------------------------- Mempool Dodger
const D_COLS: usize = 16; // 8-pixel cells across the screen
const D_ROWS: usize = 6;
const D_TOP: usize = 14;

pub struct Dodger {
    px: usize,
    items: Vec<(usize, usize, bool)>, // column, row, is_coin
    score: u32,
    alive: bool,
    paused: bool,
    last: Instant,
    rep_at: Instant,
    snd: Option<Snd>,
}

impl Dodger {
    pub fn new() -> Self {
        Dodger {
            px: D_COLS / 2,
            items: Vec::new(),
            score: 0,
            alive: true,
            paused: false,
            last: Instant::now(),
            rep_at: Instant::now(),
            snd: None,
        }
    }
    fn step(&mut self, dir: i32) {
        if self.alive && !self.paused {
            if dir < 0 && self.px > 0 {
                self.px -= 1;
            } else if dir > 0 && self.px + 1 < D_COLS {
                self.px += 1;
            }
            self.check();
        }
    }
    fn tick_ms(&self) -> u128 { 260u32.saturating_sub(self.score * 2).max(110) as u128 }
    fn check(&mut self) {
        let (row, px) = (D_ROWS - 1, self.px);
        let (mut hit, mut bonus) = (false, 0u32);
        self.items.retain(|&(c, r, coin)| {
            if r == row && c == px {
                if coin {
                    bonus += 5;
                } else {
                    hit = true;
                }
                false
            } else {
                true
            }
        });
        if bonus > 0 {
            self.score += bonus;
            self.snd = Some(Snd::Coin);
        }
        if hit {
            self.alive = false;
            self.snd = Some(Snd::Crash);
        }
    }
}

impl Game for Dodger {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        match b {
            Btn::Left => self.step(-1),
            Btn::Right => self.step(1),
            Btn::Ok if self.alive => {
                self.paused = !self.paused;
                self.last = Instant::now();
            }
            _ => return false,
        }
        self.rep_at = Instant::now() + Duration::from_millis(220);
        true
    }
    fn held(&mut self, down: &[bool; 6]) {
        let (l, r) = (down[3], down[4]);
        if l != r && Instant::now() >= self.rep_at {
            self.step(if l { -1 } else { 1 });
            self.rep_at = Instant::now() + Duration::from_millis(85);
        }
    }
    fn update(&mut self, trng: &Trng) -> bool {
        if !self.alive || self.paused || self.last.elapsed().as_millis() < self.tick_ms() {
            return false;
        }
        self.last = Instant::now();
        for it in self.items.iter_mut() {
            it.1 += 1;
        }
        self.items.retain(|it| it.1 < D_ROWS);
        if below(trng, 2).unwrap_or(0) == 0 {
            let col = below(trng, D_COLS as u32).unwrap_or(0) as usize;
            let coin = below(trng, 6).unwrap_or(1) == 0;
            self.items.push((col, 0, coin));
        }
        self.score += 1;
        self.check();
        true
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        c.text(0, 0, &format!("score {} best {}", self.score, best));
        for &(col, r, coin) in &self.items {
            let (x, y) = (col * 8, D_TOP + r * 8);
            if coin {
                c.rect(x + 1, y + 1, 6, 6);
                c.fill_rect(x + 3, y + 3, 2, 2, true);
            } else {
                c.fill_rect(x + 1, y + 1, 6, 6, true);
            }
        }
        let (x, y) = (self.px * 8, D_TOP + (D_ROWS - 1) * 8);
        c.fill_rect(x + 1, y + 3, 6, 5, true);
        c.fill_rect(x + 3, y + 1, 2, 2, true);
        if self.paused {
            c.fill_rect(20, 24, 88, 24, false);
            c.rect(20, 24, 88, 24);
            c.text(32, 30, "PAUSED - OK");
        }
        if !self.alive {
            game_over_box(c, "GAME OVER");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- Snake
const S_CELL: usize = 4;
const S_COLS: usize = 32;
const S_ROWS: usize = 12;
const S_TOP: usize = 14;

pub struct Snake {
    body: VecDeque<(i32, i32)>, // head at the front
    dir: (i32, i32),
    queue: VecDeque<(i32, i32)>, // buffered turns so quick taps are never lost
    food: (i32, i32),
    score: u32,
    alive: bool,
    paused: bool,
    last: Instant,
    snd: Option<Snd>,
}

impl Snake {
    pub fn new(trng: &Trng) -> Self {
        let mut s = Snake {
            body: VecDeque::new(),
            dir: (1, 0),
            queue: VecDeque::new(),
            food: (0, 0),
            score: 0,
            alive: true,
            paused: false,
            last: Instant::now(),
            snd: None,
        };
        for i in 0..4 {
            s.body.push_back((10 - i, 6));
        }
        s.place_food(trng);
        s
    }
    fn place_food(&mut self, trng: &Trng) {
        for _ in 0..200 {
            let f = (below(trng, S_COLS as u32).unwrap_or(0) as i32, below(trng, S_ROWS as u32).unwrap_or(0) as i32);
            if !self.body.contains(&f) {
                self.food = f;
                return;
            }
        }
    }
    fn tick_ms(&self) -> u128 { 150u32.saturating_sub(self.score * 4).max(65) as u128 }
}

impl Game for Snake {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        let want = match b {
            Btn::Up => (0, -1),
            Btn::Down => (0, 1),
            Btn::Left => (-1, 0),
            Btn::Right => (1, 0),
            Btn::Ok => {
                if self.alive {
                    self.paused = !self.paused;
                    self.last = Instant::now();
                }
                return true;
            }
            _ => return false,
        };
        let cur = self.queue.back().copied().unwrap_or(self.dir);
        if want != cur && (want.0 + cur.0, want.1 + cur.1) != (0, 0) && self.queue.len() < 3 {
            self.queue.push_back(want);
        }
        false
    }
    fn update(&mut self, trng: &Trng) -> bool {
        if !self.alive || self.paused || self.last.elapsed().as_millis() < self.tick_ms() {
            return false;
        }
        self.last = Instant::now();
        if let Some(d) = self.queue.pop_front() {
            self.dir = d;
        }
        let h = *self.body.front().unwrap();
        let n = (h.0 + self.dir.0, h.1 + self.dir.1);
        let ate = n == self.food;
        let tail_leaves = !ate;
        let hits_self = self
            .body
            .iter()
            .enumerate()
            .any(|(i, &p)| p == n && !(tail_leaves && i == self.body.len() - 1));
        if n.0 < 0 || n.1 < 0 || n.0 >= S_COLS as i32 || n.1 >= S_ROWS as i32 || hits_self {
            self.alive = false;
            self.snd = Some(Snd::Crash);
            return true;
        }
        self.body.push_front(n);
        if ate {
            self.score += 1;
            self.snd = Some(Snd::Coin);
            self.place_food(trng);
        } else {
            self.body.pop_back();
        }
        true
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        c.text(0, 0, &format!("score {} best {}", self.score, best));
        c.rect(0, S_TOP - 1, 128, S_ROWS * S_CELL + 2);
        for (i, &(x, y)) in self.body.iter().enumerate() {
            let (px, py) = (x as usize * S_CELL, S_TOP + y as usize * S_CELL);
            c.fill_rect(px, py, S_CELL - if i == 0 { 0 } else { 1 }, S_CELL - if i == 0 { 0 } else { 1 }, true);
        }
        let (fx, fy) = (self.food.0 as usize * S_CELL, S_TOP + self.food.1 as usize * S_CELL);
        c.rect(fx, fy, S_CELL, S_CELL);
        if self.paused {
            c.fill_rect(20, 24, 88, 24, false);
            c.rect(20, 24, 88, 24);
            c.text(32, 30, "PAUSED - OK");
        }
        if !self.alive {
            game_over_box(c, "GAME OVER");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- Pong
const P_TOP: i32 = 12;
const P_BOT: i32 = 63;
const P_H: i32 = 12; // paddle height

pub struct Pong {
    me: i32, // top of my paddle
    ai: i32,
    bx: i32,
    by: i32,
    vx: i32,
    vy: i32,
    dir: i32, // -1 up, 0, 1 down (from held buttons)
    score: u32,
    alive: bool,
    paused: bool,
    last: Instant,
    snd: Option<Snd>,
}

impl Pong {
    pub fn new(trng: &Trng) -> Self {
        let vy = below(trng, 3).unwrap_or(1) as i32 - 1;
        Pong {
            me: 32,
            ai: 32,
            bx: 64,
            by: 36,
            vx: 2,
            vy: if vy == 0 { 1 } else { vy },
            dir: 0,
            score: 0,
            alive: true,
            paused: false,
            last: Instant::now(),
            snd: None,
        }
    }
}

impl Game for Pong {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if b == Btn::Ok && self.alive {
            self.paused = !self.paused;
            self.last = Instant::now();
            return true;
        }
        false
    }
    fn held(&mut self, down: &[bool; 6]) {
        self.dir = match (down[1], down[2]) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if !self.alive || self.paused || self.last.elapsed().as_millis() < 30 {
            return false;
        }
        self.last = Instant::now();
        self.me = (self.me + self.dir * 3).clamp(P_TOP, P_BOT - P_H);
        // the computer paddle chases the ball, a little slower than the ball can go
        let target = self.by - P_H / 2;
        if self.bx > 64 {
            if self.ai + 1 < target {
                self.ai += 1;
            } else if self.ai - 1 > target {
                self.ai -= 1;
            }
            self.ai = self.ai.clamp(P_TOP, P_BOT - P_H);
        }
        self.bx += self.vx;
        self.by += self.vy;
        if self.by <= P_TOP + 1 || self.by >= P_BOT - 1 {
            self.vy = -self.vy;
            self.by = self.by.clamp(P_TOP + 1, P_BOT - 1);
        }
        // my paddle on the left (x = 4..6)
        if self.bx <= 7 && self.vx < 0 {
            if self.by >= self.me - 1 && self.by <= self.me + P_H + 1 {
                self.vx = -self.vx + if self.score % 4 == 3 { 1 } else { 0 };
                self.vy = ((self.by - (self.me + P_H / 2)) / 3).clamp(-3, 3);
                if self.vy == 0 {
                    self.vy = if self.by % 2 == 0 { 1 } else { -1 };
                }
                self.bx = 8;
                self.score += 1;
                self.snd = Some(Snd::Tick);
            } else if self.bx < 0 {
                self.alive = false;
                self.snd = Some(Snd::Crash);
            }
        }
        // computer paddle on the right (x = 121..123)
        if self.bx >= 120 && self.vx > 0 {
            if self.by >= self.ai - 1 && self.by <= self.ai + P_H + 1 {
                self.vx = -self.vx;
                self.bx = 119;
            } else if self.bx > 127 {
                // the computer missed: serve again, you scored a point
                self.score += 3;
                self.snd = Some(Snd::Coin);
                self.bx = 64;
                self.by = 36;
                self.vx = -2;
            }
        }
        true
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        c.text(0, 0, &format!("score {} best {}", self.score, best));
        for y in (P_TOP..P_BOT).step_by(4) {
            c.fill_rect(63, y as usize, 1, 2, true);
        }
        c.fill_rect(4, self.me as usize, 3, P_H as usize, true);
        c.fill_rect(121, self.ai as usize, 3, P_H as usize, true);
        c.fill_rect((self.bx.clamp(0, 125)) as usize, (self.by - 1).max(0) as usize, 3, 3, true);
        if self.paused {
            c.fill_rect(20, 24, 88, 24, false);
            c.rect(20, 24, 88, 24);
            c.text(32, 30, "PAUSED - OK");
        }
        if !self.alive {
            game_over_box(c, "MISSED!");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- Flappy Bao
const F_GAP: i32 = 24;

pub struct Flappy {
    y: i32,  // bird height in 1/8 px
    vy: i32, // 1/8 px per tick
    pipes: Vec<(i32, i32, bool)>, // x, gap top, already scored
    score: u32,
    alive: bool,
    started: bool,
    last: Instant,
    snd: Option<Snd>,
}

impl Flappy {
    pub fn new(trng: &Trng) -> Self {
        let g = 16 + below(trng, 20).unwrap_or(0) as i32;
        Flappy {
            y: 32 * 8,
            vy: 0,
            pipes: vec![(150, g, false)],
            score: 0,
            alive: true,
            started: false,
            last: Instant::now(),
            snd: None,
        }
    }
}

impl Game for Flappy {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if matches!(b, Btn::Ok | Btn::Up) && self.alive {
            self.started = true;
            self.vy = -18; // one flap lifts the bird about 11 px
            self.snd = Some(Snd::Click);
            return true;
        }
        false
    }
    fn update(&mut self, trng: &Trng) -> bool {
        if !self.alive || self.last.elapsed().as_millis() < 35 {
            return false;
        }
        self.last = Instant::now();
        if !self.started {
            return false;
        }
        self.vy = (self.vy + 2).min(32);
        self.y += self.vy;
        let by = self.y / 8;
        for p in self.pipes.iter_mut() {
            p.0 -= 2;
        }
        self.pipes.retain(|p| p.0 > -12);
        if self.pipes.last().map(|p| p.0 < 78).unwrap_or(true) {
            let g = 14 + below(trng, 26).unwrap_or(0) as i32;
            self.pipes.push((130, g, false));
        }
        // bird is a 6x6 box at x = 20..26
        let mut hit = by < 12 || by > 57;
        for p in self.pipes.iter_mut() {
            if p.0 < 26 && p.0 + 10 > 20 && (by < p.1 || by + 6 > p.1 + F_GAP) {
                hit = true;
            }
            if !p.2 && p.0 + 10 < 20 {
                p.2 = true;
                self.score += 1;
                self.snd = Some(Snd::Coin);
            }
        }
        if hit {
            self.alive = false;
            self.snd = Some(Snd::Crash);
        }
        true
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        c.text(0, 0, &format!("score {} best {}", self.score, best));
        c.fill_rect(0, 63, 128, 1, true);
        for p in &self.pipes {
            let x = p.0.max(0) as usize;
            let w = if p.0 < 0 { (10 + p.0).max(0) as usize } else { 10 };
            if p.0 > 127 {
                continue;
            }
            let w = w.min(128 - x);
            let gt = p.1 as usize;
            if gt > 12 {
                c.fill_rect(x, 12, w, gt - 12, true);
            }
            let gb = gt + F_GAP as usize;
            if gb < 63 {
                c.fill_rect(x, gb, w, 63 - gb, true);
            }
        }
        let by = (self.y / 8).clamp(0, 58) as usize;
        c.rect(20, by, 7, 6);
        c.fill_rect(24, by + 1, 2, 2, true); // eye
        if !self.started {
            c.text(14, 24, "OK / UP = flap");
        }
        if !self.alive {
            game_over_box(c, "SPLAT!");
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- Simon
#[derive(Clone, Copy, PartialEq)]
enum SState {
    Showing(usize, bool),
    Input(usize),
    Over,
}

pub struct Simon {
    seq: Vec<u8>, // 0 up, 1 down, 2 left, 3 right
    score: u32,
    state: SState,
    last: Instant,
    snd: Option<Snd>,
}

impl Simon {
    pub fn new(trng: &Trng) -> Self {
        Simon {
            seq: vec![below(trng, 4).unwrap_or(0) as u8],
            score: 0,
            state: SState::Showing(0, true),
            last: Instant::now(),
            snd: Some(Snd::Tick),
        }
    }
}

impl Game for Simon {
    fn press(&mut self, b: Btn, trng: &Trng) -> bool {
        let dir = match b {
            Btn::Up => 0u8,
            Btn::Down => 1,
            Btn::Left => 2,
            Btn::Right => 3,
            _ => return false,
        };
        if let SState::Input(i) = self.state {
            if self.seq[i] == dir {
                self.snd = Some(Snd::Tick);
                if i + 1 == self.seq.len() {
                    self.score += 1;
                    self.seq.push(below(trng, 4).unwrap_or(0) as u8);
                    self.state = SState::Showing(0, true);
                    self.last = Instant::now();
                } else {
                    self.state = SState::Input(i + 1);
                }
            } else {
                self.state = SState::Over;
                self.snd = Some(Snd::Crash);
            }
            return true;
        }
        false
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if let SState::Showing(i, lit) = self.state {
            let dur = if lit { 450 } else { 200 };
            if self.last.elapsed().as_millis() >= dur {
                self.last = Instant::now();
                self.state = if lit {
                    SState::Showing(i, false)
                } else if i + 1 < self.seq.len() {
                    self.snd = Some(Snd::Tick);
                    SState::Showing(i + 1, true)
                } else {
                    SState::Input(0)
                };
                return true;
            }
        }
        false
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        match self.state {
            SState::Showing(i, lit) => {
                c.text(0, 0, &format!("watch   score {}", self.score));
                if lit {
                    c.text_big(52, 14, ["^", "v", "<", ">"][self.seq[i] as usize], 4);
                }
            }
            SState::Input(i) => {
                c.text(0, 0, &format!("your turn  {}/{}", i, self.seq.len()));
                c.text(0, 26, "UP DOWN LEFT RIGHT");
                c.text(0, 39, &format!("best {}", best));
            }
            SState::Over => {
                game_over_box(c, "WRONG!");
                c.text(12, 33, &format!("score {} best {}", self.score, best));
            }
        }
    }
    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { self.state == SState::Over }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}

// ---------------------------------------------------------------- Reaction
#[derive(Clone, Copy, PartialEq)]
enum RState {
    Wait(u32), // milliseconds until GO
    Go,
    Result(u32),
    TooEarly,
}

pub struct Reaction {
    st: RState,
    since: Instant,
    snd: Option<Snd>,
}

impl Reaction {
    pub fn new(trng: &Trng) -> Self {
        Reaction {
            st: RState::Wait(1500 + below(trng, 3500).unwrap_or(0)),
            since: Instant::now(),
            snd: None,
        }
    }
}

impl Game for Reaction {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if b != Btn::Ok {
            return false;
        }
        match self.st {
            RState::Wait(_) => {
                self.st = RState::TooEarly;
                self.snd = Some(Snd::Crash);
            }
            RState::Go => {
                self.st = RState::Result(self.since.elapsed().as_millis() as u32);
                self.snd = Some(Snd::Coin);
            }
            _ => return false,
        }
        true
    }
    fn update(&mut self, _t: &Trng) -> bool {
        if let RState::Wait(ms) = self.st {
            if self.since.elapsed().as_millis() as u32 >= ms {
                self.st = RState::Go;
                self.since = Instant::now();
                self.snd = Some(Snd::Click);
                return true;
            }
        }
        false
    }
    fn draw(&self, c: &mut Canvas, best: u32) {
        c.text(0, 0, "Reaction test");
        match self.st {
            RState::Wait(_) => c.text_big(10, 22, "wait...", 2),
            RState::Go => {
                c.fill_rect(0, 12, 128, 52, true);
                c.fill_rect(10, 22, 108, 32, false);
                c.text_big(34, 28, "GO!", 3);
            }
            RState::Result(ms) => {
                c.text_big(10, 18, &format!("{} ms", ms), 3);
                c.text(0, 52, &format!("best {} ms  OK=again", best));
            }
            RState::TooEarly => {
                c.text_big(4, 20, "too early", 2);
                c.text(0, 52, "OK = again");
            }
        }
    }
    fn score(&self) -> u32 {
        match self.st {
            RState::Result(ms) => ms,
            _ => 0,
        }
    }
    fn over(&self) -> bool { matches!(self.st, RState::Result(_) | RState::TooEarly) }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
    fn lower_is_better(&self) -> bool { true }
}
