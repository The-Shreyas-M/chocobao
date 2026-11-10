// mandel.rs - Mandelbrot zoom in integer fixed point (Q4.28, no floating point), rendered a few rows at a time so the
// app loop never stalls. Grey levels are dithered with a 4x4 Bayer matrix onto the 1-bit OLED.
use std::time::{Duration, Instant};

pub const W: usize = 128;
pub const H: usize = 64;
const ONE: i64 = 1 << 28;
/// pixel step of the first frame: 3.5 units across 128 pixels
const START_STEP: i64 = ONE * 7 / 2 / W as i64;
/// stop zooming when one pixel is this many Q28 units (about 1.5e-7 across: Q28 runs out of precision soon after)
const MIN_STEP: i64 = 48;

/// (real, imaginary) of the zoom targets, in Q4.28.
const TARGETS: [(i64, i64, &str); 3] = [
    ((-0.7436439 * ONE as f64) as i64, (0.1318259 * ONE as f64) as i64, "Seahorse"),
    ((-1.7499577 * ONE as f64) as i64, 0, "Mini-brot"),
    ((0.2549870 * ONE as f64) as i64, (0.0005679 * ONE as f64) as i64, "Elephants"),
];

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

pub struct Mandel {
    pub target: usize,
    pub auto: bool,
    step: i64, // Q28 distance between neighbouring pixels
    level: u32, // zoom steps taken, raises the iteration limit
    row: usize, // next row to render; H = frame finished
    px: Vec<u8>, // 255 = inside the set (lit), banded glow outside
    pub zoom_x100: u32,
}

impl Mandel {
    pub fn new() -> Mandel {
        let mut m = Mandel { target: 0, auto: true, step: START_STEP, level: 0, row: 0, px: vec![0; W * H], zoom_x100: 100 };
        m.restart_zoom();
        m
    }

    pub fn name(&self) -> &'static str { TARGETS[self.target].2 }

    pub fn restart_zoom(&mut self) {
        self.step = START_STEP;
        self.level = 0;
        self.row = 0;
        self.update_zoom();
    }

    pub fn next_target(&mut self, dir: i32) {
        self.target = (self.target as i32 + dir).rem_euclid(TARGETS.len() as i32) as usize;
        self.restart_zoom();
    }

    fn update_zoom(&mut self) {
        self.zoom_x100 = ((START_STEP * 100) / self.step.max(1)).min(999_999_999) as u32;
    }

    /// One zoom step in or out (about 1.25x) and start a new frame.
    pub fn zoom(&mut self, inward: bool) {
        if inward {
            if self.step * 4 / 5 >= MIN_STEP {
                self.step = self.step * 4 / 5;
                self.level += 1;
            } else {
                self.restart_zoom(); // reached the precision limit: loop back to the start
                return;
            }
        } else if self.step * 5 / 4 <= START_STEP {
            self.step = self.step * 5 / 4;
            self.level = self.level.saturating_sub(1);
        }
        self.row = 0;
        self.update_zoom();
    }

    fn max_iter(&self) -> u32 { (40 + 6 * self.level).min(300) }

    /// Render rows for at most `budget`; returns true if any pixel changed.
    pub fn step_render(&mut self, budget: Duration) -> bool {
        if self.row >= H {
            return false;
        }
        let t0 = Instant::now();
        let (tx, ty, _) = TARGETS[self.target];
        let maxit = self.max_iter();
        let left = tx - self.step * (W as i64 / 2);
        let top = ty - self.step * (H as i64 / 2);
        while self.row < H && t0.elapsed() < budget {
            let y = self.row;
            let ci = top + self.step * y as i64;
            for x in 0..W {
                let cr = left + self.step * x as i64;
                let (mut zr, mut zi) = (0i64, 0i64);
                let mut it = 0u32;
                let mut zr2 = 0i64;
                let mut zi2 = 0i64;
                while it < maxit && zr2 + zi2 <= 4 * ONE {
                    zi = ((zr * zi) >> 27) + ci; // 2*zr*zi
                    zr = zr2 - zi2 + cr;
                    zr2 = (zr * zr) >> 28;
                    zi2 = (zi * zi) >> 28;
                    it += 1;
                }
                self.px[y * W + x] = if it >= maxit { 255 } else { (20 + (it & 15) * 10) as u8 };
            }
            self.row += 1;
        }
        true
    }

    pub fn finished(&self) -> bool { self.row >= H }

    /// Is this pixel lit (dithered)?
    pub fn lit(&self, x: usize, y: usize) -> bool {
        let v = self.px[y * W + x] as u32;
        v > (BAYER[y & 3][x & 3] as u32) * 16 + 8
    }
}
