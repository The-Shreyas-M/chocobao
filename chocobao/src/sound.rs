// sound.rs - beeps for a buzzer wired to PC0 (board pin 14): buzzer + to the pin, buzzer - to GND.
//
// Works with BOTH kinds of buzzer:
//   * active (has its own oscillator, sounds on plain DC)  -> hears chopped beeps
//   * passive (needs an AC signal)                         -> hears a ~500 Hz / ~250 Hz buzz
// The pin is toggled once per millisecond from a background thread that sleeps between edges, so the load
// on the system is tiny (an earlier version spun at audio rate and starved the rest of the chip).
// True pitched music needs a BIO core or PWM; that is a later step.
//
// `Sound::play` never blocks: it queues steps for the thread.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use bao1x_api::{IoSetup, IoxDir, IoxDriveStrength, IoxFunction, IoxHal, IoxPort, IoxValue};

const PORT: IoxPort = IoxPort::PC;
const PIN: u8 = 0;

/// (sound for `on` ms, then silent for `off` ms, half period in ms: 0 = steady DC, 1 = ~500 Hz, 2 = ~250 Hz)
pub type Beep = (u32, u32, u8);

pub const CLICK: &[Beep] = &[(8, 0, 1)];
pub const COIN: &[Beep] = &[(40, 40, 1), (70, 0, 1)];
pub const CRASH: &[Beep] = &[(300, 0, 2)];
pub const TICK: &[Beep] = &[(20, 0, 1)];
pub const HELLO: &[Beep] = &[(40, 60, 1), (40, 60, 1), (120, 0, 1)];
pub const SOS: &[Beep] = &[
    (100, 100, 1), (100, 100, 1), (100, 300, 1), (300, 100, 1), (300, 100, 1), (300, 300, 1), (100, 100, 1),
    (100, 100, 1), (100, 0, 1),
];
/// 1: steady DC (an active buzzer sounds here), 2: fast buzz (~500 Hz), 3: slow buzz (~250 Hz)
pub const TEST: &[Beep] = &[(500, 600, 0), (500, 600, 1), (500, 0, 2)];

#[derive(Clone)]
pub struct Sound {
    tx: Sender<Vec<Beep>>,
    stop: Arc<AtomicBool>,
    pub enabled: Arc<AtomicBool>,
}

impl Sound {
    pub fn start() -> Sound {
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let enabled = Arc::new(AtomicBool::new(true));
        let s2 = stop.clone();
        std::thread::Builder::new().name("sound".into()).spawn(move || run(rx, s2)).ok();
        Sound { tx, stop, enabled }
    }

    pub fn play(&self, beeps: &[Beep]) {
        if self.enabled.load(Ordering::Relaxed) {
            self.stop.store(false, Ordering::SeqCst);
            self.tx.send(beeps.to_vec()).ok();
        }
    }

    /// Cut off whatever is playing.
    pub fn silence(&self) { self.stop.store(true, Ordering::SeqCst); }

    pub fn toggle(&self) -> bool {
        let on = !self.enabled.load(Ordering::Relaxed);
        self.enabled.store(on, Ordering::Relaxed);
        if !on {
            self.silence();
        }
        on
    }
}

fn run(rx: Receiver<Vec<Beep>>, stop: Arc<AtomicBool>) {
    let tt = ticktimer::Ticktimer::new().unwrap();
    tt.sleep_ms(3000).ok(); // after the rest of the system has started
    let iox = IoxHal::new();
    iox.setup_pin(
        PORT,
        PIN,
        Some(IoxDir::Output),
        Some(IoxFunction::Gpio),
        None,
        None,
        None,
        Some(IoxDriveStrength::Drive4mA),
    );
    let set = |hi: bool| iox.set_gpio_pin_value(PORT, PIN, if hi { IoxValue::High } else { IoxValue::Low });
    set(false);
    while let Ok(beeps) = rx.recv() {
        for (on, off, half) in beeps {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            if half == 0 {
                set(true);
                tt.sleep_ms(on as usize).ok();
            } else {
                let t0 = tt.elapsed_ms();
                let mut level = false;
                while tt.elapsed_ms() - t0 < on as u64 && !stop.load(Ordering::Relaxed) {
                    level = !level;
                    set(level);
                    tt.sleep_ms(half as usize).ok();
                }
            }
            set(false);
            if off > 0 {
                tt.sleep_ms(off as usize).ok();
            }
        }
        set(false);
    }
}
