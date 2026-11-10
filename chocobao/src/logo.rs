// logo.rs - the ChocoBao start-up animation and the HOME screen: a drop falls into a small pool, the splash
// rings out, the liquid rises in two flowing wave layers (frothy dithered top, solid body) and the name
// "ChocoBao" sits half-submerged: the text is XOR-ed with the liquid so it stays readable on both sides of the
// surface. Twinkling stars above. Pure drawing code: no hardware, no allocation, integer maths only.
use crate::oled::{Canvas, H, W};

pub const BOOT_MS: u32 = 4200;

const LEVEL_END: i32 = 58; // percent of the screen height the liquid ends at
const DROP_AT: u32 = 250; // ms: the drop starts to fall
const HIT_AT: u32 = 700; // ms: it lands, the splash starts and the level begins to rise
const RISE_MS: i32 = 1800;

/// Parabolic sine: `a` is the phase in 0..256, the result is -64..=64.
fn sn(a: i32) -> i32 {
    let a = a.rem_euclid(256);
    if a < 128 {
        a * (128 - a) / 64
    } else {
        -((a - 128) * (256 - a) / 64)
    }
}

/// A wave of the given amplitude in pixels at phase `a`.
fn wave(a: i32, amp: i32) -> i32 { sn(a) * amp / 64 }

/// Liquid level in percent of the screen height.
fn level(t: u32, pour: bool) -> i32 {
    if !pour || t >= HIT_AT + RISE_MS as u32 {
        return LEVEL_END;
    }
    if t < HIT_AT {
        return 8;
    }
    let p = ((t - HIT_AT) as i32 * 100 / RISE_MS).min(100);
    let eased = p * (200 - p) / 100; // ease-out
    8 + (LEVEL_END - 8) * eased / 100
}

/// Draws the pool, the splash and the stars. `splash_t` = ms since the drop landed (None = calm).
fn liquid(c: &mut Canvas, t: u32, lv: i32, splash_t: Option<u32>) {
    let base = H as i32 - (H as i32 * lv / 100);
    for x in 0..W as i32 {
        let mut s1 = base + wave(x * 3 + (t / 7) as i32, 2);
        let mut s2 = base + 4 + wave(x * 5 - (t / 5) as i32, 2);
        if let Some(dt) = splash_t {
            let d = (x - 64).abs();
            if d < 70 {
                let amp = 9 * 400 / (400 + dt as i32) * (70 - d) / 70;
                let w = wave(d * 6 - (dt / 3) as i32, amp);
                s1 += w;
                s2 += w / 2;
            }
        }
        let s1 = s1.clamp(0, H as i32 - 1);
        let s2 = s2.clamp(s1, H as i32 - 1);
        c.dot(x, s1, true); // bright surface line
        for y in (s1 + 1)..s2 {
            if (x + y) & 1 == 0 {
                c.dot(x, y, true); // froth
            }
        }
        if s2 < H as i32 {
            c.fill_rect(x as usize, s2 as usize, 1, (H as i32 - s2) as usize, true);
        }
    }
}

fn stars(c: &mut Canvas, t: u32) {
    const POS: [(i32, i32); 5] = [(6, 4), (121, 5), (7, 21), (120, 20), (64, 3)];
    for (i, &(x, y)) in POS.iter().enumerate() {
        let ph = ((t / 180) as usize + i * 2) % 8;
        if ph < 2 {
            c.dot(x, y, true);
        } else if ph < 4 {
            c.dot(x, y, true);
            c.dot(x - 1, y, true);
            c.dot(x + 1, y, true);
            c.dot(x, y - 1, true);
            c.dot(x, y + 1, true);
        }
    }
}

/// Text that flips colour against the liquid behind it.
fn xor_text(c: &mut Canvas, x: usize, y: usize, s: &str, scale: usize) {
    let mut t = Canvas::new();
    if scale > 1 {
        t.text_big(x, y, s, scale);
    } else {
        t.text(x, y, s);
    }
    for (d, s) in c.fb.iter_mut().zip(t.fb.iter()) {
        *d ^= *s;
    }
}

/// Full start-up animation, `t` = milliseconds since start.
pub fn boot_frame(c: &mut Canvas, t: u32) {
    let lv = level(t, true);
    let splash = if t >= HIT_AT { Some(t - HIT_AT) } else { None };
    liquid(c, t, lv, splash);
    // the drop falls from above the screen into the pool
    if (DROP_AT..HIT_AT).contains(&t) {
        let y = -4 + ((t - DROP_AT) as i32 * 63 / (HIT_AT - DROP_AT) as i32);
        c.fill_rect(62, (y + 1).max(0) as usize, 4, 3, true);
        c.fill_rect(63, (y - 2).max(0) as usize, 2, 3, true);
    }
    if t > 1500 {
        stars(c, t);
    }
    // the name appears letter by letter once the liquid is rising
    if t > 1300 {
        let n = ((t - 1300) / 90) as usize;
        let name = "ChocoBao";
        xor_text(c, 16, 14, &name[..n.min(name.len())], 2);
    }
    if t > 2900 {
        let tag = "pocket multitool";
        let n = ((t - 2900) / 45) as usize;
        xor_text(c, 16, 54, &tag[..n.min(tag.len())], 1);
    }
}

