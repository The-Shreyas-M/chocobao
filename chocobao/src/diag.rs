// diag.rs - cheap counters that let us see what the firmware is doing from outside (console command `diag`,
// polled by tools/watch_diag.py) and on the Diagnostics screen. Atomics only: no locks, no allocation.
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::OnceLock;
use std::time::Instant;

pub static TICKS: AtomicU32 = AtomicU32::new(0); // app loop iterations (heartbeat)
pub static LOOP_MAX_MS: AtomicU32 = AtomicU32::new(0); // longest app loop iteration (not counting its sleep)
pub static PUSHES: AtomicU32 = AtomicU32::new(0); // OLED pushes that worked
pub static FAILS: AtomicU32 = AtomicU32::new(0); // OLED pushes that failed
pub static PUSH_MAX_MS: AtomicU32 = AtomicU32::new(0); // slowest OLED push
pub static STUCK_MS: AtomicU32 = AtomicU32::new(0); // how long the current OLED transfer has been running
pub static PANICS: AtomicU32 = AtomicU32::new(0); // panics caught by the app loop
pub static WD_TRIPS: AtomicU32 = AtomicU32::new(0); // times the display watchdog fired
pub static SCREEN: AtomicU32 = AtomicU32::new(0); // which screen is showing (see app.rs screen_code)
pub static MODE: AtomicU32 = AtomicU32::new(0); // bit0 = display off, bit1 = sound off (safe-mode switches)
pub static BTN_EVENTS: AtomicU32 = AtomicU32::new(0); // button events received

pub static APP_BUSY_US: AtomicU32 = AtomicU32::new(0); // microseconds the app loop spent working (not sleeping), wraps
pub static DISP_BUSY_MS: AtomicU32 = AtomicU32::new(0); // milliseconds the display thread spent in I2C transfers
pub static SND_BUSY_MS: AtomicU32 = AtomicU32::new(0); // milliseconds the BIO core spent making tones
pub static TRNG_WORDS: AtomicU32 = AtomicU32::new(0); // 32-bit words read from the hardware random number generator
pub static TYPED_CHARS: AtomicU32 = AtomicU32::new(0); // characters typed into the PC over USB
pub static HEAP_USED: AtomicU32 = AtomicU32::new(0); // bytes of heap in use right now
pub static HEAP_PEAK: AtomicU32 = AtomicU32::new(0); // highest heap use so far

/// Wraps the normal allocator and keeps count, so the CPU stats screen can show how much memory the app uses.
pub struct CountingAlloc;

unsafe impl std::alloc::GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = std::alloc::System.alloc(l);
        if !p.is_null() {
            let u = HEAP_USED.fetch_add(l.size() as u32, Relaxed) + l.size() as u32;
            max(&HEAP_PEAK, u);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        HEAP_USED.fetch_sub(l.size() as u32, Relaxed);
        std::alloc::System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, new_size: usize) -> *mut u8 {
        let q = std::alloc::System.realloc(p, l, new_size);
        if !q.is_null() {
            if new_size >= l.size() {
                let u = HEAP_USED.fetch_add((new_size - l.size()) as u32, Relaxed) + (new_size - l.size()) as u32;
                max(&HEAP_PEAK, u);
            } else {
                HEAP_USED.fetch_sub((l.size() - new_size) as u32, Relaxed);
            }
        }
        q
    }
}

static START: OnceLock<Instant> = OnceLock::new();

pub fn init() { START.get_or_init(Instant::now); }

pub fn uptime_ms() -> u32 { START.get_or_init(Instant::now).elapsed().as_millis() as u32 }

pub fn bump(a: &AtomicU32) { a.fetch_add(1, Relaxed); }

pub fn max(a: &AtomicU32, v: u32) {
    if v > a.load(Relaxed) {
        a.store(v, Relaxed);
    }
}

/// One line, easy to parse: `DIAG up=1234 ticks=.. ...`
pub fn line() -> String {
    format!(
        "DIAG up={} ticks={} loopmax={} push={} fail={} pushmax={} stuck={} panic={} wd={} scr={} btn={} mode={}",
        uptime_ms(),
        TICKS.load(Relaxed),
        LOOP_MAX_MS.load(Relaxed),
        PUSHES.load(Relaxed),
        FAILS.load(Relaxed),
        PUSH_MAX_MS.load(Relaxed),
        STUCK_MS.load(Relaxed),
        PANICS.load(Relaxed),
        WD_TRIPS.load(Relaxed),
        SCREEN.load(Relaxed),
        BTN_EVENTS.load(Relaxed),
        MODE.load(Relaxed),
    )
}
