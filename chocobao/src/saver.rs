// saver.rs - screensavers. Every one is a pure function of time (no state), uses few lit pixels and keeps moving so
// no pixel stays on (OLED burn-in). `kind` is chosen in Settings > Screensaver.
use crate::oled::Canvas;

pub const NAMES: [&str; 6] = ["Drifting bun", "Auto Pong", "Eyes", "Escape artist", "Bao and bar", "Surprise me"];
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
        3 => walker(c, t),
        4 => bao_bar(c, t),
        _ => draw(c, ((t / 60_000) % 5) as usize, t),
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
    let sh = tri(t / 900, 6) - 3; // the centre line and the score drift sideways (burn-in)
    for y in (0..64).step_by(6) {
        c.dot(64 + sh, y, true);
        c.dot(64 + sh, y + 1, true);
    }
    c.text((50 + sh) as usize, 0, &format!("{}", (t / 7000) % 10));
    c.text((72 + sh) as usize, 0, &format!("{}", (t / 11000) % 10));
}

// ------------------------------------------------------------------ eyes with moods, a mouth that eats chocolate
const R: i32 = 11; // eye radius
const FEED_AT: u32 = 13_000; // ms into every 20 s cycle when the chocolate bar arrives
const CYCLE: u32 = 20_000;
const MOODS: u32 = 13; // neutral, happy, sad, angry, surprised, sleepy, wink, love, dizzy, suspicious, crying, laughing, bored
const HEART: [u8; 6] = [0b0110110, 0b1111111, 0b1111111, 0b0111110, 0b0011100, 0b0001000];

fn disc(c: &mut Canvas, cx: i32, cy: i32, r: i32) {
    for dy in -r..=r {
        let w = Canvas::dome_h(r, r, dy);
        c.fill_rect((cx - w).max(0) as usize, (cy + dy).max(0) as usize, (2 * w + 1) as usize, 1, true);
    }
}

