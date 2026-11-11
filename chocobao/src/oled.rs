// oled.rs - minimal SSD1306 128x64 I2C OLED driver for ChocoBao.
//
// * Talks to the display at I2C address 0x3C on the Dabao's I2C0 bus (PB11 = SCL, PB12 = SDA, 3.3 V).
// * Keeps a 1 KiB frame buffer in RAM; `flush` sends it one 128-byte page at a time.
// * Text uses the 6x12 1-bit font from the Xous bootloader (originally from the embedded-graphics crate).
// * Never panics: every I2C failure just makes the call return false / None, so a missing or confused
//   display cannot take the rest of the app down.
// * No floating point, nothing is written to ReRAM.
use bao1x_api::{I2cApi, I2cResult};
use bao1x_hal::i2c::I2c;

const ADDR: u8 = 0x3C;
pub const W: usize = 128;
pub const H: usize = 64;

const FONT: &[u8] = include_bytes!("font6x12_1bpp.raw");
const FONT_W: usize = 6;
const FONT_H: usize = 12;
const FONT_IMG_W: usize = 96; // the font image is 96 pixels wide: 16 characters per row

fn isqrt(n: u32) -> u32 {
    let (mut x, mut r) = (n, 0u32);
    let mut bit = 1u32 << 30;
    while bit > x {
        bit >>= 2;
    }
    while bit != 0 {
        if x >= r + bit {
            x -= r + bit;
            r = (r >> 1) + bit;
        } else {
            r >>= 1;
        }
        bit >>= 2;
    }
    r
}

pub type Frame = [u8; W * H / 8];

/// Frame buffer + drawing primitives. No hardware access, so it can never block or fail.
pub struct Canvas {
    pub fb: Frame,
}

const INIT: &[u8] = &[
    0xD5, 0x80, // clock divide / oscillator
    0xA8, 0x3F, // multiplex ratio: 64 rows
    0xD3, 0x00, // display offset 0
    0x40, // start line 0
    0x8D, 0x14, // charge pump on
    0x20, 0x00, // horizontal addressing mode
    0xA1, // segment remap (flip horizontally)
    0xC8, // COM scan direction (flip vertically)
    0xDA, 0x12, // COM pins config for 128x64
    0x81, 0xCF, // contrast
    0xD9, 0xF1, // pre-charge
    0xDB, 0x40, // VCOMH level
    0xA4, // resume from RAM
    0xA6, // normal (not inverted)
];

impl Canvas {
    pub fn new() -> Canvas { Canvas { fb: [0; W * H / 8] } }

    pub fn clear(&mut self) { self.fb.fill(0); }

    fn pixel(&mut self, x: usize, y: usize, on: bool) {
        if x >= W || y >= H {
            return;
        }
        let i = (y / 8) * W + x;
        let m = 1u8 << (y % 8);
        if on {
            self.fb[i] |= m;
        } else {
            self.fb[i] &= !m;
        }
    }

    /// Draw text with its top-left corner at (x, y) in pixels. 21 characters fit on a line, 5 lines on screen.
    pub fn text(&mut self, x: usize, y: usize, s: &str) {
        let mut cx = x;
        for ch in s.chars() {
            let idx = if ch >= ' ' && ch <= '~' { ch as usize - ' ' as usize } else { 0 };
            let (col, row) = (idx % (FONT_IMG_W / FONT_W), idx / (FONT_IMG_W / FONT_W));
            for gy in 0..FONT_H {
                for gx in 0..FONT_W {
                    let bit = col * FONT_W + gx + FONT_IMG_W * (row * FONT_H + gy);
                    let on = FONT[bit / 8] & (1 << (7 - bit % 8)) != 0;
                    self.pixel(cx + gx, y + gy, on);
                }
            }
            cx += FONT_W;
        }
    }

