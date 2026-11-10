// doom.rs - "ChocoDoom": a tiny first-person shooter for the 128x64 OLED (a ray-casting engine like the one
// in Wolfenstein 3D). One level, five demons. UP/DOWN walk, LEFT/RIGHT turn (hold), OK fires.
//
// Why not real DOOM? Its game data (WAD, about 4 MB) is bigger than the whole app region of this board (about
// 0.87 MB) and the engine wants several MB of RAM; the chip has 2 MiB shared with the OS. This engine keeps the
// idea (first-person view, enemies that chase you, shooting) in a few KiB. It uses f32, which is software
// emulated on this CPU but still cheap at 64 rays per frame.
use std::time::Instant;

use bao1x_hal_service::trng::Trng;

use crate::controls::Btn;
use crate::games::{Game, Snd};
use crate::oled::Canvas;

const MAP: [&str; 16] = [
    "################",
    "#..............#",
    "#.####....####.#",
    "#.#..........#.#",
    "#.#..######..#.#",
    "#....#....#....#",
    "#.#..#....#..#.#",
    "#.#..##..##..#.#",
    "#.#..........#.#",
    "#.####....####.#",
    "#..............#",
    "#...##....##...#",
    "#...#......#...#",
    "#...#......#...#",
    "#..............#",
    "################",
];

const NCOL: usize = 64; // rays per frame, each drawn 2 pixels wide
const VIEW_TOP: i32 = 12;
const VIEW_H: i32 = 52;
const TICK_MS: u128 = 70;

struct Enemy {
    x: f32,
    y: f32,
    hp: i32,
}

pub struct Doom {
    px: f32,
    py: f32,
    a: f32,
    hp: i32,
    kills: u32,
    enemies: Vec<Enemy>,
    fwd: i32,
    turn: i32,
    fire_cd: i32,
    flash: i32,
    hurt_cd: i32,
    alive: bool,
    won: bool,
    last: Instant,
    snd: Option<Snd>,
}

fn is_wall(x: f32, y: f32) -> bool {
    if x < 0.0 || y < 0.0 {
        return true;
    }
    let (ix, iy) = (x as usize, y as usize);
    if iy >= MAP.len() || ix >= MAP[0].len() {
        return true;
    }
    MAP[iy].as_bytes()[ix] == b'#'
}

impl Doom {
    pub fn new(_trng: &Trng) -> Self {
        let spots = [(13.5, 1.5), (13.5, 13.5), (8.0, 8.0), (1.5, 13.5), (8.0, 5.0)];
        Doom {
            px: 1.5,
            py: 1.5,
            a: 0.6,
            hp: 5,
            kills: 0,
            enemies: spots.iter().map(|&(x, y)| Enemy { x, y, hp: 2 }).collect(),
            fwd: 0,
            turn: 0,
            fire_cd: 0,
            flash: 0,
            hurt_cd: 0,
            alive: true,
            won: false,
            last: Instant::now(),
            snd: None,
        }
    }

    /// Enemy position in camera space: (sideways, depth). Depth > 0 means in front of the player.
    fn project(&self, e: &Enemy) -> (f32, f32) {
        let (dx, dy) = (self.a.cos(), self.a.sin());
        let (rx, ry) = (e.x - self.px, e.y - self.py);
        // plane vector is (-dy, dx) * 0.66; invert the 2x2 camera matrix
        let (plx, ply) = (-dy * 0.66, dx * 0.66);
        let inv = 1.0 / (plx * dy - dx * ply);
        let tx = inv * (dy * rx - dx * ry);
        let ty = inv * (-ply * rx + plx * ry);
        (tx, ty)
    }

    /// Distance to the wall straight ahead (used to decide whether a shot can reach an enemy).
    fn wall_ahead(&self) -> f32 {
        let (dx, dy) = (self.a.cos(), self.a.sin());
        let mut d = 0.0;
        while d < 20.0 {
            if is_wall(self.px + dx * d, self.py + dy * d) {
                return d;
            }
            d += 0.05;
        }
        20.0
    }

