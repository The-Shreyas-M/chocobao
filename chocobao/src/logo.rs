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

/// The HOME screen: the same pool, calm and gently moving, plus the hint to open the menu.
pub fn home_frame(c: &mut Canvas, t: u32) {
    liquid(c, t, LEVEL_END, None);
    stars(c, t);
    xor_text(c, 16, 14, "ChocoBao", 2);
    if (t / 700) % 4 != 3 {
        xor_text(c, 31, 54, "BACK = menu", 1);
    }
}
