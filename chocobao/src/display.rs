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

const REFRESH_MS: u32 = 8_000; // every 3 s: send the OLED settings again and resend the whole picture (heals a power glitch)
const MIN_GAP_MS: u32 = 20; // at most about 40 pushes per second
const REINIT_AFTER_FAILS: u32 = 3;

struct Shared {
    frame: Mutex<Frame>,
    seq: AtomicU32,
    taken: AtomicU32, // the `seq` of the picture the display thread last copied
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
            taken: AtomicU32::new(0),
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
            if f[..] == c.fb[..] {
                return; // nothing changed: no new picture, no I2C traffic
            }
            f.copy_from_slice(&c.fb);
        }
        self.sh.seq.fetch_add(1, Ordering::SeqCst);
    }

    /// True when the display thread has picked up the last picture, so drawing the next one is useful.
    pub fn ready(&self) -> bool { self.sh.taken.load(Ordering::SeqCst) == self.sh.seq.load(Ordering::SeqCst) }

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
    let mut ok = panel.init();
    sh.busy_since.store(0, Ordering::SeqCst);
    sh.present.store(ok, Ordering::SeqCst);
    ready.send(true).ok(); // even without an answer we keep trying: the OLED may be plugged in later
    while !ok {
        tt.sleep_ms(1000).ok();
        sh.busy_since.store(now_ms(&sh) + 1, Ordering::SeqCst);
        ok = panel.init();
        sh.busy_since.store(0, Ordering::SeqCst);
    }
    sh.present.store(true, Ordering::SeqCst);
    let mut shown = u32::MAX;
    let mut last_full = now_ms(&sh);
    let mut last_push = 0u32;
    let mut gap_ms = MIN_GAP_MS; // grows with the time a push takes, so the I2C bus never hogs the CPU
    let mut consecutive_fails = 0u32;
    let mut local: Frame = [0; W * H / 8];
    loop {
        let r = catch_unwind(AssertUnwindSafe(|| {
            let now = now_ms(&sh);
            let seq = sh.seq.load(Ordering::SeqCst);
            let due = now.saturating_sub(last_full) >= REFRESH_MS;
            if (seq != shown || due) && now.saturating_sub(last_push) >= gap_ms {
                if let Ok(f) = sh.frame.lock() {
                    local.copy_from_slice(&*f);
                }
                sh.taken.store(seq, Ordering::SeqCst);
                sh.busy_since.store(now + 1, Ordering::SeqCst);
                let mut ok = true;
                if due {
                    ok = panel.refresh_settings(); // settings again (no flicker) + resend everything
                    last_full = now;
                }
                ok = ok && panel.push(&local);
                let took = now_ms(&sh).saturating_sub(now);
                sh.busy_since.store(0, Ordering::SeqCst);
                crate::diag::STUCK_MS.store(0, Ordering::Relaxed);
                crate::diag::max(&crate::diag::PUSH_MAX_MS, took);
                crate::diag::DISP_BUSY_MS.fetch_add(took, Ordering::Relaxed);
                last_push = now_ms(&sh);
                gap_ms = MIN_GAP_MS.max(took * 6 / 5); // at most about 45% of the time on the bus
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
                    last_full = now_ms(&sh).saturating_sub(REFRESH_MS - 300); // settings + full picture again in 0.3 s
                }
            }
        }));
        if r.is_err() {
            log::error!("display thread panic caught");
            sh.busy_since.store(0, Ordering::SeqCst);
        }
        tt.sleep_ms(4).ok();
    }
}
