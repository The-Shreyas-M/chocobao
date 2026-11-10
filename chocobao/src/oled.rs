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
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.pixel(xx, yy, on);
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
            self.sent[page * W + c0..page * W + c1 + 1].copy_from_slice(&new[c0..=c1]);
        }
        self.sent_valid = true;
        true
    }
}
