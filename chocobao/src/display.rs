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

const REFRESH_MS: u32 = 2000; // full resend (no display-off) to heal any glitch
const MIN_GAP_MS: u32 = 40; // at most 25 pushes per second
const REINIT_AFTER_FAILS: u32 = 3;

struct Shared {
    frame: Mutex<Frame>,
    seq: AtomicU32,
    busy_since: AtomicU32, // 0 = idle, otherwise ms since `t0` (+1)
    present: AtomicBool,
    pushes: AtomicU32,
    fails: AtomicU32,
    max_ms: AtomicU32,
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
            pushes: AtomicU32::new(0),
            fails: AtomicU32::new(0),
            max_ms: AtomicU32::new(0),
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

    /// (successful pushes, failed pushes, slowest push in ms)
    pub fn stats(&self) -> (u32, u32, u32) {
        (
            self.sh.pushes.load(Ordering::Relaxed),
            self.sh.fails.load(Ordering::Relaxed),
            self.sh.max_ms.load(Ordering::Relaxed),
        )
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
    let mut last_push = 0u32;
    let mut consecutive_fails = 0u32;
    let mut local: Frame = [0; W * H / 8];
    loop {
        let r = catch_unwind(AssertUnwindSafe(|| {
            let now = now_ms(&sh);
            let seq = sh.seq.load(Ordering::SeqCst);
            let due = now.saturating_sub(last_full) >= REFRESH_MS;
            if (seq != shown || due) && now.saturating_sub(last_push) >= MIN_GAP_MS {
                if let Ok(f) = sh.frame.lock() {
                    local.copy_from_slice(&*f);
                }
                sh.busy_since.store(now + 1, Ordering::SeqCst);
                if due {
                    panel.invalidate(); // resend everything, but never switch the panel off
                    last_full = now;
                }
                let ok = panel.push(&local);
                let took = now_ms(&sh).saturating_sub(now);
                sh.busy_since.store(0, Ordering::SeqCst);
                crate::diag::STUCK_MS.store(0, Ordering::Relaxed);
                crate::diag::max(&crate::diag::PUSH_MAX_MS, took);
                last_push = now_ms(&sh);
                if took > sh.max_ms.load(Ordering::Relaxed) {
                    sh.max_ms.store(took, Ordering::Relaxed);
                }
                if ok {
                    sh.pushes.fetch_add(1, Ordering::Relaxed);
                    crate::diag::bump(&crate::diag::PUSHES);
                    consecutive_fails = 0;
                    shown = seq;
                } else {
                    sh.fails.fetch_add(1, Ordering::Relaxed);
                    crate::diag::bump(&crate::diag::FAILS);
                    consecutive_fails += 1;
                    if consecutive_fails >= REINIT_AFTER_FAILS {
                        sh.busy_since.store(now_ms(&sh) + 1, Ordering::SeqCst);
                        panel.init(); // the controller may have lost its settings
                        sh.busy_since.store(0, Ordering::SeqCst);
                        consecutive_fails = 0;
                    }
                    last_full = now_ms(&sh).saturating_sub(REFRESH_MS - 300); // full refresh in 0.3 s
                }
            }
        }));
        if r.is_err() {
            log::error!("display thread panic caught");
            sh.busy_since.store(0, Ordering::SeqCst);
        }
        tt.sleep_ms(10).ok();
    }
}