fn eye(c: &mut Canvas, cx: i32, cy: i32, inner: i32, mood: u32, blink: i32, gaze: (i32, i32), t: u32) {
    // outline
    for dx in -R..=R {
        let h = Canvas::dome_h(R, R, dx);
        c.dot(cx + dx, cy - h, true);
        c.dot(cx + dx, cy + h, true);
        c.dot(cx - h, cy + dx, true);
        c.dot(cx + h, cy + dx, true);
    }
    // pupil
    match mood {
        7 => {
            for (r, bits) in HEART.iter().enumerate() {
                for col in 0..7 {
                    if bits & (0b1000000 >> col) != 0 {
                        c.dot(cx + gaze.0 - 3 + col, cy + gaze.1 - 3 + r as i32, true);
                    }
                }
            }
        }
        8 => {
            let a = (t / 12) as i32; // dizzy: the pupil circles quickly
            disc(c, cx + cosn(a) * 5 / 64, cy + sn(a) * 5 / 64, 2);
        }
        4 => disc(c, cx + gaze.0, cy + gaze.1, 2),
        _ => {
            disc(c, cx + gaze.0, cy + gaze.1, 4);
            c.dot(cx + gaze.0 - 2, cy + gaze.1 - 2, false); // shine
        }
    }
    // eyelids: the upper lid (relative to the centre) as a function of the column; everything above it is closed
    for dx in -R..=R {
        let h = Canvas::dome_h(R, R, dx);
        let s = dx * inner; // positive towards the nose
        let mut up = -R - 1;
        match mood {
            2 | 10 => up = -R / 3 - s * 2 / 3,  // sad / crying: the outer corner droops
            12 => up = -R / 8,                  // bored: heavy lids
            3 => up = -R / 3 + s * 2 / 3,  // angry: the inner corner drops
            5 => up = 1,                    // sleepy: half closed
            9 => up = -R / 5,               // suspicious: narrowed
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
        if mood == 1 || mood == 7 || mood == 11 {
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

/// shape 0 flat, 1 smile, 2 frown, 3 small "o", 4 wavy, 5 tilted, 6 open mouth (`open` 0..=100 = how wide)
fn mouth(c: &mut Canvas, cx: i32, cy: i32, shape: u32, open: i32, t: u32) {
    match shape {
        1 => {
            for dx in -14..=14 {
                c.dot(cx + dx, cy - 4 + (196 - dx * dx) / 40, true);
            }
        }
        2 => {
            for dx in -12..=12 {
                c.dot(cx + dx, cy + 4 - (144 - dx * dx) / 36, true);
            }
        }
        3 => {
            for dx in -4..=4i32 {
                let h = Canvas::dome_h(4, 4, dx);
                c.dot(cx + dx, cy - h, true);
                c.dot(cx + dx, cy + h, true);
            }
        }
        4 => {
            for dx in -14..=14 {
                c.dot(cx + dx, cy + sn(dx * 10 + (t / 8) as i32) / 24, true);
            }
        }
        5 => {
            for dx in -12..=12 {
                c.dot(cx + dx, cy + dx / 8, true);
            }
        }
        6 => {
            let ry = 1 + open * 9 / 100;
            for dx in -14..=14i32 {
                let h = Canvas::dome_h(14, ry, dx);
                c.dot(cx + dx, cy - h, true);
                c.dot(cx + dx, cy + h, true);
            }
            for dy in -ry..=ry {
                let w = Canvas::dome_h(14, ry, 0).max(1);
                let _ = w;
                let hw = (14 * 14 - 14 * 14 * dy * dy / (ry * ry).max(1)).max(0);
                let hw = (hw as f32).sqrt() as i32;
                c.dot(cx - hw, cy + dy, true);
                c.dot(cx + hw, cy + dy, true);
            }
            if open > 55 {
                for dx in -5..=5i32 {
                    c.dot(cx + dx, cy + ry - 1 - dx * dx / 12, true); // tongue
                }
            }
        }
        _ => {
            for dx in -10..=10 {
                c.dot(cx + dx, cy, true);
            }
        }
    }
}

/// a small chocolate bar, 10 wide: two columns, three rows, drawn with its top-left corner at (x, y)
fn bar(c: &mut Canvas, x: i32, y: i32, clip_y: i32) {
    for r in 0..3i32 {
        for q in 0..2i32 {
            let (bx, by) = (x + q * 6, y + r * 6);
            for dy in 0..6 {
                if by + dy >= clip_y {
                    continue;
                }
                for dx in 0..6 {
                    let edge = dx == 0 || dx == 5 || dy == 0 || dy == 5;
                    if edge || (dx == 2 && dy == 2) {
                        c.dot(bx + dx, by + dy, true);
                    }
                }
            }
        }
    }
}

fn cosn(a: i32) -> i32 { sn(a + 64) }
fn sn(a: i32) -> i32 {
    let a = a.rem_euclid(256);
    if a < 128 {
        a * (128 - a) / 64
    } else {
        -((a - 128) * (256 - a) / 64)
    }
}

fn eyes(c: &mut Canvas, t: u32) {
    // the whole face drifts around a little so no pixel stays lit (burn-in)
    let ox = tri(t / 380, 8) - 4;
    let oy = tri(t / 520, 4) - 2;
    let u = t % CYCLE;
    let feeding = u >= FEED_AT;
    let step = t / 1100;
    let mut gaze = ((hash(step) % 7) as i32 - 3, (hash(step + 99) % 5) as i32 - 2);
    let mut mood = (u / 1300) % MOODS;
    let ph = t % 2600;
    let mut blink = if ph < 160 { 100 - (ph as i32 - 80).abs() * 100 / 80 } else { 0 };
    let mut mshape = match mood {
        1 | 6 | 7 => 1,
        2 | 3 | 10 => 2,
        11 => 6,
        4 => 3,
        8 => 4,
        9 => 5,
        _ => 0,
    };
    let mut open = if mood == 11 { 45 + tri(t / 70, 30) } else { 0 };
    let mut blink_r = blink;
    if mood == 12 {
        gaze = (3, -3);
    }
    if mood == 5 {
        blink = 50 + tri(t / 40, 40); // sleepy: slow droop
        blink_r = blink;
        gaze = (0, 3);
    }
    if mood == 6 {
        blink_r = 100; // wink with the right eye
    }
    if mood == 9 {
        gaze = (if (t / 700) % 2 == 0 { -4 } else { 4 }, 0);
    }
    let (ex1, ex2, ey, my) = (36 + ox, 92 + ox, 22 + oy, 49 + oy);
    if feeding {
        let f = u - FEED_AT; // 0..7000
        gaze = (0, -2);
        mood = 4;
        blink = 0;
        blink_r = 0;
        mshape = 3;
        let bar_x = 64 + ox - 6;
        if f < 1500 {
            // the bar falls from the top towards the mouth, the mouth opens
            let by = -20 + (f as i32) * (my - 26 + 20) / 1500;
            bar(c, bar_x, by, 200);
            mshape = 6;
            open = (f as i32) * 70 / 1500;
        } else if f < 2300 {
            // it goes in: only the part above the lips is visible
            let p = (f - 1500) as i32;
            bar(c, bar_x, my - 26 + p * 22 / 800, my - 2);
            mshape = 6;
            open = 100;
        } else if f < 5200 {
            // chewing
            mood = 1;
            mshape = 6;
            open = 25 + tri(t / 90, 40) * 100 / 40 / 2;
            if (t / 140) % 2 == 0 {
                c.dot(64 + ox - 16, my + 8 + ((t / 60) % 6) as i32, true); // crumbs
                c.dot(64 + ox + 16, my + 9 + ((t / 70) % 6) as i32, true);
            }
        } else if f < 6300 {
            mood = 1;
            mshape = 1;
            c.text((52 + ox).max(0) as usize, (my + 10).max(0) as usize, "yum!");
        } else {
            mood = 7; // happy hearts, then the cycle starts again
            mshape = 1;
        }
    }
    eye(c, ex1, ey, 1, mood, blink.clamp(0, 100), gaze, t);
    eye(c, ex2, ey, -1, mood, blink_r.clamp(0, 100), gaze, t);
    mouth(c, 64 + ox, my, mshape, open, t);
    if mood == 10 && !feeding {
        for (k, ex) in [ex1, ex2].iter().enumerate() {
            for j in 0..2u32 {
                let fall = ((t / 50 + j * 7 + k as u32 * 3) % 22) as i32;
                c.dot(*ex - 4 + (j as i32) * 8, ey + R + 1 + fall, true);
                c.dot(*ex - 4 + (j as i32) * 8, ey + R + 2 + fall, true);
            }
        }
    }
}

/// A little bao walks up to a chocolate bar, eats it square by square, hops happily and walks on; then a new bar appears.
fn bao_bar(c: &mut Canvas, t: u32) {
    let u = t % 12_000;
    let oy = tri(t / 700, 6) - 3;
    let ox = tri(t / 900, 6) - 3;
    let eaten = if u < 3500 { 0 } else { ((u - 3500) / 600).min(6) as i32 };
    // the bar: 2 columns x 3 rows of squares, eaten from the left side
    for r in 0..3i32 {
        for q in 0..2i32 {
            let idx = r * 2 + q;
            let gone = (idx < eaten) || (u > 8200);
            if gone {
                continue;
            }
            let (bx, by) = (90 + ox + q * 9, 18 + oy + r * 9);
            c.rect(bx as usize, by as usize, 9, 9);
            c.fill_rect((bx + 2) as usize, (by + 2) as usize, 4, 1, true);
        }
    }
    // the bun
    let x = if u < 3500 {
        10 + (u as i32) * 64 / 3500
    } else if u < 8500 {
        74
    } else {
        74 + ((u - 8500) as i32) * 70 / 3500
    };
    let hop = if (7100..8500).contains(&u) { ((u - 7100) / 120 % 2) as i32 * -5 } else { 0 };
    let bob = if u < 3500 || u >= 8500 { ((t / 140) % 2) as i32 } else { 0 };
    let (cx, base) = (x + ox, 50 + oy + hop - bob);
    for dx in -11..=11i32 {
        let h = Canvas::dome_h(11, 12, dx);
        c.fill_rect((cx + dx).max(0) as usize, (base - h).max(0) as usize, 1, h.max(1) as usize, true);
    }
    c.dot(cx - 4, base - 7, false);
    c.dot(cx + 3, base - 7, false);
    if (3500..7100).contains(&u) && (u / 150) % 2 == 0 {
        for dy in 0..4i32 {
            c.fill_rect((cx + 4).max(0) as usize, (base - 5 + dy).max(0) as usize, 8, 1, false); // chomp
        }
    }
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
    let fy = 59 + tri(t / 20000, 2); // the floor level changes slowly (burn-in)
    let feet = fy - 2;
    c.fill_rect(0, fy as usize, 128, 1, true); // floor
    let door_open = (12000..14500).contains(&u);
    // the exit door at the left wall
    c.rect(0, (fy - 16) as usize, 8, 16);
    if door_open {
        c.fill_rect(1, (fy - 15) as usize, 6, 14, true);
    } else {
        c.dot(6, fy - 8, true);
    }
    if (12500..16500).contains(&u) && (u / 300) % 2 == 0 {
        c.text(10, (fy - 24) as usize, "EXIT");
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
