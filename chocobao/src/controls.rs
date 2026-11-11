// controls.rs - reads the six buttons and drives the status LED. It only produces events; the app
// thread (app.rs) decides what they mean.
//
// Wiring: each button between its pin and GND (internal pull-ups, no resistors needed).
// The board is mounted rotated, so (9 Nov): OK=PB2, UP=PC10, DOWN=PC2, LEFT=PC3, RIGHT=PC9, BACK=PB5.
// LED (PB4, resistor to GND): 3 quick blinks at start, solid while any button is held, then a blink code on
// release (count the blinks): OK 1, UP 2, DOWN 3, LEFT 4, RIGHT 5, BACK 6.
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use bao1x_api::{IoGpio, IoSetup, IoxDir, IoxDriveStrength, IoxEnable, IoxFunction, IoxHal, IoxPort, IoxValue};

const POLL_MS: u32 = 4;
const PRESS_SAMPLES: u32 = 2; // 8 ms: a press is reported almost immediately (tactile switches bounce < 5 ms)
const RELEASE_SAMPLES: u32 = 4; // 16 ms: contact bounce on release is ignored
const LED_ON_MS: u32 = 120;
const LED_OFF_MS: u32 = 150;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Btn {
    Ok,
    Up,
    Down,
    Left,
    Right,
    Back,
}

pub enum Input {
    /// a debounced press
    Press(Btn),
    /// a debounced release, with how long it was held in milliseconds
    Release(Btn, u32),
}

// (button, name, port letter, pin, LED blink code)
const BUTTONS: [(Btn, &str, char, u8, u32); 6] = [
    (Btn::Ok, "OK", 'B', 2, 1),
    (Btn::Up, "UP", 'C', 10, 2),
    (Btn::Down, "DOWN", 'C', 2, 3),
    (Btn::Left, "LEFT", 'C', 3, 4),
    (Btn::Right, "RIGHT", 'C', 9, 5),
    (Btn::Back, "BACK", 'B', 5, 6),
];
const LED_PORT: IoxPort = IoxPort::PB;
const LED_PIN: u8 = 4;

fn port(letter: char) -> IoxPort { if letter == 'C' { IoxPort::PC } else { IoxPort::PB } }

pub fn start(tx: Sender<Input>) { std::thread::Builder::new().name("controls".into()).spawn(move || run(tx)).ok(); }

#[derive(Clone, Copy)]
enum Phase {
    Idle,
    On(u32),
    Off(u32),
}

/// Non-blocking LED sequencer: solid while a button is held, blink codes otherwise.
struct Led {
    phase: Phase,
    pending: u32,
    lit: bool,
}
impl Led {
    fn new() -> Self { Led { phase: Phase::Idle, pending: 0, lit: false } }

    fn request(&mut self, blinks: u32) {
        self.pending = blinks;
        self.phase = Phase::Idle;
    }

    fn set(&mut self, iox: &IoxHal, on: bool) {
        if on != self.lit {
            iox.set_gpio_pin_value(LED_PORT, LED_PIN, if on { IoxValue::High } else { IoxValue::Low });
            self.lit = on;
        }
    }

    fn tick(&mut self, iox: &IoxHal, forced_on: bool) {
        if forced_on {
            self.set(iox, true);
            self.phase = Phase::Idle;
            return;
        }
        match self.phase {
            Phase::Idle => {
                if self.pending > 0 {
                    self.set(iox, true);
                    self.phase = Phase::On(LED_ON_MS);
                } else {
                    self.set(iox, false);
                }
            }
            Phase::On(t) => {
                if t <= POLL_MS {
                    self.set(iox, false);
                    self.pending = self.pending.saturating_sub(1);
                    self.phase = Phase::Off(LED_OFF_MS);
                } else {
                    self.phase = Phase::On(t - POLL_MS);
                }
            }
            Phase::Off(t) => {
                if t <= POLL_MS {
                    self.phase = Phase::Idle;
                } else {
                    self.phase = Phase::Off(t - POLL_MS);
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BtnState {
    low: u32,
    high: u32,
    pressed: bool,
    since: Instant,
}

fn run(tx: Sender<Input>) {
    let tt = ticktimer::Ticktimer::new().unwrap();
    tt.sleep_ms(2500).ok(); // let USB and the console finish starting up
    let iox = IoxHal::new();
    for (_, _, letter, pin, _) in BUTTONS.iter() {
        iox.setup_pin(
            port(*letter),
            *pin,
            Some(IoxDir::Input),
            Some(IoxFunction::Gpio),
            Some(IoxEnable::Enable), // schmitt trigger
            Some(IoxEnable::Enable), // internal pull-up: the button goes between the pin and GND
            None,
            None,
        );
    }
    iox.setup_pin(
        LED_PORT,
        LED_PIN,
        Some(IoxDir::Output),
        Some(IoxFunction::Gpio),
        None,
        None,
        None,
        Some(IoxDriveStrength::Drive4mA),
    );
    iox.set_gpio_pin_value(LED_PORT, LED_PIN, IoxValue::Low);
    log::info!("controls ready: OK=PB2 UP=PC10 DOWN=PC2 LEFT=PC3 RIGHT=PC9 BACK=PB5, LED=PB4");

    let mut led = Led::new();
    led.request(3); // "app started"
    let mut st = [BtnState { low: 0, high: 0, pressed: false, since: Instant::now() }; 6];
    let mut next = Instant::now();
    let mut last_active = Instant::now();
    loop {
        // two bank reads per poll instead of six pin reads
        let (pb, pc) = (iox.get_gpio_bank_value(IoxPort::PB), iox.get_gpio_bank_value(IoxPort::PC));
        for (i, (btn, name, letter, pin, code)) in BUTTONS.iter().enumerate() {
            let bank = if *letter == 'C' { pc } else { pb };
            let down = bank & (1u32 << *pin) == 0;
            let b = &mut st[i];
            if down {
                b.low += 1;
                b.high = 0;
                if !b.pressed && b.low >= PRESS_SAMPLES {
                    b.pressed = true;
                    b.since = Instant::now();
                    log::info!("{} pressed", name);
                    crate::diag::bump(&crate::diag::BTN_EVENTS);
                    tx.send(Input::Press(*btn)).ok();
                }
            } else {
                b.high += 1;
                b.low = 0;
                if b.pressed && b.high >= RELEASE_SAMPLES {
                    b.pressed = false;
                    let held = b.since.elapsed().as_millis() as u32;
                    log::info!("{} released after {} ms", name, held);
                    tx.send(Input::Release(*btn, held)).ok();
                    led.request(*code);
                }
            }
        }
        let any_down = st.iter().any(|b| b.pressed);
        led.tick(&iox, any_down);
        // fixed-rate schedule: sleep until the next 5 ms slot instead of "5 ms after we finished"
        // while nothing is pressed (and for a moment after) poll every 8 ms; the CPU has less to do for the games
        if any_down {
            last_active = Instant::now();
        }
        let step = if last_active.elapsed() < Duration::from_millis(1500) { POLL_MS } else { POLL_MS * 2 };
        next += Duration::from_millis(step as u64);
        let now = Instant::now();
        if next > now {
            tt.sleep_ms(((next - now).as_millis() as usize).max(1)).ok();
        } else {
            next = now; // we fell behind; do not try to catch up in a burst
        }
    }
}
