// logo.rs - the ChocoBao start-up animation and the HOME logo: a white steamed bao bun with pleats, chocolate
// poured over it from a little jug (the chocolate is the dark part with a glossy shine, and it drips over the
// edge), steam curling above. Pure drawing code: no hardware, no allocation, integer maths only.
use crate::oled::Canvas;

pub const BOOT_MS: u32 = 3800;

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
            if k < height - 2 || (t / 120 + k as u32) % 2 == 0 {
                c.dot(x0 + off, y, true);
                c.dot(x0 + off, y - 1, true);
            }
        }
    }
}

/// How far the chocolate hangs down at horizontal offset `dx`: an uneven lower edge with a few long drips.
fn drip(dx: i32) -> i32 {
    let d = (dx * 7 + 31).rem_euclid(11);
    match d {
        0 | 1 => 6,
        2 => 2,
        3 | 4 => 4,
        5 => 1,
        6 | 7 => 3,
        8 => 7,
        _ => 2,
    }
}

/// The bun. `choc` = how much of the dome is covered by chocolate, 0..=100 percent.
/// Lit pixels = white bun. Pleats are dark lines. The chocolate is dark with a few lit shine dots.
fn bun(c: &mut Canvas, cx: i32, base: i32, rx: i32, ry: i32, choc: i32) {
    let top = base - ry;
    // 1. white body
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        for y in (base - h)..base {
            c.dot(cx + dx, y, true);
        }
    }
    // 2. pleats radiate from the knot at the top (dark lines on the white bun)
    for k in [-2i32, -1, 0, 1, 2] {
        c.line_set(cx, top + 3, cx + k * rx * 2 / 5, base - 1, false);
    }
    // 3. chocolate poured over the top: dark, with an uneven edge and drips
    if choc > 0 {
        for dx in -rx..=rx {
            let h = Canvas::dome_h(rx, ry, dx);
            let surface = base - h;
            let reach = ry * choc / 100 / 2 + (drip(dx) * choc / 100);
            for y in (surface + 1)..(surface + 1 + reach).min(base) {
                c.dot(cx + dx, y, false);
            }
        }
        // glossy shine
        if choc > 40 {
            c.dot(cx - rx / 2, top + 3, true);
            c.dot(cx - rx / 2 + 1, top + 4, true);
            c.dot(cx - rx / 2 - 1, top + 5, true);
            c.dot(cx + rx / 3, top + 4, true);
        }
    }
    // 4. a bamboo steamer tray under the bun
    c.fill_rect((cx - rx - 3).max(0) as usize, base as usize, (2 * rx + 7) as usize, 2, true);
}

/// The chocolate jug tilted over the bun, with the stream running down to the chocolate surface.
fn jug(c: &mut Canvas, x: i32, y: i32, stream_to: i32, pouring: bool) {
    // body
    c.fill_rect(x as usize, y as usize, 9, 7, true);
    c.fill_rect((x + 2) as usize, (y + 2) as usize, 5, 3, false);
    // handle
    c.line(x + 9, y + 1, x + 11, y + 3);
    c.line(x + 11, y + 3, x + 9, y + 6);
    // spout
    c.line(x, y, x - 2, y + 2);
    if pouring {
        let sx = x - 2;
        for yy in (y + 3)..stream_to {
            c.dot(sx, yy, (yy / 2) % 2 == 0 || yy < stream_to - 3);
        }
    }
}

/// Full start-up animation, `t` = milliseconds since start.
pub fn boot_frame(c: &mut Canvas, t: u32) {
    let (cx, rx, ry) = (33, 24, 20);
    // 0-800 ms: the bun rises from below the screen (ease-out)
    let p = (t.min(800) * 100 / 800) as i32;
    let eased = p * (200 - p) / 100;
    let base = 56 + (100 - eased) * 44 / 100;
    // 900-2300 ms: chocolate is poured
    let choc = ((t.saturating_sub(900)).min(1400) * 100 / 1400) as i32;
    bun(c, cx, base, rx, ry, choc);
    // the jug swings in from the right, pours, then leaves
    if (700..2600).contains(&t) {
        let jx = if t < 900 { 80 - ((t - 700) as i32 / 8) } else if t < 2300 { 55 } else { 55 + ((t - 2300) as i32 / 6) };
        let jy = 4;
        let pouring = (900..2300).contains(&t);
        let surface = base - ry + 2 + (ry * choc / 100 / 2);
        jug(c, jx, jy, surface.max(jy + 6), pouring);
    }
    if t > 2000 {
        steam(c, cx, base - ry - 2, t, 5);
    }
    // 2200+ ms: the name, typed out one letter per 110 ms
    if t > 2200 {
        let n = ((t - 2200) / 110) as usize;
        let choco = "Choco";
        let bao = "Bao";
        let a = n.min(choco.len());
        c.text_big(63, 8, &choco[..a], 2);
        if n > choco.len() {
            let b = (n - choco.len()).min(bao.len());
            c.text_big(63, 30, &bao[..b], 3);
        }
    }
    if t > 3200 {
        c.text(63, 54, "multitool");
    }
}

/// The HOME screen: the finished bun, steaming, plus the hint to open the menu.
pub fn home_frame(c: &mut Canvas, t: u32) {
    bun(c, 28, 50, 22, 17, 100);
    steam(c, 28, 31, t, 5);
    c.text_big(58, 6, "Choco", 2);
    c.text_big(58, 26, "Bao", 2);
    if (t / 700) % 4 != 3 {
        c.text(58, 52, "BACK = menu");
    }
}
