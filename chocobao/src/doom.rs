// doom.rs - "Doom": a tiny first-person shooter for the 128x64 OLED (a ray-casting engine like the one
// in Wolfenstein 3D). One level, laid out like the first level of the original (E1M1, the hangar), six demons. UP/DOWN walk, LEFT/RIGHT turn (hold), OK fires.
//
// Why not real DOOM? Its game data (WAD, about 4 MB) is bigger than the whole app region of this board (about
// 0.87 MB) and the engine wants several MB of RAM; the chip has 2 MiB shared with the OS. This engine keeps the
// idea (first-person view, enemies that chase you, shooting) in a few KiB. It uses f32, which is software
// emulated on this CPU but still cheap at 64 rays per frame.
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use bao1x_hal_service::trng::Trng;

use crate::controls::Btn;
use crate::games::{Game, Snd};
use crate::oled::Canvas;

const MAP: [&str; 24] = [
    "########################",
    "#......#..........#....#",
    "#......#..........#....#",
    "#......#..........#....#",
    "#......................#",
    "#......#..........#....#",
    "###.########.#######.###",
    "#......................#",
    "#......................#",
    "#..%%%%%%%%%%%%%%%%%%..#",
    "#..%................%..#",
    "#..%................%..#",
    "#..%................%..#",
    "#..%%%%%%%%.%%%%%%%%%..#",
    "#......................#",
    "#......................#",
    "#.....###......###.....#",
    "#.....###......###.....#",
    "#.....###......###.....#",
    "#......................#",
    "#......................#",
    "#......................#",
    "#......................#",
    "########################",
];

// levels 2 to 5: generated layouts (every room is reachable); OK after a win starts the next level
const MAP2: [&str; 24] = [
    "########################",
    "#......................#",
    "#......................#",
    "#..####.########.####..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..#...%........%...#..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..####.###%%###.####..#",
    "#..####.###%%###.####..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..#...%........%...#..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..#.......##.......#..#",
    "#..####.########.####..#",
    "#......................#",
    "#......................#",
    "########################",
];

const MAP3: [&str; 24] = [
    "########################",
    "#......................#",
    "#......................#",
    "#####################..#",
    "#####################..#",
    "#......................#",
    "#......................#",
    "#..#####################",
    "#..#####################",
    "#.....%.....%.....%....#",
    "#......................#",
    "#####################..#",
    "#####################..#",
    "#......................#",
    "#......................#",
    "#..#####################",
    "#..#####################",
    "#......................#",
    "#......................#",
    "#####################..#",
    "#####################..#",
    "#.....%.....%.....%....#",
    "#......................#",
    "########################",
];

const MAP4: [&str; 24] = [
    "########################",
    "#....##############....#",
    "#......................#",
    "#......................#",
    "#....######..######....#",
    "##..#######..#######..##",
    "##..##............##..##",
    "##..##............##..##",
    "##..##............##..##",
    "##..##...%%...%%..##..##",
    "##..##...%%...%%..##..##",
    "##....................##",
    "##....................##",
    "##..##............##..##",
    "##..##...%%...%%..##..##",
    "##..##...%%...%%..##..##",
    "##..##............##..##",
    "##..##............##..##",
    "##..#######..#######..##",
    "#....######..######....#",
    "#......................#",
    "#......................#",
    "#....##############....#",
    "########################",
];

