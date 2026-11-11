// player.rs - the music player screen: a record that turns slowly. Kept deliberately cheap to draw (a few lines and
// two circles) so the music and the rest of the system never stutter.
use crate::oled::Canvas;

const CX: i32 = 32;
const CY: i32 = 43;
const R: i32 = 19;

pub struct View<'a> {
    pub title: &'a str,
    pub by: &'a str,
    pub playing: bool,
    pub ended: bool,
    pub spin: f32, // record angle in radians (advances while playing)
    pub elapsed_s: u32,
    pub total_s: u32,
}

fn circle(c: &mut Canvas, r: i32) {
    for dy in -r..=r {
        let w = Canvas::dome_h(r, r, dy);
        c.dot(CX - w, CY + dy, true);
        c.dot(CX + w, CY + dy, true);
    }
    for dx in -r..=r {
        let h = Canvas::dome_h(r, r, dx);
        c.dot(CX + dx, CY - h, true);
        c.dot(CX + dx, CY + h, true);
    }
}

pub fn draw(c: &mut Canvas, v: &View) {
    c.text(0, 0, v.title);
    c.text(0, 11, v.by);
    circle(c, R);
    circle(c, R - 5);
    circle(c, R - 10);
    // two light spokes that turn with the record, and the label hub
    for k in 0..2 {
        let a = v.spin + k as f32 * std::f32::consts::PI;
        let (ca, sa) = (a.cos(), a.sin());
        c.line(CX + (4.0 * ca) as i32, CY + (4.0 * sa) as i32, CX + ((R - 2) as f32 * ca) as i32, CY + ((R - 2) as f32 * sa) as i32);
    }
    c.fill_rect((CX - 3) as usize, (CY - 3) as usize, 7, 7, false);
    c.rect((CX - 3) as usize, (CY - 3) as usize, 7, 7);
    c.dot(CX, CY, true);
    // right side: progress and keys
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
