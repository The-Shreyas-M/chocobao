// pacman.rs - a small Pac-Man for the 128x64 OLED: 25x10 tile maze (5 px tiles), 124 open cells, 3 ghosts,
// 4 power pellets, 3 lives. Arrows steer (the turn is remembered until the corridor allows it), OK / BACK
// pause. Ghosts chase with a little randomness; after a power pellet they flee and can be eaten.
use std::time::{Duration, Instant};

use bao1x_hal_service::trng::Trng;

use crate::controls::Btn;
use crate::games::{Game, Snd};
use crate::oled::Canvas;
use crate::rng::below;

const W: usize = 25;
const H: usize = 10;
const T: usize = 5; // tile size in pixels
const X0: usize = 1;
const Y0: usize = 13;

// left half including the centre column; mirrored at start-up
const LEFT: [&str; H] = [
    "#############",
    "#o.....#.....",
    "#.###.#.#.##.",
    "#.#.........#",
    "....#.##.#...",
    "#.#...#...#.#",
    "#.#.#.#.#.#.#",
    "#.....#...#.#",
    "#o###.#.##.o.",
    "#############",
];

const PAC_START: (i32, i32) = (12, 4); // (x, y)
const GHOST_START: [(i32, i32); 3] = [(11, 3), (13, 3), (12, 1)];

// 5x5 sprites, one row per entry, bit 4 = leftmost pixel
const PAC_CLOSED: [u8; 5] = [0b01110, 0b11111, 0b11111, 0b11111, 0b01110];
const PAC_RIGHT: [u8; 5] = [0b01110, 0b11100, 0b11000, 0b11100, 0b01110];
const PAC_LEFT: [u8; 5] = [0b01110, 0b00111, 0b00011, 0b00111, 0b01110];
const PAC_UP: [u8; 5] = [0b01010, 0b11011, 0b11111, 0b11111, 0b01110];
const PAC_DOWN: [u8; 5] = [0b01110, 0b11111, 0b11111, 0b11011, 0b01010];
const GHOST: [u8; 5] = [0b01110, 0b11111, 0b10101, 0b11111, 0b10101];
const GHOST_SCARED: [u8; 5] = [0b01110, 0b10101, 0b11111, 0b10101, 0b10101];

struct Ghost {
    x: i32,
    y: i32,
    dir: (i32, i32),
    release_at: Instant,
}

pub struct Pacman {
    maze: [[u8; W]; H],
    px: i32,
    py: i32,
    dir: (i32, i32),
    want: (i32, i32),
    ghosts: Vec<Ghost>,
    score: u32,
    lives: i32,
    dots_left: u32,
    power_until: Option<Instant>,
    alive: bool,
    won: bool,
    paused: bool,
    mouth: bool,
    last_pac: Instant,
    last_ghost: Instant,
    ate: u32,
    snd: Option<Snd>,
}

impl Pacman {
    pub fn new(_t: &Trng) -> Self {
        let mut maze = [[b' '; W]; H];
        let mut dots = 0;
        for (y, l) in LEFT.iter().enumerate() {
            let full: Vec<u8> = l.bytes().chain(l.bytes().rev().skip(1)).collect();
            for x in 0..W {
                maze[y][x] = full[x];
                if full[x] == b'.' || full[x] == b'o' {
                    dots += 1;
                }
            }
        }
        // no pellet under the start tiles
        for &(x, y) in std::iter::once(&PAC_START).chain(GHOST_START.iter()) {
            if maze[y as usize][x as usize] != b' ' {
                maze[y as usize][x as usize] = b' ';
                dots -= 1;
            }
        }
        let mut p = Pacman {
            maze,
            px: PAC_START.0,
            py: PAC_START.1,
            dir: (0, 0),
            want: (0, 0),
            ghosts: Vec::new(),
            score: 0,
            lives: 3,
            dots_left: dots,
            power_until: None,
            alive: true,
            won: false,
            paused: false,
            mouth: false,
            last_pac: Instant::now(),
            last_ghost: Instant::now(),
            ate: 0,
            snd: None,
        };
        p.reset_ghosts();
        p
    }

