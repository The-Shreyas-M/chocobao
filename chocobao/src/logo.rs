// logo.rs - the ChocoBao start-up animation and the HOME screen. Pure drawing code: no hardware, no allocation, integer
// maths only; every frame is a function of time.
//
// BOOT (about 4.8 s): a silicon chip draws itself (outline, pins, four BIO cores lighting up one by one), a padlock snaps
// shut on it (secure), the chip dissolves into random noise (the hardware TRNG) and the noise settles into the name
// "ChocoBao" next to a bun, then "secure. fun. open." and "by Shreyas".
// HOME: a bao spinning on the turntable of a microwave while chocolate melts over it, a chocolate bar beside it.
use crate::oled::{Canvas, H, W};

pub const BOOT_MS: u32 = 4800;

const BUN: [u8; 8] = [0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0xFF, 0x7E, 0xFF];

/// Parabolic sine: `a` is the phase in 0..256, the result is -64..=64.
fn sn(a: i32) -> i32 {
    let a = a.rem_euclid(256);
    if a < 128 {
        a * (128 - a) / 64
    } else {
        -((a - 128) * (256 - a) / 64)
    }
}
fn cosn(a: i32) -> i32 { sn(a + 64) }

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA6B) ^ seed.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 13;
    h
}

fn get(c: &Canvas, x: usize, y: usize) -> bool { c.fb[(y / 8) * W + x] & (1 << (y % 8)) != 0 }

/// Blend two pictures pixel by pixel: `p` percent of the pixels come from `b`, the rest from `a`; `noise` percent of
/// all pixels are then replaced by random ones. Which pixel switches when is fixed by a hash, so it looks like a dissolve.
fn dissolve(out: &mut Canvas, a: &Canvas, b: &Canvas, p: u32, noise: u32, seed: u32) {
    for y in 0..H {
        for x in 0..W {
            let h = hash(x as u32, y as u32, 7) % 100;
            let src = if h < p { get(b, x, y) } else { get(a, x, y) };
            let n = hash(x as u32, y as u32, seed) % 100;
            let v = if n < noise { hash(x as u32, y as u32, seed + 1) & 1 == 1 } else { src };
            out.dot(x as i32, y as i32, v);
        }
    }
}

// ------------------------------------------------------------------ boot
const CHIP_X: i32 = 44;
const CHIP_Y: i32 = 13;
const CHIP: i32 = 40;

fn pins(c: &mut Canvas, count: u32) {
    // 6 pins per side, appear one after the other (top, right, bottom, left)
    let mut n = 0;
    for side in 0..4 {
        for k in 0..6 {
            n += 1;
            if n > count {
                return;
            }
            let o = 5 + k * 6;
            match side {
                0 => c.fill_rect((CHIP_X + o) as usize, (CHIP_Y - 4) as usize, 2, 4, true),
                1 => c.fill_rect((CHIP_X + CHIP) as usize, (CHIP_Y + o) as usize, 4, 2, true),
                2 => c.fill_rect((CHIP_X + o) as usize, (CHIP_Y + CHIP) as usize, 2, 4, true),
                _ => c.fill_rect((CHIP_X - 4) as usize, (CHIP_Y + o) as usize, 4, 2, true),
            }
        }
    }
}