    /// Filled (or cleared) rectangle.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, on: bool) {
        // one byte covers 8 pixel rows, so work column by column with masks (about 8x faster than pixel by pixel)
        let (x1, y1) = (x.saturating_add(w).min(W), y.saturating_add(h).min(H));
        if x >= x1 || y >= y1 {
            return;
        }
        for page in (y / 8)..=((y1 - 1) / 8) {
            let (top, bot) = (page * 8, page * 8 + 8);
            let (a, b) = (y.max(top) - top, y1.min(bot) - top); // rows a..b inside this page
            let mask = (0xFFu16 << a) as u8 & (0xFFu16 >> (8 - b)) as u8;
            let row = &mut self.fb[page * W + x..page * W + x1];
            if on {
                for v in row.iter_mut() {
                    *v |= mask;
                }
            } else {
                for v in row.iter_mut() {
                    *v &= !mask;
                }
            }
        }
    }

    /// 1-pixel outline rectangle.
    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(x, y, w, 1, true);
        self.fill_rect(x, y + h - 1, w, 1, true);
        self.fill_rect(x, y, 1, h, true);
        self.fill_rect(x + w - 1, y, 1, h, true);
    }

    /// Invert a rectangle (used for the selection bar in menus).
    pub fn invert_rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                if xx < W && yy < H {
                    self.fb[(yy / 8) * W + xx] ^= 1u8 << (yy % 8);
                }
            }
        }
    }


    /// Straight line (Bresenham).
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) { self.line_set(x0, y0, x1, y1, true); }

    /// Straight line that sets (`on`) or clears pixels.
    pub fn line_set(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, on: bool) {
        let (mut x, mut y) = (x0, y0);
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        for _ in 0..512 {
            if x >= 0 && y >= 0 {
                self.pixel(x as usize, y as usize, on);
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Set a single pixel (clipped).
    pub fn dot(&mut self, x: i32, y: i32, on: bool) {
        if x >= 0 && y >= 0 {
            self.pixel(x as usize, y as usize, on);
        }
    }

    /// Height of the upper half of an ellipse (rx, ry) at horizontal offset dx from its centre.
    pub fn dome_h(rx: i32, ry: i32, dx: i32) -> i32 {
        if dx.abs() > rx || rx == 0 {
            return 0;
        }
        isqrt((ry * ry * (rx * rx - dx * dx) / (rx * rx)) as u32) as i32
    }

    /// Small triangle arrow: dir 0 = up, 1 = down, 2 = left, 3 = right. Up/down are 5x3, left/right 3x5.
    pub fn arrow(&mut self, x: usize, y: usize, dir: u8) {
        match dir {
            0 => {
                self.fill_rect(x + 2, y, 1, 1, true);
                self.fill_rect(x + 1, y + 1, 3, 1, true);
                self.fill_rect(x, y + 2, 5, 1, true);
            }
            1 => {
                self.fill_rect(x, y, 5, 1, true);
                self.fill_rect(x + 1, y + 1, 3, 1, true);
                self.fill_rect(x + 2, y + 2, 1, 1, true);
            }
            2 => {
                self.fill_rect(x, y + 2, 1, 1, true);
                self.fill_rect(x + 1, y + 1, 1, 3, true);
                self.fill_rect(x + 2, y, 1, 5, true);
            }
            _ => {
                self.fill_rect(x, y, 1, 5, true);
                self.fill_rect(x + 1, y + 1, 1, 3, true);
                self.fill_rect(x + 2, y + 2, 1, 1, true);
            }
        }
    }

    /// 8x8 monochrome icon, one byte per row, most significant bit = leftmost pixel.
    pub fn icon(&mut self, x: usize, y: usize, rows: &[u8; 8]) {
        for (r, &bits) in rows.iter().enumerate() {
            for c in 0..8 {
                if bits & (0x80 >> c) != 0 {
                    self.pixel(x + c, y + r, true);
                }
            }
        }
    }

    /// Transition helper: everything to the right of column `x` is replaced by the previous picture, with a
    /// bright edge at the seam (the new picture "wipes in" from the left).
    pub fn wipe(&mut self, old: &Frame, x: usize) {
        for page in 0..H / 8 {
            for col in x.min(W)..W {
                self.fb[page * W + col] = old[page * W + col];
            }
        }
        if x < W {
            for y in 0..H {
                self.pixel(x, y, true);
            }
        }
    }

    /// Raised "soft button" panel: bright top/left edge, dithered shadow on the bottom/right, rounded corners.
    pub fn card(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w < 4 || h < 4 {
            return;
        }
        self.fill_rect(x + 1, y, w - 2, 1, true);
        self.fill_rect(x, y + 1, 1, h - 2, true);
        for i in 1..w - 1 {
            if (x + i) % 2 == 0 {
                self.pixel(x + i, y + h - 1, true);
            }
        }
        for j in 1..h - 1 {
            if (y + j) % 2 == 0 {
                self.pixel(x + w - 1, y + j, true);
            }
        }
    }

    /// Sunken panel: dithered shadow on the top/left, bright bottom/right edge.
    pub fn inset(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w < 4 || h < 4 {
            return;
        }
        for i in 1..w - 1 {
            if (x + i) % 2 == 0 {
                self.pixel(x + i, y, true);
            }
        }
        for j in 1..h - 1 {
            if (y + j) % 2 == 0 {
                self.pixel(x, y + j, true);
            }
        }
        self.fill_rect(x + 1, y + h - 1, w - 2, 1, true);
        self.fill_rect(x + w - 1, y + 1, 1, h - 2, true);
    }

    /// Text scaled up by an integer factor (each font pixel becomes scale x scale pixels).
    pub fn text_big(&mut self, x: usize, y: usize, s: &str, scale: usize) {
        let mut cx = x;
        for ch in s.chars() {
            let idx = if ch >= ' ' && ch <= '~' { ch as usize - ' ' as usize } else { 0 };
            let (col, row) = (idx % (FONT_IMG_W / FONT_W), idx / (FONT_IMG_W / FONT_W));
            for gy in 0..FONT_H {
                for gx in 0..FONT_W {
                    let bit = col * FONT_W + gx + FONT_IMG_W * (row * FONT_H + gy);
                    if FONT[bit / 8] & (1 << (7 - bit % 8)) != 0 {
                        self.fill_rect(cx + gx * scale, y + gy * scale, scale, scale, true);
                    }
                }
            }
            cx += FONT_W * scale;
        }
    }
}

/// The physical SSD1306. Only sends the part of the picture that changed.
pub struct Panel {
    i2c: I2c,
    sent: Frame,
    sent_valid: bool,
}

impl Panel {
    pub fn new() -> Panel { Panel { i2c: I2c::new(), sent: [0; W * H / 8], sent_valid: false } }

    fn cmds(&mut self, c: &[u8]) -> bool { matches!(self.i2c.i2c_write(ADDR, 0x00, c), Ok(I2cResult::Ack(_))) }

    /// (Re)initialise the controller. Returns false if nothing answers at 0x3C.
    pub fn init(&mut self) -> bool {
        self.sent_valid = false;
        self.cmds(&[0xAE]) && self.cmds(INIT) && self.cmds(&[0xAF])
    }

    /// Re-send the settings without switching the panel off (no flicker). If the OLED lost power for a moment
    /// (USB unplugged and plugged, a brown-out) it forgot them and shows nothing; this brings it back.
    pub fn refresh_settings(&mut self) -> bool {
        self.sent_valid = false;
        self.cmds(INIT) && self.cmds(&[0xAF])
    }

    /// Switch the panel off (it keeps its picture in memory but shows nothing and uses almost no power).
    pub fn display_off(&mut self) -> bool { self.cmds(&[0xAE]) }

    /// Make the next `push` resend everything.
    pub fn invalidate(&mut self) { self.sent_valid = false; }

    /// Send only the changed column range of each changed page. Returns false on any I2C failure
    /// (the next push then resends everything).
    pub fn push(&mut self, fb: &Frame) -> bool {
        for page in 0..H / 8 {
            let (new, old) = (&fb[page * W..(page + 1) * W], &self.sent[page * W..(page + 1) * W]);
            let (mut c0, mut c1) = (0usize, W - 1);
            if self.sent_valid {
                match (0..W).find(|&i| new[i] != old[i]) {
                    None => continue,
                    Some(f) => {
                        c0 = f;
                        c1 = (0..W).rev().find(|&i| new[i] != old[i]).unwrap_or(f);
                    }
                }
            }
            // ONE transaction per page: control byte 0x80 = "one command follows", repeated for the address
            // window, then control byte 0x40 = "all the rest is display data".
            let mut buf: Vec<u8> = Vec::with_capacity(16 + (c1 - c0 + 1));
            buf.extend_from_slice(&[
                0x21, 0x80, c0 as u8, 0x80, c1 as u8, 0x80, 0x22, 0x80, page as u8, 0x80, page as u8, 0x40,
            ]);
            buf.extend_from_slice(&new[c0..=c1]);
            let mut ok = matches!(self.i2c.i2c_write(ADDR, 0x80, &buf), Ok(I2cResult::Ack(_)));
            if !ok {
                // one immediate retry before giving up on this frame
                ok = matches!(self.i2c.i2c_write(ADDR, 0x80, &buf), Ok(I2cResult::Ack(_)));
            }
            if !ok {
                // the OLED did not answer: report it (the display thread re-initialises it) and resend everything later
                self.sent_valid = false;
                return false;
            }
            self.sent[page * W + c0..page * W + c1 + 1].copy_from_slice(&new[c0..=c1]);
        }
        self.sent_valid = true;
        true
    }
}