const MAP5: [&str; 24] = [
    "########################",
    "#......##......##......#",
    "#......##......##......#",
    "#..%................%..#",
    "#......................#",
    "#......##......##......#",
    "#......##......##......#",
    "###..######..######..###",
    "###..######..######..###",
    "#......##......##......#",
    "#......##......##......#",
    "#..........%%..........#",
    "#..........%%..........#",
    "#......##......##......#",
    "#......##......##......#",
    "###..######..######..###",
    "###..######..######..###",
    "#......##......##......#",
    "#......##......##......#",
    "#......................#",
    "#..%................%..#",
    "#......##......##......#",
    "#......##......##......#",
    "########################",
];
const STARTS: [(f32, f32, f32); 4] = [(2.5, 2.5, 0.79), (1.5, 1.5, 0.79), (2.5, 2.5, 0.79), (2.5, 2.5, 0.79)];
const SPOTS: [&[(f32, f32)]; 4] = [
    &[(22.5, 22.5), (22.5, 1.5), (1.5, 22.5), (13.5, 10.5), (11.5, 21.5), (2.5, 12.5), (22.5, 12.5), (11.5, 1.5)],
    &[(1.5, 21.5), (22.5, 11.5), (15.5, 22.5), (14.5, 1.5), (7.5, 10.5), (22.5, 2.5), (14.5, 14.5), (22.5, 19.5), (8.5, 18.5)],
    &[(22.5, 22.5), (22.5, 1.5), (1.5, 22.5), (12.5, 12.5), (12.5, 2.5), (2.5, 12.5), (21.5, 10.5), (10.5, 21.5), (7.5, 7.5), (17.5, 17.5)],
    &[(22.5, 22.5), (22.5, 1.5), (1.5, 22.5), (13.5, 11.5), (3.5, 12.5), (11.5, 21.5), (12.5, 1.5), (22.5, 12.5), (17.5, 17.5), (17.5, 5.5), (9.5, 6.5)],
];

static LEVEL: AtomicUsize = AtomicUsize::new(0);
static TUTORIAL_SEEN: AtomicBool = AtomicBool::new(false); // the how-to is shown once, before level 1

fn map() -> &'static [&'static str] {
    match LEVEL.load(Ordering::Relaxed) {
        1 => &MAP2[..],
        2 => &MAP3[..],
        3 => &MAP4[..],
        4 => &MAP5[..],
        _ => &MAP[..],
    }
}

const NCOL: usize = 64; // rays per frame, each drawn 2 pixels wide
const VIEW_TOP: i32 = 0;
const VIEW_H: i32 = 54;
const VIEW_BOT: i32 = 53; // the status bar owns the last 10 rows
const MAX_HP: i32 = 8;
const TURN_MIN: f32 = 0.8; // rad/s the moment LEFT/RIGHT goes down (fine aiming)
const TURN_MAX: f32 = 2.0; // rad/s after the key was held for TURN_RAMP seconds
const TURN_RAMP: f32 = 0.5;
const TURN_ACC: f32 = 14.0; // rad/s^2
const MOVE_MAX: f32 = 2.6; // map units/s
const MOVE_ACC: f32 = 18.0;
const FIRE_CD: f32 = 0.16; // seconds between shots

struct Enemy {
    x: f32,
    y: f32,
    hp: i32,
    cd: f32,    // seconds until it may shoot again
    kind: u8, // 0 = zombieman (shoots from a distance), 1 = imp (comes close)
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
    fire_cd: f32,
    flash: f32,
    hurt_cd: f32,
    turn_v: f32,
    turn_held: f32,
    level: usize,
    move_v: f32,
    fire_held: bool,
    born: Instant,
    alive: bool,
    won: bool,
    paused: bool,
    started: bool, // false while the how-to is on screen: nothing moves until OK
    last: Instant,
    snd: Option<Snd>,
}

/// True when nothing solid is between the two points.
fn line_clear(ax: f32, ay: f32, bx: f32, by: f32) -> bool {
    let (dx, dy) = (bx - ax, by - ay);
    let n = ((dx * dx + dy * dy).sqrt() / 0.25) as i32;
    for i in 1..n {
        let f = i as f32 / n as f32;
        if is_wall(ax + dx * f, ay + dy * f) {
            return false;
        }
    }
    true
}

fn is_wall(x: f32, y: f32) -> bool {
    if x < 0.0 || y < 0.0 {
        return true;
    }
    let (ix, iy) = (x as usize, y as usize);
    let m = map();
    if iy >= m.len() || ix >= m[0].len() {
        return true;
    }
    m[iy].as_bytes()[ix] != b'.'
}

