// display.rs - owns the OLED on its own thread so a slow or confused I2C transfer can never freeze the
// buttons, the games or the menu.
//
//  * The app draws into a `Canvas` and calls `submit`; this thread picks up the newest frame (older
//    frames are simply skipped) and sends only what changed.
//  * Every 3 s the controller is fully re-initialised and the whole frame is resent, so any glitch
//    (brown-out, a lost byte, electrical noise) heals itself.
//  * `stuck_ms` reports how long the current I2C transfer has been running; the app uses it as a watchdog.
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::oled::{Canvas, Frame, Panel, H, W};

const REFRESH_MS: u32 = 3000;

struct Shared {
    frame: Mutex<Frame>,
    seq: AtomicU32,
    busy_since: AtomicU32, // 0 = idle, otherwise ms since `t0` (+1)
    present: AtomicBool,
    t0: Instant,
}

#[derive(Clone)]
pub struct Display {
    sh: Arc<Shared>,
}

impl Display {
    /// Start the display thread. Returns None if no display answers.
    pub fn start() -> Option<Display> {
        let sh = Arc::new(Shared {
            frame: Mutex::new([0; W * H / 8]),
            seq: AtomicU32::new(0),
            busy_since: AtomicU32::new(0),
            present: AtomicBool::new(false),
            t0: Instant::now(),
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let sh2 = sh.clone();
        std::thread::Builder::new().name("display".into()).spawn(move || thread_main(sh2, tx)).ok()?;
        match rx.recv_timeout(Duration::from_secs(4)) {
            Ok(true) => Some(Display { sh }),
            _ => None,
        }
    }

    /// Publish a new picture. Never blocks on the display.
    pub fn submit(&self, c: &Canvas) {
        if let Ok(mut f) = self.sh.frame.lock() {
            f.copy_from_slice(&c.fb);
        }
        self.sh.seq.fetch_add(1, Ordering::SeqCst);
    }

    /// How long the current display transfer has been running (0 when idle).
    pub fn stuck_ms(&self) -> u32 {
        let b = self.sh.busy_since.load(Ordering::SeqCst);
        if b == 0 {
            0
        } else {
            now_ms(&self.sh).saturating_sub(b - 1)
        }
    }

    #[allow(dead_code)]
    pub fn present(&self) -> bool { self.sh.present.load(Ordering::SeqCst) }
}

fn now_ms(sh: &Shared) -> u32 { sh.t0.elapsed().as_millis() as u32 }

fn thread_main(sh: Arc<Shared>, ready: std::sync::mpsc::Sender<bool>) {
    let tt = ticktimer::Ticktimer::new().unwrap();
    let mut panel = Panel::new();
    sh.busy_since.store(1, Ordering::SeqCst);
    let ok = panel.init();
    sh.busy_since.store(0, Ordering::SeqCst);
    sh.present.store(ok, Ordering::SeqCst);
    ready.send(ok).ok();
    if !ok {
        return;
    }
    let mut shown = u32::MAX;
    let mut last_full = now_ms(&sh);
    let mut local: Frame = [0; W * H / 8];
    loop {
        let r = catch_unwind(AssertUnwindSafe(|| {
            let seq = sh.seq.load(Ordering::SeqCst);
            let due = now_ms(&sh).saturating_sub(last_full) >= REFRESH_MS;
            if seq != shown || due {
                if let Ok(f) = sh.frame.lock() {
                    local.copy_from_slice(&*f);
                }
                sh.busy_since.store(now_ms(&sh) + 1, Ordering::SeqCst);
                if due {
                    panel.init();
                    panel.invalidate();
                    last_full = now_ms(&sh);
                }
                let ok = panel.push(&local);
                sh.busy_since.store(0, Ordering::SeqCst);
                if ok {
                    shown = seq;
                } else {
                    log::info!("display: transfer failed, will resend");
                    last_full = now_ms(&sh).saturating_sub(REFRESH_MS - 300); // full refresh in 0.3 s
                    tt.sleep_ms(100).ok();
                }
            }
        }));
        if r.is_err() {
            log::error!("display thread panic caught");
            sh.busy_since.store(0, Ordering::SeqCst);
        }
        tt.sleep_ms(15).ok();
    }
}