    fn fire(&mut self) {
        if self.fire_cd > 0 || !self.alive {
            return;
        }
        self.fire_cd = 4;
        self.flash = 2;
        self.snd = Some(Snd::Tick);
        let wall = self.wall_ahead();
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.hp <= 0 {
                continue;
            }
            let (tx, ty) = self.project(e);
            if ty > 0.2 && ty < wall && (tx / ty).abs() < 0.22 && best.map(|b| ty < b.1).unwrap_or(true) {
                best = Some((i, ty));
            }
        }
        if let Some((i, _)) = best {
            self.enemies[i].hp -= 1;
            if self.enemies[i].hp <= 0 {
                self.kills += 1;
                self.snd = Some(Snd::Coin);
            }
        }
    }
}

fn shade(level: u8, x: i32, y: i32) -> bool {
    match level {
        0 => true,
        1 => (x + y) & 1 == 0,
        2 => (x & 1 == 0) && (y & 1 == 0),
        _ => (x & 3 == 0) && (y & 3 == 0),
    }
}

impl Game for Doom {
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if b == Btn::Ok {
            self.fire();
            return true;
        }
        false
    }

    fn held(&mut self, down: &[bool; 6]) {
        self.fwd = (down[1] as i32) - (down[2] as i32);
        self.turn = (down[4] as i32) - (down[3] as i32);
    }

    fn update(&mut self, _t: &Trng) -> bool {
        if !self.alive || self.last.elapsed().as_millis() < TICK_MS {
            return false;
        }
        self.last = Instant::now();
        self.a += self.turn as f32 * 0.14;
        if self.fwd != 0 {
            let (nx, ny) = (self.px + self.a.cos() * 0.17 * self.fwd as f32, self.py + self.a.sin() * 0.17 * self.fwd as f32);
            if !is_wall(nx, self.py) {
                self.px = nx;
            }
            if !is_wall(self.px, ny) {
                self.py = ny;
            }
        }
        if self.fire_cd > 0 {
            self.fire_cd -= 1;
        }
        if self.flash > 0 {
            self.flash -= 1;
        }
        if self.hurt_cd > 0 {
            self.hurt_cd -= 1;
        }
        let (px, py) = (self.px, self.py);
        let mut hurt = false;
        for e in self.enemies.iter_mut().filter(|e| e.hp > 0) {
            let (rx, ry) = (px - e.x, py - e.y);
            let d = (rx * rx + ry * ry).sqrt();
            if d < 9.0 && d > 0.6 {
                let (sx, sy) = (rx / d * 0.05, ry / d * 0.05);
                if !is_wall(e.x + sx, e.y) {
                    e.x += sx;
                }
                if !is_wall(e.x, e.y + sy) {
                    e.y += sy;
                }
            } else if d <= 0.6 {
                hurt = true;
            }
        }
        if hurt && self.hurt_cd == 0 {
            self.hp -= 1;
            self.hurt_cd = 10;
            self.snd = Some(Snd::Crash);
        }
        if self.hp <= 0 {
            self.alive = false;
        } else if self.enemies.iter().all(|e| e.hp <= 0) {
            self.alive = false;
            self.won = true;
        }
        true
    }

    fn draw(&self, c: &mut Canvas, _best: u32) {
        c.text(0, 0, &format!("HP {}  KILLS {}/{}", self.hp.max(0), self.kills, self.enemies.len()));
        let (dx, dy) = (self.a.cos(), self.a.sin());
        let (plx, ply) = (-dy * 0.66, dx * 0.66);
        let mid = VIEW_TOP + VIEW_H / 2;
        let mut zbuf = [20.0f32; NCOL];
        // ---- walls
        for col in 0..NCOL {
            let cam = 2.0 * col as f32 / NCOL as f32 - 1.0;
            let (rdx, rdy) = (dx + plx * cam, dy + ply * cam);
            let (mut mx, mut my) = (self.px as i32, self.py as i32);
            let ddx = if rdx == 0.0 { 1e30 } else { (1.0 / rdx).abs() };
            let ddy = if rdy == 0.0 { 1e30 } else { (1.0 / rdy).abs() };
            let (sx, mut sdx) = if rdx < 0.0 { (-1, (self.px - mx as f32) * ddx) } else { (1, (mx as f32 + 1.0 - self.px) * ddx) };
            let (sy, mut sdy) = if rdy < 0.0 { (-1, (self.py - my as f32) * ddy) } else { (1, (my as f32 + 1.0 - self.py) * ddy) };
            let mut side = 0;
            for _ in 0..40 {
                if sdx < sdy {
                    sdx += ddx;
                    mx += sx;
                    side = 0;
                } else {
                    sdy += ddy;
                    my += sy;
                    side = 1;
                }
                if is_wall(mx as f32, my as f32) {
                    break;
                }
            }
            let perp = (if side == 0 { sdx - ddx } else { sdy - ddy }).max(0.05);
            zbuf[col] = perp;
            let h = ((VIEW_H as f32) / perp) as i32;
            let y0 = (mid - h / 2).max(VIEW_TOP);
            let y1 = (mid + h / 2).min(63);
            let level = if perp < 1.6 { 0 } else if perp < 3.2 { 1 } else if perp < 5.5 { 2 } else { 3 };
            let level = if side == 1 && level < 3 { level + 1 } else { level };
            let x = (col * 2) as i32;
            for y in y0..=y1 {
                for xx in x..x + 2 {
                    if shade(level, xx, y) || y == y0 || y == y1 {
                        c.dot(xx, y, true);
                    }
                }
            }
        }
        // ---- enemies (nearest last so near ones overdraw far ones)
        let mut order: Vec<(f32, f32, usize)> = Vec::new();
        for (i, e) in self.enemies.iter().enumerate() {
            if e.hp > 0 {
                let (tx, ty) = self.project(e);
                if ty > 0.25 {
                    order.push((tx, ty, i));
                }
            }
        }
        order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (tx, ty, _) in order {
            let sx = ((NCOL as f32 / 2.0) * (1.0 + tx / ty)) as i32 * 2;
            let h = ((VIEW_H as f32) / ty) as i32;
            let w = (h * 6 / 10).max(2);
            let top = (mid - h / 2).max(VIEW_TOP);
            let bot = (mid + h / 2).min(63);
            for xx in (sx - w / 2)..(sx + w / 2) {
                let col = (xx / 2) as usize;
                if xx < 0 || xx >= 128 || col >= NCOL || ty >= zbuf[col] {
                    continue;
                }
                for y in top..=bot {
                    let eye = y > top + h / 6 && y < top + h / 3 && ((xx - sx).abs() > w / 6) && ((xx - sx).abs() < w / 3);
                    c.dot(xx, y, !eye && ((xx + y) & 1 == 0 || y == top || y == bot || xx == sx - w / 2));
                    if !eye && (xx + y) & 1 == 1 && y > top + h / 2 {
                        c.dot(xx, y, true);
                    }
                }
            }
        }
        // ---- crosshair and gun
        c.dot(64, mid, true);
        c.dot(62, mid, true);
        c.dot(66, mid, true);
        c.dot(64, mid - 2, true);
        c.dot(64, mid + 2, true);
        c.fill_rect(60, 58, 8, 6, true);
        c.fill_rect(62, 54, 4, 5, true);
        if self.flash > 0 {
            c.line(64, 53, 58, 46);
            c.line(64, 53, 70, 46);
            c.line(64, 53, 64, 44);
        }
        if self.hurt_cd > 6 {
            c.invert_rect(0, VIEW_TOP as usize, 128, 52);
        }
        if !self.alive {
            c.fill_rect(6, 16, 116, 42, false);
            c.rect(6, 16, 116, 42);
            c.text_big(12, 19, if self.won { "CLEARED!" } else { "YOU DIED" }, 2);
            c.text(12, 44, "OK retry  BACK exit");
        }
    }

    fn score(&self) -> u32 { self.kills * 100 + if self.won { self.hp.max(0) as u32 * 50 } else { 0 } }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}