    fn reset_ghosts(&mut self) {
        self.ghosts.clear();
        for (i, &(x, y)) in GHOST_START.iter().enumerate() {
            self.ghosts.push(Ghost {
                x,
                y,
                dir: (0, 0),
                release_at: Instant::now() + Duration::from_millis(1500 + 2500 * i as u64),
            });
        }
    }

    fn wall(&self, x: i32, y: i32) -> bool {
        let x = x.rem_euclid(W as i32);
        if y < 0 || y >= H as i32 {
            return true;
        }
        self.maze[y as usize][x as usize] == b'#'
    }

    fn scared(&self) -> bool { self.power_until.map(|t| Instant::now() < t).unwrap_or(false) }

    fn hit_check(&mut self) {
        let scared = self.scared();
        let (px, py) = (self.px, self.py);
        let mut lost = false;
        for g in self.ghosts.iter_mut() {
            if g.x == px && g.y == py {
                if scared {
                    self.score += 200;
                    self.snd = Some(Snd::Coin);
                    let i = GHOST_START[0];
                    g.x = i.0;
                    g.y = i.1;
                    g.dir = (0, 0);
                    g.release_at = Instant::now() + Duration::from_millis(2500);
                } else {
                    lost = true;
                }
            }
        }
        if lost {
            self.lives -= 1;
            self.snd = Some(Snd::Crash);
            if self.lives <= 0 {
                self.alive = false;
            } else {
                self.px = PAC_START.0;
                self.py = PAC_START.1;
                self.dir = (0, 0);
                self.want = (0, 0);
                self.power_until = None;
                self.reset_ghosts();
            }
        }
    }

    fn step_pacman(&mut self) {
        self.mouth = !self.mouth;
        if self.want != (0, 0) && !self.wall(self.px + self.want.0, self.py + self.want.1) {
            self.dir = self.want;
        }
        if self.dir != (0, 0) && !self.wall(self.px + self.dir.0, self.py + self.dir.1) {
            self.px = (self.px + self.dir.0).rem_euclid(W as i32);
            self.py += self.dir.1;
            let cell = &mut self.maze[self.py as usize][self.px as usize];
            if *cell == b'.' {
                *cell = b' ';
                self.score += 10;
                self.dots_left -= 1;
                self.ate += 1;
                if self.ate % 4 == 0 {
                    self.snd = Some(Snd::Tick);
                }
            } else if *cell == b'o' {
                *cell = b' ';
                self.score += 50;
                self.dots_left -= 1;
                self.power_until = Some(Instant::now() + Duration::from_millis(6500));
                self.snd = Some(Snd::Coin);
            }
        }
        if self.dots_left == 0 {
            self.alive = false;
            self.won = true;
        }
    }

    fn step_ghosts(&mut self, trng: &Trng) {
        let scared = self.scared();
        let (px, py) = (self.px, self.py);
        let now = Instant::now();
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for gi in 0..self.ghosts.len() {
            if now < self.ghosts[gi].release_at {
                continue;
            }
            let (gx, gy, gd) = (self.ghosts[gi].x, self.ghosts[gi].y, self.ghosts[gi].dir);
            let mut options: Vec<(i32, i32)> = Vec::new();
            for &d in dirs.iter() {
                if !self.wall(gx + d.0, gy + d.1) && (d.0 != -gd.0 || d.1 != -gd.1 || gd == (0, 0)) {
                    options.push(d);
                }
            }
            if options.is_empty() {
                options.push((-gd.0, -gd.1)); // dead end: turn around
            }
            let random = below(trng, 100).unwrap_or(0) < if scared { 40 } else { 22 };
            let pick = if random {
                options[below(trng, options.len() as u32).unwrap_or(0) as usize]
            } else {
                let dist = |d: &(i32, i32)| {
                    let nx = (gx + d.0).rem_euclid(W as i32);
                    (nx - px).abs() + (gy + d.1 - py).abs()
                };
                if scared {
                    *options.iter().max_by_key(|d| dist(d)).unwrap()
                } else {
                    *options.iter().min_by_key(|d| dist(d)).unwrap()
                }
            };
            let g = &mut self.ghosts[gi];
            g.dir = pick;
            g.x = (gx + pick.0).rem_euclid(W as i32);
            g.y = gy + pick.1;
        }
    }
}

