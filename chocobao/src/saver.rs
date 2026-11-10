// saver.rs - screensavers. Every one is a pure function of time (no state), uses few lit pixels and keeps moving so
// no pixel stays on (OLED burn-in). `kind` is chosen in Settings > Screensaver.
use crate::oled::Canvas;

pub const NAMES: [&str; 4] = ["Drifting bun", "Auto Pong", "Eyes", "Escape artist"];
pub const COUNT: usize = NAMES.len();

const I_BUN: [u8; 8] = [0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0xFF, 0x7E, 0xFF];

/// Triangle wave 0..=span that bounces back and forth as `t` grows.
fn tri(t: u32, span: u32) -> i32 {
    let p = t % (2 * span);
    (if p < span { p } else { 2 * span - p }) as i32
}

fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

pub fn draw(c: &mut Canvas, kind: usize, t: u32) {
    match kind {
        0 => c.icon(tri(t / 110, 118) as usize, tri(t / 170, 54) as usize, &I_BUN),
        1 => pong(c, t),
        2 => eyes(c, t),
        _ => walker(c, t),
    }
}

// ------------------------------------------------------------------ auto pong
fn ball_pos(t: u32) -> (i32, i32) { (4 + tri(t / 9 + 40, 112), 2 + tri(t / 13 + (t / 9000) * 7, 56)) }

fn pong(c: &mut Canvas, t: u32) {
    let (bx, by) = ball_pos(t);
    c.fill_rect(bx as usize, by as usize, 3, 3, true);
    // the paddles chase the ball with a little delay, so they sometimes look slightly late
    let ly = ball_pos(t.saturating_sub(260)).1 - 5;
    let ry = ball_pos(t.saturating_sub(180)).1 - 5;
    c.fill_rect(0, ly.clamp(0, 52) as usize, 2, 12, true);
    c.fill_rect(126, ry.clamp(0, 52) as usize, 2, 12, true);
    for y in (0..64).step_by(6) {
        c.dot(64, y, true);
        c.dot(64, y + 1, true);
    }
    let score = (t / 7000) % 10;
    c.text(50, 0, &format!("{}", score));
    c.text(72, 0, &format!("{}", (t / 11000) % 10));
}

// ------------------------------------------------------------------ eyes with moods
fn disc(c: &mut Canvas, cx: i32, cy: i32, r: i32) {
    for dy in -r..=r {
        let w = Canvas::dome_h(r, r, dy);
        c.fill_rect((cx - w).max(0) as usize, (cy + dy).max(0) as usize, (2 * w + 1) as usize, 1, true);
    }
}

fn eye(c: &mut Canvas, cx: i32, cy: i32, inner: i32, mood: u32, blink: i32, gaze: (i32, i32)) {
    const R: i32 = 15;
    // outline
    for dx in -R..=R {
        let h = Canvas::dome_h(R, R, dx);
        c.dot(cx + dx, cy - h, true);
        c.dot(cx + dx, cy + h, true);
        let w = Canvas::dome_h(R, R, dx);
        c.dot(cx - w, cy + dx, true);
        c.dot(cx + w, cy + dx, true);
    }
    // pupil
    let pr = if mood == 4 { 2 } else { 5 };
    disc(c, cx + gaze.0, cy + gaze.1, pr);
    c.fill_rect((cx + gaze.0 - 3).max(0) as usize, (cy + gaze.1 - 3).max(0) as usize, 1, 1, false); // shine
    // eyelids: upper lid height (relative to the centre) as a function of the column; everything above it is closed
    for dx in -R..=R {
        let h = Canvas::dome_h(R, R, dx);
        let s = dx * inner; // positive towards the nose
        let mut up = -R - 1;
        match mood {
            1 => {}
            2 => up = -R / 3 - s * 2 / 3,       // sad: outer corner droops
            3 => up = -R / 3 + s * 2 / 3,       // angry: inner corner drops
            5 => up = 1,                         // sleepy: half closed
            _ => {}
        }
        if blink > 0 {
            up = up.max(-R + 2 * R * blink / 100);
        }
        if up > -R - 1 {
            for y in (cy - h)..(cy + up).min(cy + h + 1) {
                c.dot(cx + dx, y, false);
            }
            if cy + up >= cy - h && cy + up <= cy + h {
                c.dot(cx + dx, cy + up, true);
            }
        }
        if mood == 1 {
            // happy: the lower lid pushes up in an arch
            let b = -R / 3 + dx * dx * 4 / (3 * R);
            for y in (cy + b + 1)..=(cy + h) {
                c.dot(cx + dx, y, false);
            }
            if cy + b <= cy + h {
                c.dot(cx + dx, cy + b, true);
            }
        }
    }
}

