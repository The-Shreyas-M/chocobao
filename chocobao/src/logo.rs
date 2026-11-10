// logo.rs - the ChocoBao start-up animation (a chocolate bao bun with steam) and a small idle bun for HOME.
// Pure drawing code: no hardware, no allocation, integer maths only.
use crate::oled::Canvas;

pub const BOOT_MS: u32 = 3600;

/// Triangle wave -2..2 used for the steam wiggle.
fn wig(phase: u32) -> i32 {
    let w = (phase % 8) as i32;
    (if w < 4 { w } else { 8 - w }) - 2
}

fn steam(c: &mut Canvas, cx: i32, top: i32, t: u32, height: i32) {
    for i in 0..3i32 {
        let x0 = cx - 9 + 9 * i;
        for k in 0..height {
            let y = top - 2 - k * 2;
            let off = wig(t / 90 + (k as u32) * 2 + (i as u32) * 3);
            // fade out towards the top by skipping dots
            if k < height - 2 || (t / 120 + k as u32) % 2 == 0 {
                c.dot(x0 + off, y, true);
                c.dot(x0 + off, y - 1, true);
            }
        }
    }
}

/// The bun: a dome with pleats, chocolate dither filling from the top down to `choc` (0..=100 percent).
fn bun(c: &mut Canvas, cx: i32, base: i32, rx: i32, ry: i32, choc: i32) {
    let top = base - ry;
    // chocolate (checkerboard dither) inside the dome
    let level = top + (ry * choc / 100);
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        for y in (base - h)..=level.min(base - 1) {
            if (dx + y) & 1 == 0 {
                c.dot(cx + dx, y, true);
            }
        }
    }
    // outline, 2 px thick
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        let hn = Canvas::dome_h(rx, ry, dx + 1).max(Canvas::dome_h(rx, ry, dx - 1));
        for y in (base - h.max(hn))..=(base - h) {
            c.dot(cx + dx, y, true);
            c.dot(cx + dx, y + 1, true);
        }
    }
    // pleats from the knot at the top
    c.fill_rect((cx - 2) as usize, (top - 1).max(0) as usize, 5, 3, true);
    for k in [-2i32, -1, 1, 2] {
        c.line(cx, top + 2, cx + k * rx / 3, base - 2);
    }
    // base
    c.fill_rect((cx - rx - 2).max(0) as usize, base as usize, (2 * rx + 5) as usize, 2, true);
}

/// Full start-up animation, `t` = milliseconds since start.
pub fn boot_frame(c: &mut Canvas, t: u32) {
    let (cx, rx, ry) = (33, 24, 20);
    // 0-900 ms: the bun rises from below the screen (ease-out)
    let p = (t.min(900) * 100 / 900) as i32; // 0..100
    let eased = p * (200 - p) / 100; // 0..100
    let base = 54 + (100 - eased) * 44 / 100;
    // 700-1800 ms: chocolate pours over it
    let choc = ((t.saturating_sub(700)).min(1100) * 100 / 1100) as i32;
    bun(c, cx, base, rx, ry, choc);
    if t > 1500 {
        steam(c, cx, base - ry - 2, t, 5);
    }
    // 1700+ ms: the name, typed out one letter per 110 ms
    if t > 1700 {
        let n = ((t - 1700) / 110) as usize;
        let choco = "Choco";
        let bao = "Bao";
        let a = n.min(choco.len());
        c.text_big(63, 8, &choco[..a], 2);
        if n > choco.len() {
            let b = (n - choco.len()).min(bao.len());
            c.text_big(63, 30, &bao[..b], 3);
        }
    }
    if t > 2900 {
        c.text(63, 54, "multitool");
    }
    // a short flash when the logo is complete
    if (3000..3080).contains(&t) {
        c.invert_rect(0, 0, 128, 64);
    }
}

/// Small bun with steam for the HOME screen (about 26 px wide), `t` in ms.
pub fn mini(c: &mut Canvas, x: i32, y: i32, t: u32) {
    // dome 12 wide, 8 high, with a dither "chocolate" top and steam
    let (rx, ry) = (12, 9);
    let base = y + ry;
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        for yy in (base - h)..base {
            if yy == base - h || (dx + yy) & 1 == 0 {
                c.dot(x + dx, yy, true);
            }
        }
    }
    c.fill_rect((x - rx - 1).max(0) as usize, base as usize, (2 * rx + 3) as usize, 2, true);
    steam(c, x, y - 1, t, 2);
}