fn blit(c: &mut Canvas, x: usize, y: usize, rows: &[u8; 5]) {
    for (r, &bits) in rows.iter().enumerate() {
        for col in 0..5 {
            if bits & (0b10000 >> col) != 0 {
                c.dot((x + col) as i32, (y + r) as i32, true);
            }
        }
    }
}

impl Game for Pacman {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if self.paused {
            if b == Btn::Ok {
                self.paused = false;
                self.last_pac = Instant::now();
                self.last_ghost = Instant::now();
            }
            return true;
        }
        match b {
            Btn::Up => self.want = (0, -1),
            Btn::Down => self.want = (0, 1),
            Btn::Left => self.want = (-1, 0),
            Btn::Right => self.want = (1, 0),
            Btn::Ok => self.set_paused(true),
            _ => return false,
        }
        true
    }

    fn update(&mut self, trng: &Trng) -> bool {
        if !self.alive || self.paused {
            return false;
        }
        let mut redraw = false;
        if self.last_pac.elapsed().as_millis() >= 140 {
            self.last_pac = Instant::now();
            self.step_pacman();
            self.hit_check();
            redraw = true;
        }
        let ghost_ms = if self.scared() { 230 } else { 175 };
        if self.last_ghost.elapsed().as_millis() >= ghost_ms {
            self.last_ghost = Instant::now();
            self.step_ghosts(trng);
            self.hit_check();
            redraw = true;
        }
        redraw
    }

    fn draw(&self, c: &mut Canvas, best: u32) {
        c.text(0, 0, &format!("{} best {}", self.score, best));
        let hearts = "o".repeat(self.lives.max(0) as usize);
        c.text(128 - 6 * hearts.len().max(1), 0, &hearts);
        // maze
        for y in 0..H {
            for x in 0..W {
                let (sx, sy) = (X0 + x * T, Y0 + y * T);
                match self.maze[y][x] {
                    b'#' => {
                        c.fill_rect(sx + 1, sy + 1, 3, 3, true);
                        if x + 1 < W && self.maze[y][x + 1] == b'#' {
                            c.fill_rect(sx + 4, sy + 1, 2, 3, true);
                        }
                        if y + 1 < H && self.maze[y + 1][x] == b'#' {
                            c.fill_rect(sx + 1, sy + 4, 3, 2, true);
                        }
                    }
                    b'.' => c.dot((sx + 2) as i32, (sy + 2) as i32, true),
                    b'o' => c.fill_rect(sx + 1, sy + 1, 3, 3, true),
                    _ => {}
                }
            }
        }
        // pac-man
        let sprite = if !self.mouth {
            &PAC_CLOSED
        } else {
            match self.dir {
                (1, 0) => &PAC_RIGHT,
                (-1, 0) => &PAC_LEFT,
                (0, -1) => &PAC_UP,
                (0, 1) => &PAC_DOWN,
                _ => &PAC_RIGHT,
            }
        };
        c.fill_rect(X0 + self.px as usize * T, Y0 + self.py as usize * T, 5, 5, false);
        blit(c, X0 + self.px as usize * T, Y0 + self.py as usize * T, sprite);
        // ghosts
        let scared = self.scared();
        let flash = self.power_until.map(|t| t.saturating_duration_since(Instant::now()).as_millis() < 1800).unwrap_or(false);
        for g in &self.ghosts {
            let (sx, sy) = (X0 + g.x as usize * T, Y0 + g.y as usize * T);
            c.fill_rect(sx, sy, 5, 5, false);
            let s = if scared && !(flash && self.mouth) { &GHOST_SCARED } else { &GHOST };
            blit(c, sx, sy, s);
        }
        if !self.alive {
            c.fill_rect(6, 16, 116, 42, false);
            c.rect(6, 16, 116, 42);
            c.text_big(12, 19, if self.won { "CLEARED!" } else { "GAME OVER" }, 2);
            c.text(12, 44, "OK retry  BACK exit");
        }
    }

    fn score(&self) -> u32 { self.score }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.alive {
            self.paused = p;
            self.last_pac = Instant::now();
            self.last_ghost = Instant::now();
        }
    }
}