fn eyes(c: &mut Canvas, t: u32) {
    let mood = (t / 3600) % 6; // neutral, happy, sad, angry, surprised, sleepy
    let mood = if mood == 0 { 0 } else { mood };
    let sx = tri(t / 90, 12) - 6;
    let sy = tri(t / 140, 6) - 3;
    let step = t / 1100;
    let gaze = ((hash(step) % 9) as i32 - 4, (hash(step + 99) % 7) as i32 - 3);
    let ph = t % 2600;
    let blink = if mood == 5 {
        50 + tri(t / 40, 40) // sleepy: slow droop
    } else if ph < 160 {
        100 - (ph as i32 - 80).abs() * 100 / 80
    } else {
        0
    };
    let gaze = if mood == 5 { (0, 3) } else { gaze };
    eye(c, 34 + sx, 30 + sy, 1, mood, blink.clamp(0, 100), gaze);
    eye(c, 94 + sx, 30 + sy, -1, mood, blink.clamp(0, 100), gaze);
    let _ = mood;
}

// ------------------------------------------------------------------ the escape artist
fn person(c: &mut Canvas, x: i32, feet: i32, frame: u32, arms_up: bool) {
    // head, body, arms, legs (the legs swap every frame so he walks)
    disc(c, x, feet - 12, 3);
    c.line(x, feet - 9, x, feet - 4);
    if arms_up {
        c.line(x, feet - 8, x - 4, feet - 13);
        c.line(x, feet - 8, x + 4, feet - 13);
    } else {
        let sw = if frame % 2 == 0 { 3 } else { -3 };
        c.line(x, feet - 8, x + sw, feet - 5);
        c.line(x, feet - 8, x - sw, feet - 5);
    }
    let lg = if frame % 2 == 0 { 3 } else { -3 };
    c.line(x, feet - 4, x + lg, feet);
    c.line(x, feet - 4, x - lg, feet);
}

fn walker(c: &mut Canvas, t: u32) {
    let u = t % 18000;
    let feet = 58;
    c.fill_rect(0, 60, 128, 1, true); // floor
    let door_open = (12000..14500).contains(&u);
    // the exit door at the left wall
    c.rect(0, 44, 8, 16);
    if door_open {
        c.fill_rect(1, 45, 6, 14, true);
    } else {
        c.dot(6, 52, true);
    }
    if (12500..16500).contains(&u) && (u / 300) % 2 == 0 {
        c.text(10, 36, "EXIT");
    }
    let frame = t / 130;
    match u {
        0..=3000 => person(c, 12 + (u as i32) * 100 / 3000, feet, frame, false), // walk right
        3000..=4300 => {
            // bumps into the edge, stars
            person(c, 112, feet, 0, true);
            let k = (u - 3000) / 130;
            c.text(116, (36 + (k % 3) as usize * 2) as usize, "*");
            if (u / 200) % 2 == 0 {
                c.text(106, 30, "!");
            }
        }
        4300..=7000 => person(c, 112 - ((u - 4300) as i32) * 52 / 2700, feet, frame, false), // walks back to the middle
        7000..=10500 => {
            // jumps again and again for something up high, never reaching it
            let k = (u - 7000) % 700;
            let jump = if k < 350 { (k as i32) * (350 - k as i32) * 22 / (175 * 175) } else { 0 };
            person(c, 60, feet - jump, 1, jump > 4);
            c.rect(58, 6, 5, 7); // a tiny window up high
            if k >= 350 && (u / 350) % 2 == 0 {
                c.text(68, 40, "?");
            }
        }
        10500..=12000 => person(c, 60 - ((u - 10500) as i32) * 48 / 1500, feet, frame * 2, false), // spots the door
        12000..=14500 => {
            // reaches the door and goes through it
            let p = ((u - 12000) as i32).min(1200);
            person(c, 12 - p * 18 / 1200, feet, frame * 2, false);
        }
        _ => {
            // empty room for a while, then he wanders back in from the right and the story starts again
            if u > 16500 {
                person(c, 140 - ((u - 16500) as i32) * 128 / 1500, feet, frame, false);
            }
        }
    }
}