/// How far the chocolate hangs down at horizontal offset `dx`: an uneven lower edge with a few long drips.
fn drip(dx: i32) -> i32 {
    match (dx * 7 + 31).rem_euclid(11) {
        0 | 1 => 5,
        2 => 2,
        3 | 4 => 3,
        5 => 1,
        6 | 7 => 2,
        8 => 6,
        _ => 2,
    }
}

fn cosn(a: i32) -> i32 { sn(a + 64) }

/// The HOME screen: a bao spinning on the turntable of a little microwave, melted chocolate running over it and a
/// chocolate bar beside it. Mostly dark: only outlines and small shapes are lit.
pub fn home_frame(c: &mut Canvas, t: u32) {
    // --- microwave body, window, control panel
    c.rect(0, 2, 68, 60);
    c.rect(3, 5, 42, 54);
    c.rect(48, 6, 17, 11);
    let left = 59 - (t / 1000) % 60; // countdown on the display, like cooking time
    c.text(52, 8, &format!("{:02}", left));
    for k in 0..3usize {
        c.rect(51, 22 + 9 * k, 11, 6);
    }
    // timer knob with a pointer that turns
    let ka = (t / 30) as i32;
    for dx in -4..=4i32 {
        let h = Canvas::dome_h(4, 4, dx);
        c.dot(56 + dx, 52 - h, true);
        c.dot(56 + dx, 52 + h, true);
    }
    c.line(56, 52, 56 + cosn(ka) * 3 / 64, 52 + sn(ka) * 3 / 64);

    // --- turntable plate (ellipse) with a marker that circles with the plate
    let (cx, pcy) = (24i32, 51i32);
    for dx in -17..=17i32 {
        let h = Canvas::dome_h(17, 4, dx);
        c.dot(cx + dx, pcy - h, true);
        c.dot(cx + dx, pcy + h, true);
    }
    let a = (t / 9) as i32;
    let (mx, my) = (cx + 17 * cosn(a) / 64, pcy + 4 * sn(a) / 64);
    c.fill_rect((mx - 1).max(0) as usize, (my - 1).max(0) as usize, 2, 2, true);

    // --- the bao: it turns, so its visible width breathes and the pleats slide
    let base = 49i32;
    let ry = 14i32;
    let rx = 8 + 7 * cosn(a).abs() / 64;
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        for y in (base - h)..base {
            c.dot(cx + dx, y, true);
        }
    }
    for k in [-1i32, 1] {
        let px = cx + k * rx * cosn(a) / 64 / 2 + (k * rx / 3);
        c.line_set(px, base - ry + 3, px + k * 2, base - 1, false);
    }
    // chocolate melting over the top, it keeps flowing
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        let surface = base - h;
        let flow = ((t / 130 + dx.rem_euclid(5) as u32) % 4) as i32;
        let reach = ry / 3 + drip(dx) + flow;
        for y in (surface + 1)..(surface + 1 + reach).min(base - 1) {
            c.dot(cx + dx, y, false);
        }
    }
    c.dot(cx - rx / 2, base - ry + 3, true); // glossy shine
    c.dot(cx - rx / 2 + 1, base - ry + 4, true);
    // heat shimmer rising from the bun (it is baking)
    for k in 0..3i32 {
        for j in 0..3i32 {
            let rise = ((t / 70) as i32 + j * 6 + k * 5) % 18;
            let y = base - ry - 2 - rise;
            if y > 8 {
                c.dot(cx - 9 + k * 9 + sn(rise * 24 + k * 50) / 40, y, true);
            }
        }
    }

    // --- title, chocolate bar, hint
    c.text(76, 4, "ChocoBao");
    c.rect(77, 16, 32, 3); // foil strip
    for r in 0..3usize {
        for q in 0..3usize {
            if r == 2 && q == 2 {
                continue; // the broken-off corner: that piece is the melted chocolate
            }
            let (x, y) = (77 + q * 11, 20 + r * 9);
            c.rect(x, y, 10, 8);
            c.fill_rect(x + 2, y + 2, 5, 1, true); // bevel highlight
            c.fill_rect(x + 2, y + 2, 1, 3, true);
        }
    }
    for q in 0..3i32 {
        let len = ((t / 160) as i32 + q * 4) % 9;
        c.fill_rect((82 + q * 11) as usize, 47, 1, len.max(1) as usize, true); // melting drips under the bar
    }
    if (t / 700) % 4 != 3 {
        c.text(74, 55, "BACK=menu");
    }
}