fn chip_scene(c: &mut Canvas, t: u32) {
    // outline grows along its edges during the first 500 ms
    let p = (t.min(500) as i32) * 4 * CHIP / 500; // total perimeter length drawn so far
    let seg = |from: i32| (p - from).clamp(0, CHIP);
    let (x, y) = (CHIP_X, CHIP_Y);
    c.fill_rect(x as usize, y as usize, seg(0) as usize + 1, 1, true);
    c.fill_rect((x + CHIP) as usize, y as usize, 1, seg(CHIP) as usize + 1, true);
    c.fill_rect((x + CHIP - seg(2 * CHIP)) as usize, (y + CHIP) as usize, seg(2 * CHIP) as usize + 1, 1, true);
    c.fill_rect(x as usize, (y + CHIP - seg(3 * CHIP)) as usize, 1, seg(3 * CHIP) as usize + 1, true);
    if t > 350 {
        pins(c, (t - 350) / 25);
    }
    // four BIO cores light up in turn, then pulse
    for i in 0..4u32 {
        let (cx, cy) = (x + 4 + (i as i32 % 2) * 18, y + 4 + (i as i32 / 2) * 18);
        let start = 750 + i * 170;
        if t >= start {
            let on = t - start;
            c.rect(cx as usize, cy as usize, 14, 14);
            let fill = (on.min(200) as i32 * 12 / 200).max(0);
            c.fill_rect((cx + 1) as usize, (cy + 1) as usize, 12, fill as usize, true);
            if t > 1500 && ((t / 120 + i) % 4 == 0) {
                c.invert_rect((cx + 3) as usize, (cy + 3) as usize, 8, 8);
            }
        }
    }
    // padlock snaps shut in the middle (secure)
    if t > 1350 {
        let k = (t - 1350).min(350) as i32;
        c.fill_rect(56, 20, 16, 16, false);
        c.fill_rect(58, 27, 12, 9, true); // lock body
        c.fill_rect(63, 30, 2, 3, false); // keyhole
        let lift = (3 - k * 3 / 350).max(0); // the shackle drops from open to closed
        c.fill_rect(60, (22 - lift) as usize, 1, 5, true);
        c.fill_rect(67, (22 - lift) as usize, 1, 5, true);
        c.fill_rect(60, (21 - lift) as usize, 8, 1, true);
    }
}

fn title_scene(c: &mut Canvas) {
    c.icon(8, 24, &BUN);
    c.text_big(24, 22, "ChocoBao", 2);
}

/// Full start-up animation, `t` = milliseconds since start.
pub fn boot_frame(c: &mut Canvas, t: u32) {
    let mut chip = Canvas::new();
    chip_scene(&mut chip, t.min(2100));
    if t < 1900 {
        c.fb.copy_from_slice(&chip.fb);
    } else if t < 3400 {
        // 1900-2500: the chip dissolves into random noise (the TRNG); 2500-3400: the noise settles into the title
        let blank = Canvas::new();
        if t < 2500 {
            let p = (t - 1900) * 100 / 600; // chip -> blank
            dissolve(c, &chip, &blank, p, 40 * (t - 1900) / 600, t / 60);
        } else {
            let mut title = Canvas::new();
            title_scene(&mut title);
            let p = ((t - 2500) * 100 / 900).min(100); // blank -> title
            dissolve(c, &blank, &title, p, 40 - 40 * p / 100, t / 60);
        }
    } else {
        title_scene(c);
    }
    if t > 3500 {
        let tag = "secure. fun. open.";
        let n = ((t - 3500) / 40) as usize;
        c.text(8, 44, &tag[..n.min(tag.len())]);
    }
    if t > 4150 {
        let by = "by Shreyas";
        let n = ((t - 4150) / 50) as usize;
        c.text(64, 55, &by[..n.min(by.len())]);
    }
}

// ------------------------------------------------------------------ home
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

