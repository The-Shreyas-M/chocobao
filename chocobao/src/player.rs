// player.rs - the music player screen: a record on a turntable (drawn as a tilted ellipse, so it looks 3D) with
// amplitude bars standing around its rim, like the visualisers on NCS videos. The bars react to the note that is
// really playing: the bar nearest to the note's pitch jumps up when the note starts and falls back while it rings.
use crate::oled::Canvas;

const N_BARS: usize = 36;
const CX: i32 = 34;
const CY: i32 = 46;
const RX: i32 = 25;
const RY: i32 = 10;

pub struct View<'a> {
    pub title: &'a str,
    pub by: &'a str,
    pub playing: bool,
    pub ended: bool,
    pub spin: f32,     // record angle in radians (advances while playing)
    pub hz: u32,       // pitch of the note now playing (0 = rest)
    pub since_ms: u32, // milliseconds since that note started
    pub t_ms: u32,     // free running time, for the little random wobble
    pub elapsed_s: u32,
    pub total_s: u32,
}

fn hash(n: u32) -> u32 {
    let mut x = n.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

fn ellipse(c: &mut Canvas, cx: i32, cy: i32, rx: i32, ry: i32, dash: Option<i32>) {
    for k in 0..72i32 {
        if let Some(ph) = dash {
            if (k + ph).rem_euclid(4) == 0 {
                continue; // gaps that move: the grooves seem to turn
            }
        }
        let a = k as f32 * (std::f32::consts::TAU / 72.0);
        c.dot(cx + (rx as f32 * a.cos()) as i32, cy + (ry as f32 * a.sin()) as i32, true);
    }
}

pub fn draw(c: &mut Canvas, v: &View) {
    c.text(0, 0, v.title);
    c.text(0, 11, v.by);
    // ---- amplitude bars around the record (36 of them, each leaning outwards)
    let bucket = if v.hz == 0 { None } else { Some(((v.hz.clamp(300, 2600) - 300) * N_BARS as u32 / 2300) as i32) };
    let decay = if v.playing { 1.0 - (v.since_ms as f32 / 260.0).min(1.0) } else { 0.0 };
    for i in 0..N_BARS {
        let a = i as f32 * (std::f32::consts::TAU / N_BARS as f32);
        let (ca, sa) = (a.cos(), a.sin());
        let noise = (hash(i as u32 + (v.t_ms / 70) * 131) >> 29) as f32 / 7.0; // 0..1
        let mut h = 1.5;
        if v.playing {
            let peak = match bucket {
                Some(b) => {
                    let d = ((i as i32 - b).rem_euclid(N_BARS as i32)).min((b - i as i32).rem_euclid(N_BARS as i32)) as f32;
                    10.0 * (-(d * d) / 14.0).exp() * (0.35 + 0.65 * decay)
                }
                None => 0.0,
            };
            h += 1.5 + peak + noise * (1.5 + 3.5 * decay);
        }
        // start on the rim, go outwards; upward bars are drawn full height, those towards the viewer shorter (depth)
        let depth = if sa > 0.0 { 1.0 - 0.45 * sa } else { 1.0 };
        let (x0, y0) = (CX as f32 + (RX + 2) as f32 * ca, CY as f32 + (RY + 1) as f32 * sa);
        let (x1, y1) = (x0 + h * depth * ca * 0.9, y0 + h * depth * sa * 1.0);
        c.line(x0 as i32, y0 as i32, x1 as i32, y1 as i32);
        if sa < 0.0 {
            c.line(x0 as i32 + 1, y0 as i32, x1 as i32 + 1, y1 as i32); // far side: a thicker bar, it looks lit from the front
        }
    }
    // ---- the record: thickness, rim, grooves, label
    for dy in 1..=3 {
        for k in 36..=72i32 {
            // lower half of the ellipse repeated a little lower = the edge of the disc
            let a = k as f32 * (std::f32::consts::TAU / 72.0);
            if dy == 3 {
                c.dot(CX + (RX as f32 * a.cos()) as i32, CY + dy + (RY as f32 * a.sin()) as i32, true);
            }
        }
    }
    c.line(CX - RX, CY, CX - RX, CY + 3);
    c.line(CX + RX, CY, CX + RX, CY + 3);
    ellipse(c, CX, CY, RX, RY, None);
    let ph = (v.spin * 4.0) as i32;
    ellipse(c, CX, CY, RX * 3 / 4, RY * 3 / 4, Some(ph));
    ellipse(c, CX, CY, RX / 2 + 1, RY / 2 + 1, Some(-ph));
    // label with a marker that orbits, so you see the record turn
    for dx in -(RX / 4)..=(RX / 4) {
        let hh = Canvas::dome_h(RX / 4, RY / 4 + 1, dx);
        c.line_set(CX + dx, CY - hh, CX + dx, CY + hh, false);
    }
    ellipse(c, CX, CY, RX / 4, RY / 4 + 1, None);
    let (mx, my) = (CX + ((RX / 5) as f32 * v.spin.cos()) as i32, CY + ((RY / 5) as f32 * v.spin.sin()) as i32);
    c.dot(mx, my, true);
    c.dot(mx + 1, my, true);
    c.dot(CX, CY, true);
    // a glare that sweeps across the record
    let g = (v.spin * 1.3).sin();
    let gx = CX + (g * (RX as f32 * 0.55)) as i32;
    c.line(gx - 2, CY - RY + 2, gx + 2, CY - RY / 2 + 1);
    // ---- right side: progress and keys
    let (px, pw) = (72usize, 54usize);
    let frac = if v.total_s > 0 { (v.elapsed_s * pw as u32 / v.total_s).min(pw as u32) as usize } else { 0 };
    c.rect(px, 28, pw, 5);
    c.fill_rect(px, 28, frac.max(1), 5, true);
    c.text(px, 35, &format!("{}:{:02}/{}:{:02}", v.elapsed_s / 60, v.elapsed_s % 60, v.total_s / 60, v.total_s % 60));
    if v.ended {
        c.text(px, 46, "ended");
        c.text(px, 54, "OK again");
    } else {
        c.text(px, 46, if v.playing { "OK stop" } else { "OK play" });
        c.text(px, 54, "<> song");
    }
}