impl Doom {
    pub fn new(_trng: &Trng) -> Self {
        let level = LEVEL.load(Ordering::Relaxed) % 5;
        let (spots, start, angle): (Vec<(f32, f32)>, (f32, f32), f32) = if level == 0 {
            (vec![(3.5, 3.5), (13.5, 2.5), (21.5, 3.5), (11.5, 11.5), (5.5, 15.5), (18.5, 15.5), (12.5, 18.5)], (12.5, 21.5), -1.57)
        } else {
            let st = STARTS[level - 1];
            (SPOTS[level - 1].to_vec(), (st.0, st.1), st.2)
        };
        Doom {
            born: Instant::now(),
            px: start.0,
            py: start.1,
            a: angle,
            hp: MAX_HP,
            kills: 0,
            enemies: spots.iter().enumerate().map(|(i, &(x, y))| Enemy { x, y, hp: 2, cd: 1.5 + i as f32 * 0.45, kind: (i % 2) as u8 }).collect(),
            fwd: 0,
            turn: 0,
            fire_cd: 0.0,
            flash: 0.0,
            hurt_cd: 0.0,
            turn_v: 0.0,
            turn_held: 0.0,
            level,
            move_v: 0.0,
            fire_held: false,
            alive: true,
            won: false,
            paused: false,
            started: level != 0 || TUTORIAL_SEEN.load(Ordering::Relaxed),
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
        if self.fire_cd > 0.0 || !self.alive {
            return;
        }
        self.fire_cd = FIRE_CD;
        self.flash = 0.12;
        self.snd = Some(Snd::Tick);
        let wall = self.wall_ahead();
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.enemies.iter().enumerate() {
            if e.hp <= 0 {
                continue;
            }
            let (tx, ty) = self.project(e);
            if ty > 0.2 && ty < wall && (tx / ty).abs() < 0.32 && best.map(|b| ty < b.1).unwrap_or(true) {
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

/// 16x16 human-like figure (head with two eyes, shoulders, arms, torso, legs). 1 = lit pixel.
const FIGURE: [u16; 16] = [
    0b0000011111100000,
    0b0000111111110000,
    0b0000110110110000,
    0b0000111111110000,
    0b0000011111100000,
    0b0000001111000000,
    0b0011111111111100,
    0b0111111111111110,
    0b0111011111101110,
    0b0110011111100110,
    0b0110011111100110,
    0b0110001111000110,
    0b0000001111000000,
    0b0000011001100000,
    0b0000011001100000,
    0b0000111001110000,
];

fn sprite(u: i32, v: i32) -> bool {
    if !(0..16).contains(&u) || !(0..16).contains(&v) {
        return false;
    }
    FIGURE[v as usize] & (0x8000 >> u) != 0
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
    fn can_pause(&self) -> bool { true }
    fn is_paused(&self) -> bool { self.paused }
    fn set_paused(&mut self, p: bool) {
        if self.alive {
            self.paused = p;
            self.last = Instant::now();
        }
    }
    fn press(&mut self, b: Btn, _t: &Trng) -> bool {
        if !self.started {
            if b == Btn::Ok {
                self.started = true;
                TUTORIAL_SEEN.store(true, Ordering::Relaxed);
                self.last = Instant::now();
            }
            return true;
        }
        if self.paused {
            if b == Btn::Ok {
                self.paused = false;
                self.last = Instant::now();
            }
            return true;
        }
        if b == Btn::Ok {
            self.fire();
            return true;
        }
        false
    }

    fn held(&mut self, down: &[bool; 6]) {
        self.fwd = (down[1] as i32) - (down[2] as i32);
        self.turn = (down[4] as i32) - (down[3] as i32);
        self.fire_held = down[0];
    }

    fn update(&mut self, t: &Trng) -> bool {
        if !self.alive || self.paused || !self.started {
            return false;
        }
        // time based, so the speed does not depend on how often this is called (smooth and responsive)
        let dt = self.last.elapsed().as_secs_f32().min(0.08);
        self.last = Instant::now();
        let approach = |v: f32, target: f32, acc: f32| -> f32 {
            let step = acc * dt;
            if v < target - step {
                v + step
            } else if v > target + step {
                v - step
            } else {
                target
            }
        };
        // turning starts gently (easy to aim) and speeds up while the key stays down
        if self.turn != 0 {
            self.turn_held = (self.turn_held + dt).min(TURN_RAMP);
        } else {
            self.turn_held = 0.0;
        }
        let speed = TURN_MIN + (TURN_MAX - TURN_MIN) * self.turn_held / TURN_RAMP;
        self.turn_v = approach(self.turn_v, self.turn as f32 * speed, TURN_ACC);
        self.move_v = approach(self.move_v, self.fwd as f32 * MOVE_MAX, MOVE_ACC);
        let mut changed = self.turn_v != 0.0 || self.move_v != 0.0;
        self.a += self.turn_v * dt;
        if self.move_v != 0.0 {
            let step = self.move_v * dt;
            let (nx, ny) = (self.px + self.a.cos() * step, self.py + self.a.sin() * step);
            if !is_wall(nx, self.py) {
                self.px = nx;
            }
            if !is_wall(self.px, ny) {
                self.py = ny;
            }
        }
        self.fire_cd = (self.fire_cd - dt).max(0.0);
        if self.fire_held {
            self.fire(); // holding OK keeps shooting
        }
        if self.flash > 0.0 {
            self.flash = (self.flash - dt).max(0.0);
            changed = true;
        }
        if self.hurt_cd > 0.0 {
            self.hurt_cd = (self.hurt_cd - dt).max(0.0);
            changed = true;
        }
        let (px, py) = (self.px, self.py);
        let mut hurt = false;
        let mut shot_at_you = false;
        for e in self.enemies.iter_mut().filter(|e| e.hp > 0) {
            let (rx, ry) = (px - e.x, py - e.y);
            let d = (rx * rx + ry * ry).sqrt();
            let keep_away = if e.kind == 0 { 3.2 } else { 0.6 };
            e.cd = (e.cd - dt).max(0.0);
            if d < 10.0 && d > keep_away {
                let (sx, sy) = (rx / d * 0.7 * dt, ry / d * 0.7 * dt);
                if !is_wall(e.x + sx, e.y) {
                    e.x += sx;
                }
                if !is_wall(e.x, e.y + sy) {
                    e.y += sy;
                }
                changed = true;
            } else if d <= 0.6 {
                hurt = true;
            }
            // shooting: needs a clear line to you; one shot in three hits
            if e.cd <= 0.0 && d < 8.0 && line_clear(e.x, e.y, px, py) {
                e.cd = 2.2 + (crate::rng::below(t, 100).unwrap_or(0) as f32) / 60.0;
                if crate::rng::below(t, 3).unwrap_or(0) == 0 {
                    shot_at_you = true;
                }
                self.snd = Some(Snd::Tick);
            }
        }
        if shot_at_you {
            hurt = true;
        }
        if hurt && self.hurt_cd <= 0.0 {
            self.hp -= 1;
            self.hurt_cd = 0.7;
            self.snd = Some(Snd::Crash);
            changed = true;
        }
        if self.hp <= 0 {
            self.alive = false;
            changed = true;
        } else if self.enemies.iter().all(|e| e.hp <= 0) {
            self.alive = false;
            self.won = true;
            LEVEL.store((self.level + 1) % 5, Ordering::Relaxed);
            changed = true;
        }
        changed || self.fire_cd > 0.0
    }

    fn draw(&self, c: &mut Canvas, _best: u32) {
        let (dx, dy) = (self.a.cos(), self.a.sin());
        let (plx, ply) = (-dy * 0.66, dx * 0.66);
        let mid = VIEW_TOP + VIEW_H / 2;
        // ---- floor and ceiling: a light dither, denser towards the viewer (kept cheap so the frame rate stays high)
        for y in VIEW_TOP..=VIEW_BOT {
            let dist = (y - mid).abs();
            if dist < 2 {
                continue;
            }
            let (stride, on_row) = if y > mid {
                if dist < 8 { (4, y & 3 == 0) } else if dist < 18 { (4, y & 1 == 0) } else { (2, true) }
            } else {
                (4, dist < 12 && y & 3 == 0)
            };
            if !on_row {
                continue;
            }
            let mut x = if (y / 2) & 1 == 0 { 0 } else { stride / 2 };
            while x < 128 {
                c.dot(x, y, true);
                x += stride;
            }
        }
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
            let hit = map().get(my.max(0) as usize).map(|r| r.as_bytes().get(mx.max(0) as usize).copied().unwrap_or(b'#')).unwrap_or(b'#');
            let perp = (if side == 0 { sdx - ddx } else { sdy - ddy }).max(0.05);
            zbuf[col] = perp;
            let h = ((VIEW_H as f32) / perp) as i32;
            let y0 = (mid - h / 2).max(VIEW_TOP);
            let y1 = (mid + h / 2).min(VIEW_BOT);
            let level = if perp < 1.6 { 0 } else if perp < 3.2 { 1 } else if perp < 5.5 { 2 } else { 3 };
            let level = if side == 1 && level < 3 { level + 1 } else { level };
            let x = (col * 2) as i32;
            if hit == b'%' {
                // slime pit wall: vertical stripes
                for y in y0..=y1 {
                    for xx in x..x + 2 {
                        if y == y0 || y == y1 || (xx / 2 + level as i32) % 2 == 0 {
                            c.dot(xx, y, true);
                        }
                    }
                }
            } else if level == 0 {
                c.fill_rect(x as usize, y0 as usize, 2, (y1 - y0 + 1).max(1) as usize, true);
            } else {
                for y in y0..=y1 {
                    for xx in x..x + 2 {
                        if y == y0 || y == y1 || shade(level, xx, y) {
                            c.dot(xx, y, true);
                        }
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
            let bot = (mid + h / 2).min(VIEW_BOT);
            let left = sx - w / 2;
            for xx in left..(sx + w / 2) {
                let col = (xx / 2) as usize;
                if xx < 0 || xx >= 128 || col >= NCOL || ty >= zbuf[col] {
                    continue;
                }
                let u = (xx - left) * 16 / w.max(1);
                for y in top..=bot {
                    let v = (y - (mid - h / 2)) * 16 / h.max(1);
                    if sprite(u, v) {
                        c.dot(xx, y, true);
                    } else if sprite(u - 1, v) || sprite(u + 1, v) || sprite(u, v - 1) || sprite(u, v + 1) {
                        c.dot(xx, y, false); // dark outline so the figure stands out from the wall behind it
                    }
                }
            }
        }
        // ---- minimap (top right): the 16 x 16 cells around you at 2 pixels per cell. You are always in the middle
        // as a big blinking square with a pointer; enemies blink as dots.
        {
            let (mx0, my0) = (95i32, 1i32);
            c.fill_rect((mx0 - 1) as usize, (my0 - 1) as usize, 34, 34, false);
            c.rect((mx0 - 1) as usize, (my0 - 1) as usize, 34, 34);
            let (ox, oy) = (self.px as i32 - 8, self.py as i32 - 8);
            for (yy, row) in map().iter().enumerate() {
                let j = yy as i32 - oy;
                if !(0..16).contains(&j) {
                    continue;
                }
                for (xx, ch) in row.bytes().enumerate() {
                    let i = xx as i32 - ox;
                    if ch != b'.' && (0..16).contains(&i) {
                        c.fill_rect((mx0 + i * 2) as usize, (my0 + j * 2) as usize, 2, 2, true);
                    }
                }
            }
            if (self.born.elapsed().as_millis() / 250) % 2 == 0 {
                for e in self.enemies.iter().filter(|e| e.hp > 0) {
                    let (ex, ey) = (((e.x - ox as f32) * 2.0) as i32, ((e.y - oy as f32) * 2.0) as i32);
                    if (0..32).contains(&ex) && (0..32).contains(&ey) {
                        c.dot(mx0 + ex, my0 + ey, true);
                        c.dot(mx0 + ex + 1, my0 + ey, false);
                    }
                }
            }
            let (pxm, pym) = (mx0 + ((self.px - ox as f32) * 2.0) as i32, my0 + ((self.py - oy as f32) * 2.0) as i32);
            c.fill_rect((pxm - 2).max(0) as usize, (pym - 2).max(0) as usize, 5, 5, false);
            if (self.born.elapsed().as_millis() / 150) % 2 == 0 {
                c.fill_rect((pxm - 1).max(0) as usize, (pym - 1).max(0) as usize, 3, 3, true);
            } else {
                c.dot(pxm, pym, true);
            }
            c.line(pxm, pym, pxm + (self.a.cos() * 5.0) as i32, pym + (self.a.sin() * 5.0) as i32);
        }
        // ---- crosshair and pistol (it bobs while you walk)
        c.dot(64, mid, true);
        c.dot(62, mid, true);
        c.dot(66, mid, true);
        c.dot(64, mid - 2, true);
        c.dot(64, mid + 2, true);
        let bob = if self.move_v.abs() > 0.3 { ((self.born.elapsed().as_millis() as f32 / 110.0).sin() * 2.0) as i32 } else { 0 };
        let gy = 40 + bob + (self.flash * 20.0) as i32; // the pistol kicks up when it fires
        c.fill_rect(62, (gy + 1) as usize, 4, 5, true); // barrel
        c.fill_rect(60, (gy + 6) as usize, 8, 4, true); // slide
        c.fill_rect(61, (gy + 10) as usize, 6, (VIEW_BOT - gy - 9).max(1) as usize, true); // grip
        c.fill_rect(63, (gy + 2) as usize, 2, 3, false); // muzzle hole
        if self.flash > 0.0 {
            c.line(64, gy, 58, gy - 7);
            c.line(64, gy, 70, gy - 7);
            c.line(64, gy, 64, gy - 9);
        }
        if self.hurt_cd > 0.4 {
            c.invert_rect(0, VIEW_TOP as usize, 128, 54);
        }
        // ---- status bar: health, face, level and kills
        c.fill_rect(0, 54, 128, 10, false);
        c.fill_rect(0, 54, 128, 1, true);
        let pct = (self.hp.max(0) * 100 / MAX_HP) as u32;
        c.text(1, 54, &format!("{}%", pct));
        c.rect(59, 54, 10, 10);
        if self.hp <= 0 {
            c.dot(62, 57, true);
            c.dot(63, 58, true);
            c.dot(66, 57, true);
            c.dot(65, 58, true);
            c.line(62, 61, 66, 62);
        } else {
            c.dot(62, 57, true);
            c.dot(66, 57, true);
            if self.hurt_cd > 0.3 {
                c.dot(64, 61, true);
                c.dot(64, 62, true); // ouch
            } else if pct > 60 {
                c.dot(62, 61, true);
                c.dot(63, 62, true);
                c.dot(64, 62, true);
                c.dot(65, 62, true);
                c.dot(66, 61, true);
            } else if pct > 30 {
                c.line(62, 61, 66, 61);
            } else {
                c.dot(62, 62, true);
                c.dot(63, 61, true);
                c.dot(64, 61, true);
                c.dot(65, 61, true);
                c.dot(66, 62, true);
                c.dot(60, 60, true); // blood
            }
        }
        c.text(78, 54, &format!("E1M{} {}/{}", self.level + 1, self.kills, self.enemies.len()));
        if !self.started {
            c.fill_rect(4, 2, 120, 50, false);
            c.rect(4, 2, 120, 50);
            c.text(10, 5, "UP/DOWN   walk");
            c.text(10, 16, "LEFT/RIGHT turn");
            c.text(10, 27, "OK shoot, kill all");
            c.text(10, 38, "OK = START");
            c.invert_rect(8, 39, 112, 11);
        }
        if !self.alive {
            c.fill_rect(6, 16, 116, 42, false);
            c.rect(6, 16, 116, 42);
            c.text_big(12, 19, if self.won { "CLEARED!" } else { "YOU DIED" }, 2);
            c.text(12, 44, if self.won { "OK = next level" } else { "OK retry  BACK exit" });
        }
    }

    fn score(&self) -> u32 { self.kills * 100 + if self.won { self.hp.max(0) as u32 * 50 } else { 0 } }
    fn over(&self) -> bool { !self.alive }
    fn take_sound(&mut self) -> Option<Snd> { self.snd.take() }
}