/// The HOME screen. Landscape microwave on the left, chocolate bar on the right.
pub fn home_frame(c: &mut Canvas, t: u32) {
    c.text(2, 0, "ChocoBao - by Shreyas");
    // --- microwave body, window, control panel
    c.rect(0, 11, 90, 53);
    c.rect(3, 14, 58, 47);
    c.rect(65, 15, 22, 11);
    let left = 59 - (t / 1000) % 60; // cooking time counting down
    c.text(70, 17, &format!("{:02}s", left));
    c.rect(66, 31, 20, 5);
    c.rect(66, 39, 20, 5);
    let ka = (t / 30) as i32; // timer knob with a pointer that turns
    for dx in -4..=4i32 {
        let h = Canvas::dome_h(4, 4, dx);
        c.dot(76 + dx, 53 - h, true);
        c.dot(76 + dx, 53 + h, true);
    }
    c.line(76, 53, 76 + cosn(ka) * 3 / 64, 53 + sn(ka) * 3 / 64);

    // --- turntable plate (ellipse) and its marker
    let (cx, pcy) = (32i32, 53i32);
    for dx in -24..=24i32 {
        let h = Canvas::dome_h(24, 4, dx);
        c.dot(cx + dx, pcy - h, true);
        c.dot(cx + dx, pcy + h, true);
    }
    let a = (t / 9) as i32;
    let (mx, my) = (cx + 24 * cosn(a) / 64, pcy + 4 * sn(a) / 64);
    c.fill_rect((mx - 1).max(0) as usize, (my - 1).max(0) as usize, 2, 2, true);

    // --- the bao: a shaded round bun, pleats twisting into the top knot; they sweep around as it turns
    let base = 51i32;
    let ry = 17i32;
    let rx = 11 + 6 * cosn(a).abs() / 64; // the bun turns: its width breathes
    let top = base - ry;
    for dx in -rx..=rx {
        let h = Canvas::dome_h(rx, ry, dx);
        for y in (base - h)..base {
            // light comes from the upper left: the lower right is dithered darker
            let u = dx * 100 / rx;
            let v = (y - top) * 100 / ry;
            let s = u + v - 30;
            let (xx, yy) = (cx + dx, y);
            let lit = if s > 150 {
                false
            } else if s > 100 {
                (xx + yy) & 1 == 0 && (xx & 1) == 0
            } else if s > 55 {
                (xx + yy) & 1 == 0
            } else {
                true
            };
            if lit {
                c.dot(xx, yy, true);
            }
        }
    }
    for k in 0..4i32 {
        let ph = a + k * 64;
        if cosn(ph) > 8 {
            let px = cx + rx * sn(ph) / 64;
            c.line_set(cx, top + 1, px, base - 2, false); // a pleat from the knot to the rim
        }
    }
    c.fill_rect((cx - 1) as usize, (top - 2) as usize, 3, 2, true); // the knot on top
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
    // seeds on the surface travel around with the rotation (dark dots on the light body)
    for k in 0..6i32 {
        let ph = a + k * 43;
        if cosn(ph) > 6 {
            c.dot(cx + rx * 9 / 10 * sn(ph) / 64, base - 5 - (k % 3) * 3, false);
        }
    }
    c.dot(cx - rx / 2, top + 3, true); // shine
    c.dot(cx - rx / 2 + 1, top + 4, true);
    c.dot(cx + rx / 3, top + 5, true);
    // heat shimmer rising from the bun
    for k in 0..4i32 {
        for j in 0..3i32 {
            let rise = ((t / 70) as i32 + j * 6 + k * 5) % 18;
            let y = top - 3 - rise;
            if y > 16 {
                c.dot(cx - 12 + k * 8 + sn(rise * 24 + k * 50) / 40, y, true);
            }
        }
    }

    // --- chocolate bar: two columns of squares, one corner broken off and melting
    c.rect(97, 14, 24, 3); // foil strip
    for r in 0..4usize {
        for q in 0..2usize {
            if r == 3 && q == 1 {
                continue;
            }
            let (x, y) = (97 + q * 12, 18 + r * 9);
            c.rect(x, y, 12, 9);
            c.fill_rect(x + 2, y + 2, 6, 1, true); // bevel highlight
            c.fill_rect(x + 2, y + 2, 1, 4, true);
        }
    }
    for k in 0..3i32 {
        let len = ((t / 150) as i32 + k * 3) % 9;
        c.fill_rect((111 + k * 4) as usize, 45, 1, len.max(1) as usize, true); // melting drips at the broken corner
    }
    if (t / 700) % 4 != 3 {
        c.text(97, 56, "BACK");
    }
}
