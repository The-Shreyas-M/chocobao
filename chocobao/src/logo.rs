// logo.rs - the ChocoBao start-up animation and the HOME logo: a white steamed bao bun with pleats and
// chocolate that keeps pouring over it from a jug (the chocolate is the dark part with a glossy shine; it runs
// down and drips over the edge). Pure drawing code: no hardware, no allocation, integer maths only.
use crate::oled::Canvas;

pub const BOOT_MS: u32 = 3800;

/// How far the chocolate hangs down at horizontal offset `dx`: an uneven lower edge with a few long drips.
fn drip(dx: i32) -> i32 {
    match (dx * 7 + 31).rem_euclid(11) {
        0 | 1 => 6,
        2 => 2,
        3 | 4 => 4,
        5 => 1,
        6 | 7 => 3,
        8 => 7,
        _ => 2,
    }
}

/// The bun. Lit pixels = white bun, pleats = dark lines, chocolate = dark with a lit glossy shine.
/// `choc` = how much of the dome is covered, 0..=100 percent. `t` (ms) makes the drips keep flowing.
fn bun(c: &mut Canvas, cx: i32, base: i32, rx: i32, ry: i32, choc: i32, t: u32) {
    let top = base - ry;
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        for y in (base - h)..base {
            c.dot(cx + dx, y, true);
        }
    }
    for k in [-2i32, -1, 0, 1, 2] {
        c.line_set(cx, top + 3, cx + k * rx * 2 / 5, base - 1, false);
    }
    if choc > 0 {
        for dx in -rx..=rx {
            let h = Canvas::dome_h(rx, ry, dx);
            let surface = base - h;
            // the drips keep creeping down and back: the chocolate never stops flowing
            let flow = if choc >= 100 { ((t / 140 + dx.rem_euclid(5) as u32) % 4) as i32 } else { 0 };
            let reach = ry * choc / 100 / 2 + drip(dx) * choc / 100 + flow;
            for y in (surface + 1)..(surface + 1 + reach).min(base) {
                c.dot(cx + dx, y, false);
            }
        }
        if choc > 40 {
            c.dot(cx - rx / 2, top + 3, true);
            c.dot(cx - rx / 2 + 1, top + 4, true);
            c.dot(cx - rx / 2 - 1, top + 5, true);
            c.dot(cx + rx / 3, top + 4, true);
        }
    }
    c.fill_rect((cx - rx - 3).max(0) as usize, base as usize, (2 * rx + 7) as usize, 2, true);
}

/// The chocolate jug with its spout at x-2; the stream runs from the spout down to `stream_to`.
fn jug(c: &mut Canvas, x: i32, y: i32, stream_to: i32, t: u32) {
    c.fill_rect(x as usize, y as usize, 9, 7, true);
    c.fill_rect((x + 2) as usize, (y + 2) as usize, 5, 3, false);
    c.line(x + 9, y + 1, x + 11, y + 3);
    c.line(x + 11, y + 3, x + 9, y + 6);
    c.line(x, y, x - 2, y + 2);
    let sx = x - 2;
    for yy in (y + 3)..stream_to {
        // dashes slide downwards so the stream visibly flows
        if (yy as u32 + t / 40) % 4 < 3 {
            c.dot(sx, yy, true);
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
    // chocolate keeps pouring from 900 ms to the end; the coat is complete after ~2.3 s
    let choc = ((t.saturating_sub(900)).min(1400) * 100 / 1400) as i32;
    bun(c, cx, base, rx, ry, choc, t);
    if t > 700 {
        let jx = if t < 900 { 90 - ((t - 700) as i32 * 44 / 200) } else { 46 };
        let surface = base - ry + 2 + (ry * choc / 100 / 2);
        if t >= 900 {
            jug(c, jx, 3, surface.max(10), t);
        } else {
            jug(c, jx, 3, 3, t);
        }
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

/// The HOME screen: the bun with chocolate still pouring over it, plus the hint to open the menu.
pub fn home_frame(c: &mut Canvas, t: u32) {
    bun(c, 28, 52, 22, 17, 100, t);
    let surface = 52 - 17 + 4;
    jug(c, 41, 2, surface, t);
    c.text_big(60, 6, "Choco", 2);
    c.text_big(60, 26, "Bao", 2);
    if (t / 700) % 4 != 3 {
        c.text(60, 52, "BACK = menu");
    }
}
