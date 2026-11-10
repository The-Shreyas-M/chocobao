// app.rs - the ChocoBao "OS": a small menu system on the 128x64 OLED, driven by six buttons.
//
//   HOME       the logo (chocolate pouring over a bao). BACK opens the menu. Nothing else is reachable here.
//   MENU       UP/DOWN move, OK opens, BACK goes back. Each entry has an icon and a one-line description.
//     Utilities  Password maker, Passphrase, PIN maker, Dice and coin, Random number, Stopwatch, Timer, Entropy meter
//     Games      Mempool Dodger, Snake, Pong, Flappy Bao, Simon, Reaction, ChocoDoom, Magic 8-ball
//     Lab        Button test, Buzzer test, SOS beeps, Sound on/off, Diagnostics
//     About
//   Every app shows a short how-to screen the first time it is opened (OK continues, BACK cancels).
//
// Design rules (so it behaves like a small RTOS and never wedges):
//  * One fixed 16 ms tick. Nothing in the loop waits for hardware except that tick.
//  * The display lives on its own thread (display.rs): a stuck I2C transfer cannot freeze the buttons or
//    the game.
//  * Each step runs inside catch_unwind: a bug on one screen drops you back to HOME instead of killing the app.
//  * Press = instant event (controls.rs); holding LEFT/RIGHT auto-repeats where it helps.
//  * A button release only counts if you pressed it on the same screen (no accidental carry-over).
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use bao1x_hal_service::trng::Trng;
use usb_bao1x::UsbHid;

use crate::controls::{Btn, Input};
use crate::display::Display;
use crate::games::{Game, Snd, GAMES};
use crate::logo;
use crate::oled::{Canvas, Frame};
use crate::rng;
use crate::sound::{self, Sound};

const TICK_MS: u64 = 16;
const WATCHDOG_MS: u32 = 20_000; // only a truly dead display bus should restart the app
const WIPE_MS: u128 = 180;
const SAVER_S: u64 = 45; // idle seconds before the screensaver starts
const SAVER_BLANK_MS: u32 = 150_000; // after this long in the screensaver the screen goes fully dark

// ------------------------------------------------------------------ icons (8x8, MSB = left pixel)
const I_TOOLBOX: [u8; 8] = [0x3C, 0x24, 0xFF, 0x81, 0xBD, 0x81, 0xFF, 0x00];
const I_GAMEPAD: [u8; 8] = [0x00, 0x7E, 0xFF, 0xDB, 0xFF, 0xFF, 0x7E, 0x00];
const I_FLASK: [u8; 8] = [0x38, 0x10, 0x10, 0x28, 0x44, 0x7C, 0x7C, 0x00];
const I_INFO: [u8; 8] = [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00];
const I_KEY: [u8; 8] = [0x38, 0x44, 0x44, 0x38, 0x10, 0x18, 0x10, 0x18];
const I_WORDS: [u8; 8] = [0xEE, 0x00, 0xFE, 0x00, 0xDE, 0x00, 0xF0, 0x00];
const I_PIN: [u8; 8] = [0x54, 0x00, 0x54, 0x00, 0x54, 0x00, 0x28, 0x00];
const I_DICE: [u8; 8] = [0x7E, 0x81, 0xA5, 0x81, 0x81, 0xA5, 0x81, 0x7E];
const I_RANDOM: [u8; 8] = [0x3C, 0x42, 0x04, 0x08, 0x10, 0x00, 0x10, 0x00];
const I_STOPWATCH: [u8; 8] = [0x3C, 0x42, 0x89, 0x91, 0x99, 0x81, 0x42, 0x3C];
const I_HOURGLASS: [u8; 8] = [0xFF, 0x7E, 0x3C, 0x18, 0x18, 0x24, 0x5A, 0xFF];
const I_BARS: [u8; 8] = [0x01, 0x05, 0x45, 0x55, 0x55, 0xD7, 0xD7, 0xFF];
const I_DODGER: [u8; 8] = [0x3C, 0x3C, 0x00, 0x00, 0x00, 0x10, 0x38, 0x7C];
const I_SNAKE: [u8; 8] = [0x7C, 0x04, 0x04, 0x3C, 0x20, 0x20, 0x3E, 0x00];
const I_PONG: [u8; 8] = [0x00, 0x80, 0x80, 0x81, 0x89, 0x01, 0x01, 0x00];
const I_BIRD: [u8; 8] = [0x3C, 0x7E, 0xF6, 0xFF, 0xFE, 0x7C, 0x38, 0x00];
const I_SIMON: [u8; 8] = [0xEE, 0xEE, 0xEE, 0x00, 0xEE, 0xEE, 0xEE, 0x00];
const I_BOLT: [u8; 8] = [0x1C, 0x38, 0x70, 0xFE, 0x1C, 0x38, 0x70, 0xE0];
const I_CROSS: [u8; 8] = [0x10, 0x10, 0x00, 0xE7, 0xE7, 0x00, 0x10, 0x10];
const I_BALL: [u8; 8] = [0x3C, 0x7E, 0xE7, 0xDB, 0xE7, 0xDB, 0x66, 0x3C];
const I_BUTTON: [u8; 8] = [0x3C, 0x42, 0x99, 0x99, 0x81, 0x42, 0x3C, 0x00];
const I_BUN: [u8; 8] = [0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0xFF, 0x7E, 0xFF];
const I_PAC: [u8; 8] = [0x3C, 0x7E, 0xF8, 0xF0, 0xF8, 0x7E, 0x3C, 0x00];
const I_GEAR: [u8; 8] = [0x10, 0x92, 0x7C, 0xC6, 0xC6, 0x7C, 0x92, 0x10];
const I_SPEAKER: [u8; 8] = [0x08, 0x18, 0xF8, 0xF8, 0xF8, 0x18, 0x08, 0x00];

#[derive(Clone, Copy, PartialEq)]
enum MenuId {
    Main,
    Utilities,
    Games,
    Music,
    Lab,
    Settings,
}

struct Item {
    label: &'static str,
    icon: &'static [u8; 8],
    desc: &'static str,
}

const fn it(label: &'static str, icon: &'static [u8; 8], desc: &'static str) -> Item { Item { label, icon, desc } }

static MAIN_ITEMS: [Item; 5] = [
    it("Tools", &I_TOOLBOX, "Passwords, clock, timers"),
    it("Games", &I_GAMEPAD, "9 games incl. Pac-Man"),
    it("Music", &I_SPEAKER, "Tunes and a piano"),
    it("Lab", &I_FLASK, "Experiments"),
    it("Settings", &I_GEAR, "Saver, sound, tests, about"),
];
static UTIL_ITEMS: [Item; 10] = [
    it("Password maker", &I_KEY, "Rules for any website"),
    it("Passphrase", &I_WORDS, "Random words, 11 bits each"),
    it("PIN maker", &I_PIN, "Random digits"),
    it("Dice and coin", &I_DICE, "Roll any dice or flip"),
    it("Random number", &I_RANDOM, "Pick a number 1 to N"),
    it("Stopwatch", &I_STOPWATCH, "Start, stop, reset"),
    it("Timer", &I_HOURGLASS, "Countdown with alarm"),
    it("Entropy meter", &I_BARS, "Live check of the TRNG"),
    it("Seed words", &I_WORDS, "Valid BIP-39, 12-24 words"),
    it("Clock", &I_STOPWATCH, "A watch you set by hand"),
];
static GAME_ITEMS: [Item; 9] = [
    it("Mempool Dodger", &I_DODGER, "Dodge blocks, grab coins"),
    it("Snake", &I_SNAKE, "Eat, grow, don't crash"),
    it("Pong", &I_PONG, "You vs the computer"),
    it("Flappy Bao", &I_BIRD, "Fly through the gaps"),
    it("Simon", &I_SIMON, "Repeat the arrow pattern"),
    it("Reaction", &I_BOLT, "How fast are you?"),
    it("ChocoDoom", &I_CROSS, "3D shooter, 5 demons"),
    it("Pac-Man", &I_PAC, "Eat the dots, dodge ghosts"),
    it("Magic 8-ball", &I_BALL, "Ask a yes/no question"),
];
static MUSIC_ITEMS: [Item; 3] = [
    it("Blinding Lights", &I_SPEAKER, "The Weeknd (by ear)"),
    it("Attention", &I_SPEAKER, "Charlie Puth (by ear)"),
    it("Piano", &I_SPEAKER, "Play notes with buttons"),
];
static LAB_ITEMS: [Item; 2] = [
    it("Mandelbrot", &I_BUN, "Endless fractal zoom"),
    it("SOS beeps", &I_SPEAKER, "Morse SOS on the buzzer"),
];
static SET_ITEMS: [Item; 6] = [
    it("Screensaver", &I_BUN, "Pick what shows when idle"),
    it("Sound on/off", &I_SPEAKER, "Mute or unmute"),
    it("Buzzer test", &I_SPEAKER, "Plays a C major scale"),
    it("Button test", &I_BUTTON, "Check all 6 buttons"),
    it("Diagnostics", &I_BARS, "Counters for debugging"),
    it("About", &I_INFO, "Version and chip facts"),
];

fn items(m: MenuId) -> &'static [Item] {
    match m {
        MenuId::Main => &MAIN_ITEMS,
        MenuId::Utilities => &UTIL_ITEMS,
        MenuId::Games => &GAME_ITEMS,
        MenuId::Music => &MUSIC_ITEMS,
        MenuId::Lab => &LAB_ITEMS,
        MenuId::Settings => &SET_ITEMS,
    }
}

fn menu_title(m: MenuId) -> &'static str {
    match m {
        MenuId::Main => "ChocoBao",
        MenuId::Utilities => "Tools",
        MenuId::Games => "Games",
        MenuId::Music => "Music",
        MenuId::Lab => "Lab",
        MenuId::Settings => "Settings",
    }
}

// ------------------------------------------------------------------ first-open help
const H_PW: usize = 0;
const H_WORDS: usize = 1;
const H_PIN: usize = 2;
const H_DICE: usize = 3;
const H_RAND: usize = 4;
const H_STOP: usize = 5;
const H_TIMER: usize = 6;
const H_ENT: usize = 7;
const H_GAME: usize = 8; // + game index (0..=7, 8 = Magic 8-ball)
const H_SEED: usize = 17;
const H_MANDEL: usize = 18;
const H_CLOCK: usize = 19;
const H_PIANO: usize = 20;
const H_PLAYER: usize = 21;
const HELP_COUNT: usize = 22;

fn help_text(id: usize) -> (&'static str, [&'static str; 4]) {
    match id {
        H_PW => ("Password maker", ["^v  pick the type", "<>  change the length", "HOLD OK: type it in", "OK tap = new secret"]),
        H_WORDS => ("Passphrase", ["<>  number of words", "^v  other types", "HOLD OK: type it in", "OK tap = new secret"]),
        H_PIN => ("PIN maker", ["<>  number of digits", "^v  other types", "HOLD OK: type it in", "OK tap = new secret"]),
        H_DICE => ("Dice and coin", ["^v  pick the dice", "OK  roll", "", ""]),
        H_RAND => ("Random number", ["^v  pick the range", "OK  draw a number", "", ""]),
        H_STOP => ("Stopwatch", ["OK  start / stop", "UP  reset to zero", "", ""]),
        H_TIMER => ("Timer", ["<>  +/- 10 seconds", "^v  +/- 1 minute", "OK  start / pause", "alarm when it ends"]),
        H_ENT => ("Entropy meter", ["Counts the 1 bits of", "the hardware random", "generator. Healthy = 50%", "bars about equal"]),
        H_CLOCK => ("Clock", ["no battery clock, so set", "it once after power-up", "OK = set / done", "^v hours   <> minutes"]),
        H_PIANO => ("Piano", ["LEFT UP RIGHT DOWN OK", "play C E G A C", "one note per press", "BACK exits"]),
        H_PLAYER => ("Music player", ["OK = play / stop", "plays on the buzzer", "tunes were written by", "ear, so notes may differ"]),
        H_MANDEL => ("Mandelbrot", ["zooms by itself, on-chip", "OK pause   ^v zoom", "<> other place", "integer maths, no float"]),
        H_SEED => ("Seed words", ["<>  12/15/18/21/24", "^v  next / prev page", "OK  new (checksum ok)", "never typed or saved"]),
        x if x == H_GAME => ("Mempool Dodger", ["<> move, hold = glide", "dodge the blocks", "hollow squares = coins", "OK pauses"]),
        x if x == H_GAME + 1 => ("Snake", ["^v<> steer", "eat the hollow squares", "walls and tail hurt", "OK pauses"]),
        x if x == H_GAME + 2 => ("Pong", ["^v move your paddle", "return the ball", "OK pauses", ""]),
        x if x == H_GAME + 3 => ("Flappy Bao", ["OK or UP flaps", "fly through the gaps", "don't touch anything", ""]),
        x if x == H_GAME + 4 => ("Simon", ["watch the arrows", "then repeat them with", "^ v < >", "it grows each round"]),
        x if x == H_GAME + 5 => ("Reaction", ["wait for GO!", "then press OK fast", "pressing early loses", ""]),
        x if x == H_GAME + 6 => ("ChocoDoom", ["^v  walk fwd / back", "<>  turn (hold)", "OK  shoot", "kill all 5 demons"]),
        x if x == H_GAME + 7 => ("Pac-Man", ["^v<> steer (it remembers)", "eat every dot", "o = power: eat ghosts", "OK pauses"]),
        _ => ("Magic 8-ball", ["think of a question", "press OK", "", ""]),
    }
}

// ------------------------------------------------------------------ other constants
const DICE: [(u32, u32, &str); 10] = [
    (1, 4, "1d4"),
    (1, 6, "1d6"),
    (2, 6, "2d6"),
    (1, 8, "1d8"),
    (1, 10, "1d10"),
    (1, 12, "1d12"),
    (1, 20, "1d20"),
    (1, 100, "1d100"),
    (3, 6, "3d6"),
    (1, 2, "coin"),
];
const RANGES: [u32; 7] = [6, 10, 20, 100, 1000, 10_000, 65_535];

const BALL: [&str; 14] = [
    "It is certain",
    "Without a doubt",
    "Yes, definitely",
    "Most likely",
    "Outlook good",
    "Signs point to yes",
    "Reply hazy, retry",
    "Ask again later",
    "Cannot predict now",
    "Concentrate, ask",
    "Don't count on it",
    "My reply is no",
    "Outlook not so good",
    "Very doubtful",
];

struct Entropy {
    total: u32,
    ones: u32,
    bins: [u32; 16],
}

/// Where a help screen leads after OK.
enum Then {
    Game(usize),
    Open(Box<Screen>),
}

enum Screen {
    Boot(Instant),
    Home,
    Menu(MenuId, usize),
    Help { id: usize, then: Then },
    PwGen { prof: usize, len: usize, pw: String, from: usize },
    Dice { p: usize, cur: String },
    Rand { r: usize, result: Option<u32> },
    Stopwatch { running: bool, started: Instant, acc: Duration },
    Timer { set_s: u32, running: bool, end: Instant, left_ms: u32, done: bool },
    Entropy(Entropy),
    Seed { n: usize, idx: Vec<u16>, page: usize },
    Mandel(Box<crate::mandel::Mandel>),
    Clock { edit: bool },
    Player { idx: usize, playing: bool },
    Piano { key: Option<usize>, at: Instant },
    SaverPick,
    Game { idx: usize, g: Box<dyn Game> },
    Ball(Option<usize>),
    ButtonTest,
    Diag,
    About,
}

fn screen_code(s: &Screen) -> u32 {
    match s {
        Screen::Boot(_) => 0,
        Screen::Home => 1,
        Screen::Menu(..) => 2,
        Screen::Help { .. } => 3,
        Screen::PwGen { .. } => 4,
        Screen::Dice { .. } => 5,
        Screen::Rand { .. } => 6,
        Screen::Stopwatch { .. } => 7,
        Screen::Timer { .. } => 8,
        Screen::Entropy(_) => 9,
        Screen::Seed { .. } => 11,
        Screen::Mandel(_) => 12,
        Screen::Clock { .. } => 13,
        Screen::Player { .. } => 14,
        Screen::Piano { .. } => 15,
        Screen::SaverPick => 16,
        Screen::Ball(_) => 10,
        Screen::ButtonTest => 20,
        Screen::Diag => 21,
        Screen::About => 22,
        Screen::Game { idx, .. } => 30 + *idx as u32,
    }
}

/// Where BACK leads from a screen.
fn parent(s: &Screen) -> Screen {
    match s {
        Screen::Boot(_) => Screen::Home,
        Screen::Home => Screen::Menu(MenuId::Main, 0),
        Screen::Menu(MenuId::Main, _) => Screen::Home,
        Screen::Menu(MenuId::Utilities, _) => Screen::Menu(MenuId::Main, 0),
        Screen::Menu(MenuId::Games, _) => Screen::Menu(MenuId::Main, 1),
        Screen::Menu(MenuId::Music, _) => Screen::Menu(MenuId::Main, 2),
        Screen::Menu(MenuId::Lab, _) => Screen::Menu(MenuId::Main, 3),
        Screen::Menu(MenuId::Settings, _) => Screen::Menu(MenuId::Main, 4),
        Screen::Help { then, .. } => match then {
            Then::Game(i) => Screen::Menu(MenuId::Games, *i),
            Then::Open(b) => parent(b),
        },
        Screen::PwGen { from, .. } => Screen::Menu(MenuId::Utilities, *from),
        Screen::Dice { .. } => Screen::Menu(MenuId::Utilities, 3),
        Screen::Rand { .. } => Screen::Menu(MenuId::Utilities, 4),
        Screen::Stopwatch { .. } => Screen::Menu(MenuId::Utilities, 5),
        Screen::Timer { .. } => Screen::Menu(MenuId::Utilities, 6),
        Screen::Entropy(_) => Screen::Menu(MenuId::Utilities, 7),
        Screen::Seed { .. } => Screen::Menu(MenuId::Utilities, 8),
        Screen::Mandel(_) => Screen::Menu(MenuId::Lab, 0),
        Screen::Clock { .. } => Screen::Menu(MenuId::Utilities, 9),
        Screen::Player { idx, .. } => Screen::Menu(MenuId::Music, *idx),
        Screen::Piano { .. } => Screen::Menu(MenuId::Music, 2),
        Screen::SaverPick => Screen::Menu(MenuId::Settings, 0),
        Screen::Game { idx, .. } => Screen::Menu(MenuId::Games, *idx),
        Screen::Ball(_) => Screen::Menu(MenuId::Games, 8),
        Screen::ButtonTest => Screen::Menu(MenuId::Settings, 3),
        Screen::Diag => Screen::Menu(MenuId::Settings, 4),
        Screen::About => Screen::Menu(MenuId::Settings, 5),
    }
}

fn bidx(b: Btn) -> usize {
    match b {
        Btn::Ok => 0,
        Btn::Up => 1,
        Btn::Down => 2,
        Btn::Left => 3,
        Btn::Right => 4,
        Btn::Back => 5,
    }
}
const BTN_NAMES: [&str; 6] = ["OK", "UP", "DOWN", "LEFT", "RIGHT", "BACK"];

fn beeps_for(s: Snd) -> &'static [sound::Note] {
    match s {
        Snd::Click => sound::CLICK,
        Snd::Coin => sound::COIN,
        Snd::Crash => sound::CRASH,
        Snd::Tick => sound::TICK,
    }
}

fn fmt_time(d: Duration) -> String {
    let ms = d.as_millis() as u64;
    format!("{:02}:{:02}.{:02}", ms / 60000, (ms / 1000) % 60, (ms % 1000) / 10)
}

/// Shorten `s` to at most `n` characters.
fn cut(s: &str, n: usize) -> &str { if s.len() <= n { s } else { &s[..n] } }

struct App {
    display: Option<Display>,
    canvas: Canvas,
    prev: Frame,
    wipe_from: Option<Instant>,
    trng: Trng,
    usb: UsbHid,
    snd: Sound,
    screen: Screen,
    gen: u32,    // bumped on every screen change
    ok_gen: u32, // the screen generation OK was pressed on
    toast: Option<(String, Instant)>,
    help_seen: [bool; HELP_COUNT],
    started: Instant,
    down: [bool; 6],
    best: [u32; 8],
    sound_done: bool,
    anim_at: Instant,
    rep_at: Instant,
    last_input: Instant,
    saver: Option<Instant>,
    saver_kind: usize,
    clock: (u32, Instant),
    panics: u32,
    dirty: bool,
    last_secs: u64,
}

pub fn start(rx: Receiver<Input>) { std::thread::Builder::new().name("app".into()).spawn(move || run(rx)).ok(); }

fn run(rx: Receiver<Input>) {
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {}", info);
    }));
    let xns = xous_names::XousNames::new().unwrap();
    let tt = ticktimer::Ticktimer::new().unwrap();
    tt.sleep_ms(3500).ok(); // let USB, the console and the I2C service start
    let trng = Trng::new(&xns).unwrap();
    let usb = UsbHid::new();
    crate::bootmode::usb_reconnect();
    // Safe-mode switches, read from buttons held while the device boots (see BUILD_GUIDE.md):
    //   hold BACK = no display and no sound (nothing touches I2C or the BIO), hold DOWN = no sound only.
    let mut held = [false; 6];
    while let Ok(i) = rx.try_recv() {
        match i {
            Input::Press(b) => held[bidx(b)] = true,
            Input::Release(b, _) => held[bidx(b)] = false,
        }
    }
    let no_display = held[bidx(Btn::Back)];
    let no_sound = no_display || held[bidx(Btn::Down)];
    crate::diag::MODE.store(no_display as u32 | (no_sound as u32) << 1, Ordering::Relaxed);
    let display = if no_display { None } else { Display::start() };
    let snd = if no_sound { Sound::disabled() } else { Sound::start() };
    log::info!("app started, display {}", if display.is_some() { "found" } else { "not found" });
    let mut app = App {
        display,
        canvas: Canvas::new(),
        prev: [0; crate::oled::W * crate::oled::H / 8],
        wipe_from: None,
        trng,
        usb,
        snd,
        screen: Screen::Boot(Instant::now()),
        gen: 0,
        ok_gen: u32::MAX,
        toast: None,
        help_seen: [false; HELP_COUNT],
        started: Instant::now(),
        down: held,
        best: [0; 8],
        sound_done: false,
        anim_at: Instant::now(),
        rep_at: Instant::now(),
        last_input: Instant::now(),
        saver: None,
        saver_kind: 0,
        clock: (12 * 3600, Instant::now()),
        panics: 0,
        dirty: true,
        last_secs: 0,
    };
    crate::diag::init();
    let mut next = Instant::now();
    loop {
        let loop_t0 = Instant::now();
        crate::diag::bump(&crate::diag::TICKS);
        // drain button events
        let mut gone = false;
        loop {
            match rx.try_recv() {
                Ok(i) => {
                    if catch_unwind(AssertUnwindSafe(|| app.on_input(i))).is_err() {
                        app.recover();
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    gone = true;
                    break;
                }
            }
        }
        if gone {
            return;
        }
        // timers, drawing, watchdog
        if catch_unwind(AssertUnwindSafe(|| app.tick())).is_err() {
            app.recover();
        }
        if app.dirty {
            app.dirty = false;
            if catch_unwind(AssertUnwindSafe(|| app.draw())).is_err() {
                app.recover();
            }
        }
        if let Some(d) = &app.display {
            let stuck = d.stuck_ms();
            crate::diag::STUCK_MS.store(stuck, Ordering::Relaxed);
            if stuck > WATCHDOG_MS && app.started.elapsed() > Duration::from_secs(8) {
                log::error!("display bus stuck, restarting");
                crate::diag::bump(&crate::diag::WD_TRIPS);
                crate::bootmode::restart();
            }
        }
        crate::diag::max(&crate::diag::LOOP_MAX_MS, loop_t0.elapsed().as_millis() as u32);
        // fixed-rate schedule
        next += Duration::from_millis(TICK_MS);
        let now = Instant::now();
        if next > now {
            tt.sleep_ms(((next - now).as_millis() as usize).max(1)).ok();
        } else {
            next = now;
        }
    }
}

impl App {
    /// Switch screens (with a short wipe from the old picture to the new one).
    fn goto(&mut self, s: Screen) {
        crate::diag::SCREEN.store(screen_code(&s), Ordering::Relaxed);
        self.prev.copy_from_slice(&self.canvas.fb);
        self.wipe_from = Some(Instant::now());
        self.screen = s;
        self.gen = self.gen.wrapping_add(1);
        self.dirty = true;
    }

    /// Something went wrong on this screen: go home, keep running.
    fn recover(&mut self) {
        log::error!("recovered from a panic, back to HOME");
        self.panics += 1;
        crate::diag::bump(&crate::diag::PANICS);
        self.toast = Some(("recovered".into(), Instant::now()));
        self.goto(Screen::Home);
    }

    fn toast(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
        self.dirty = true;
    }

    /// Open an app; the first time, show its how-to screen first.
    fn open(&mut self, help: usize, s: Screen) {
        if !self.help_seen[help] {
            self.help_seen[help] = true;
            self.goto(Screen::Help { id: help, then: Then::Open(Box::new(s)) });
        } else {
            self.goto(s);
        }
    }

    fn open_game(&mut self, idx: usize) {
        let help = H_GAME + idx;
        if !self.help_seen[help] {
            self.help_seen[help] = true;
            self.goto(Screen::Help { id: help, then: Then::Game(idx) });
        } else {
            self.start_game(idx);
        }
    }

    fn start_game(&mut self, idx: usize) {
        let g = (GAMES[idx].make)(&self.trng);
        let tune: Option<&'static [sound::Note]> = match idx {
            0 => Some(sound::TUNE_DODGER),
            1 => Some(sound::TUNE_SNAKE),
            2 => Some(sound::TUNE_PONG),
            3 => Some(sound::TUNE_FLAPPY),
            6 => Some(sound::TUNE_DOOM),
            7 => Some(sound::TUNE_PACMAN),
            _ => None,
        };
        self.snd.tune(tune);
        self.goto(Screen::Game { idx, g });
    }

    fn new_pw(&self, prof: usize, len: usize) -> String {
        rng::generate(&self.trng, &rng::PROFILES[prof], len).unwrap_or_default()
    }

    // ------------------------------------------------------------------ input
    fn on_input(&mut self, inp: Input) {
        self.last_input = Instant::now();
        if self.saver.is_some() {
            // any button wakes the screen; the press itself is swallowed and everything resumes where it was
            self.saver = None;
            self.dirty = true;
            return;
        }
        match inp {
            Input::Press(b) => {
                self.down[bidx(b)] = true;
                if matches!(self.screen, Screen::ButtonTest) {
                    self.dirty = true;
                }
                if matches!(self.screen, Screen::Boot(_)) {
                    self.goto(Screen::Home); // any button skips the logo
                    return;
                }
                if b == Btn::Ok {
                    self.ok_gen = self.gen;
                }
                self.on_press(b);
            }
            Input::Release(b, held) => {
                self.down[bidx(b)] = false;
                if matches!(self.screen, Screen::ButtonTest) {
                    self.dirty = true;
                }
                if b == Btn::Ok && self.ok_gen == self.gen {
                    self.ok_gen = u32::MAX;
                    self.on_ok(held);
                }
            }
        }
    }

    fn on_press(&mut self, b: Btn) {
        if b == Btn::Back {
            // in a game: first BACK pauses, the second BACK quits
            if let Screen::Game { g, .. } = &mut self.screen {
                if g.can_pause() && !g.over() && !g.is_paused() {
                    g.set_paused(true);
                    self.dirty = true;
                    return;
                }
            }
            self.snd.silence();
            let to = parent(&self.screen);
            self.goto(to);
            return;
        }
        let mut redraw = false;
        let mut click = false;
        let mut restart = None;
        let mut rep = false;
        let mut help_ok = false;
        match &mut self.screen {
            Screen::Menu(m, sel) => {
                let n = items(*m).len();
                match b {
                    Btn::Up => {
                        *sel = (*sel + n - 1) % n;
                        redraw = true;
                        click = true;
                    }
                    Btn::Down => {
                        *sel = (*sel + 1) % n;
                        redraw = true;
                        click = true;
                    }
                    _ => {}
                }
            }
            Screen::Help { .. } => {
                if b == Btn::Ok {
                    help_ok = true;
                }
            }
            Screen::PwGen { prof, len, pw, .. } => {
                let n = rng::PROFILES.len();
                let mut changed = true;
                match b {
                    Btn::Up => {
                        *prof = (*prof + n - 1) % n;
                        *len = rng::PROFILES[*prof].default_len;
                    }
                    Btn::Down => {
                        *prof = (*prof + 1) % n;
                        *len = rng::PROFILES[*prof].default_len;
                    }
                    Btn::Left => *len = (*len).saturating_sub(1).max(rng::PROFILES[*prof].len_min),
                    Btn::Right => *len = (*len + 1).min(rng::PROFILES[*prof].len_max),
                    _ => changed = false,
                }
                if changed {
                    *pw = rng::generate(&self.trng, &rng::PROFILES[*prof], *len).unwrap_or_default();
                    redraw = true;
                    click = matches!(b, Btn::Up | Btn::Down);
                    rep = matches!(b, Btn::Left | Btn::Right);
                }
            }
            Screen::Seed { n, idx, page } => {
                let pages = (*n + 3) / 4;
                let mut changed = false;
                match b {
                    Btn::Up => *page = (*page + pages - 1) % pages,
                    Btn::Down => *page = (*page + 1) % pages,
                    Btn::Left | Btn::Right => {
                        *n = if b == Btn::Left { (*n).saturating_sub(3).max(12) } else { (*n + 3).min(24) };
                        changed = true;
                    }
                    Btn::Ok => changed = true,
                    _ => {}
                }
                if changed {
                    *idx = rng::seed_words(&self.trng, *n).unwrap_or_default();
                    *page = 0;
                }
                redraw = matches!(b, Btn::Up | Btn::Down | Btn::Left | Btn::Right | Btn::Ok);
                click = matches!(b, Btn::Up | Btn::Down);
                rep = matches!(b, Btn::Left | Btn::Right);
            }
            Screen::Mandel(m) => {
                match b {
                    Btn::Ok => m.auto = !m.auto,
                    Btn::Up => m.zoom(true),
                    Btn::Down => m.zoom(false),
                    Btn::Left => m.next_target(-1),
                    Btn::Right => m.next_target(1),
                    _ => {}
                }
                redraw = true;
                click = matches!(b, Btn::Left | Btn::Right);
            }
            Screen::Clock { edit } => {
                if *edit {
                    let day = 24 * 3600;
                    let now_s = (self.clock.0 + self.clock.1.elapsed().as_secs() as u32) % day;
                    let d: i64 = match b {
                        Btn::Up => 3600,
                        Btn::Down => -3600,
                        Btn::Right => 60,
                        Btn::Left => -60,
                        _ => 0,
                    };
                    if d != 0 {
                        let v = (now_s as i64 + d).rem_euclid(day as i64) as u32;
                        self.clock = (v, Instant::now());
                        redraw = true;
                        click = true;
                    }
                }
            }
            Screen::Piano { key, at } => {
                let k = match b {
                    Btn::Left => Some(0),
                    Btn::Up => Some(1),
                    Btn::Right => Some(2),
                    Btn::Down => Some(3),
                    Btn::Ok => Some(4),
                    _ => None,
                };
                if let Some(k) = k {
                    *key = Some(k);
                    *at = Instant::now();
                    self.snd.play(&[(crate::music::KEYS[k].1, 220)]);
                    redraw = true;
                }
            }
            Screen::SaverPick => {
                let n = crate::saver::COUNT;
                match b {
                    Btn::Up | Btn::Left => self.saver_kind = (self.saver_kind + n - 1) % n,
                    Btn::Down | Btn::Right => self.saver_kind = (self.saver_kind + 1) % n,
                    _ => {}
                }
                redraw = true;
                click = true;
            }
            Screen::Dice { p, cur } => match b {
                Btn::Up => {
                    *p = (*p + DICE.len() - 1) % DICE.len();
                    cur.clear();
                    redraw = true;
                    click = true;
                }
                Btn::Down => {
                    *p = (*p + 1) % DICE.len();
                    cur.clear();
                    redraw = true;
                    click = true;
                }
                _ => {}
            },
            Screen::Rand { r, result } => match b {
                Btn::Up => {
                    *r = (*r + RANGES.len() - 1) % RANGES.len();
                    *result = None;
                    redraw = true;
                    click = true;
                }
                Btn::Down => {
                    *r = (*r + 1) % RANGES.len();
                    *result = None;
                    redraw = true;
                    click = true;
                }
                _ => {}
            },
            Screen::Game { idx, g } => {
                if b == Btn::Ok && g.over() {
                    restart = Some(*idx); // OK after game over = play again
                } else {
                    redraw = g.press(b, &self.trng);
                }
            }
            Screen::Stopwatch { running, started, acc } => match b {
                Btn::Ok => {
                    if *running {
                        *acc += started.elapsed();
                        *running = false;
                    } else {
                        *started = Instant::now();
                        *running = true;
                    }
                    redraw = true;
                    click = true;
                }
                Btn::Up | Btn::Left => {
                    *running = false;
                    *acc = Duration::ZERO;
                    redraw = true;
                }
                _ => {}
            },
            Screen::Timer { set_s, running, end, left_ms, done } => match b {
                Btn::Ok => {
                    if *done {
                        *done = false;
                        *left_ms = 0;
                    } else if *running {
                        *left_ms = end.saturating_duration_since(Instant::now()).as_millis() as u32;
                        *running = false;
                    } else {
                        let ms = if *left_ms > 0 { *left_ms } else { *set_s * 1000 };
                        if ms > 0 {
                            *end = Instant::now() + Duration::from_millis(ms as u64);
                            *running = true;
                        }
                    }
                    redraw = true;
                    click = true;
                }
                _ if !*running => {
                    let d: i32 = match b {
                        Btn::Right => 10,
                        Btn::Left => -10,
                        Btn::Up => 60,
                        Btn::Down => -60,
                        _ => 0,
                    };
                    if d != 0 {
                        *set_s = (*set_s as i32 + d).clamp(0, 99 * 60 + 59) as u32;
                        *left_ms = 0;
                        *done = false;
                        redraw = true;
                        click = true;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        if rep {
            self.rep_at = Instant::now() + Duration::from_millis(400);
        }
        if click {
            self.snd.play(sound::CLICK);
        }
        if help_ok {
            self.leave_help();
        } else if let Some(idx) = restart {
            self.start_game(idx);
        } else if redraw {
            self.dirty = true;
        }
    }

    fn leave_help(&mut self) {
        let old = std::mem::replace(&mut self.screen, Screen::Home);
        if let Screen::Help { then, .. } = old {
            self.snd.play(sound::CLICK);
            match then {
                Then::Game(i) => self.start_game(i),
                Then::Open(b) => self.goto(*b),
            }
        }
    }

    fn on_ok(&mut self, held_ms: u32) {
        enum Act {
            None,
            Select(MenuId, usize),
            PwOk(bool),
            PlayerToggle,
            ClockEdit,
            Roll,
            Draw,
            Shake,
        }
        let act = match &self.screen {
            Screen::Menu(m, sel) => Act::Select(*m, *sel),
            Screen::PwGen { .. } => Act::PwOk(held_ms >= 800),
            Screen::Player { .. } => Act::PlayerToggle,
            Screen::Clock { .. } => Act::ClockEdit,
            Screen::Dice { .. } => Act::Roll,
            Screen::Rand { .. } => Act::Draw,
            Screen::Ball(_) => Act::Shake,
            _ => Act::None,
        };
        match act {
            Act::None => {}
            Act::Select(m, sel) => self.select(m, sel),
            Act::PwOk(type_it) => {
                let mut to_type: Option<String> = None;
                if let Screen::PwGen { prof, len, pw, .. } = &mut self.screen {
                    if type_it {
                        to_type = Some(pw.clone());
                    } else {
                        *pw = rng::generate(&self.trng, &rng::PROFILES[*prof], *len).unwrap_or_default();
                    }
                }
                if let Some(text) = to_type {
                    self.snd.play(sound::COIN);
                    self.type_text(&text);
                    self.toast("typed into the PC");
                } else {
                    self.snd.play(sound::CLICK);
                }
                self.dirty = true;
            }
            Act::PlayerToggle => {
                let mut tune = None;
                if let Screen::Player { idx, playing } = &mut self.screen {
                    *playing = !*playing;
                    if *playing {
                        tune = Some(crate::music::TUNES[*idx].notes);
                    }
                }
                self.snd.tune(tune);
                self.dirty = true;
            }
            Act::ClockEdit => {
                if let Screen::Clock { edit } = &mut self.screen {
                    *edit = !*edit;
                }
                self.snd.play(sound::CLICK);
                self.dirty = true;
            }
            Act::Roll => self.roll(),
            Act::Draw => {
                if let Screen::Rand { r, result } = &mut self.screen {
                    *result = Some(rng::below(&self.trng, RANGES[*r]).unwrap_or(0) + 1);
                }
                self.snd.play(sound::CLICK);
                self.dirty = true;
            }
            Act::Shake => {
                let n = rng::below(&self.trng, BALL.len() as u32).unwrap_or(0) as usize;
                self.snd.play(sound::COIN);
                self.screen = Screen::Ball(Some(n)); // same screen, keep `gen`
                self.dirty = true;
            }
        }
    }

    // ------------------------------------------------------------------ actions
    fn type_text(&mut self, text: &str) {
        // errors (no USB host connected) are ignored on purpose
        self.usb.send_str(&crate::typing::as_us_keyboard(text)).ok();
    }

    fn select(&mut self, m: MenuId, sel: usize) {
        self.snd.play(sound::CLICK);
        match (m, sel) {
            (MenuId::Main, 0) => self.goto(Screen::Menu(MenuId::Utilities, 0)),
            (MenuId::Main, 1) => self.goto(Screen::Menu(MenuId::Games, 0)),
            (MenuId::Main, 2) => self.goto(Screen::Menu(MenuId::Music, 0)),
            (MenuId::Main, 3) => self.goto(Screen::Menu(MenuId::Lab, 0)),
            (MenuId::Main, 4) => self.goto(Screen::Menu(MenuId::Settings, 0)),
            (MenuId::Utilities, 0) => {
                let prof = 0;
                let len = rng::PROFILES[prof].default_len;
                let pw = self.new_pw(prof, len);
                self.open(H_PW, Screen::PwGen { prof, len, pw, from: 0 });
            }
            (MenuId::Utilities, 1) => {
                let prof = rng::PROFILE_WORDS;
                let len = rng::PROFILES[prof].default_len;
                let pw = self.new_pw(prof, len);
                self.open(H_WORDS, Screen::PwGen { prof, len, pw, from: 1 });
            }
            (MenuId::Utilities, 2) => {
                let prof = rng::PROFILE_PIN;
                let len = rng::PROFILES[prof].default_len;
                let pw = self.new_pw(prof, len);
                self.open(H_PIN, Screen::PwGen { prof, len, pw, from: 2 });
            }
            (MenuId::Utilities, 3) => self.open(H_DICE, Screen::Dice { p: 1, cur: String::new() }),
            (MenuId::Utilities, 4) => self.open(H_RAND, Screen::Rand { r: 3, result: None }),
            (MenuId::Utilities, 5) => {
                self.open(H_STOP, Screen::Stopwatch { running: false, started: Instant::now(), acc: Duration::ZERO })
            }
            (MenuId::Utilities, 6) => self.open(
                H_TIMER,
                Screen::Timer { set_s: 60, running: false, end: Instant::now(), left_ms: 0, done: false },
            ),
            (MenuId::Utilities, 7) => {
                self.open(H_ENT, Screen::Entropy(Entropy { total: 0, ones: 0, bins: [0; 16] }))
            }
            (MenuId::Utilities, 8) => {
                let idx = rng::seed_words(&self.trng, 12).unwrap_or_default();
                self.open(H_SEED, Screen::Seed { n: 12, idx, page: 0 })
            }
            (MenuId::Games, i) if i < GAMES.len() => self.open_game(i),
            (MenuId::Games, 8) => self.open(H_GAME + 8, Screen::Ball(None)),
            (MenuId::Utilities, 9) => self.open(H_CLOCK, Screen::Clock { edit: false }),
            (MenuId::Music, 0) | (MenuId::Music, 1) => {
                self.snd.silence();
                self.open(H_PLAYER, Screen::Player { idx: sel, playing: false })
            }
            (MenuId::Music, 2) => self.open(H_PIANO, Screen::Piano { key: None, at: Instant::now() }),
            (MenuId::Lab, 0) => self.open(H_MANDEL, Screen::Mandel(Box::new(crate::mandel::Mandel::new()))),
            (MenuId::Lab, 1) => {
                self.snd.play(sound::SOS);
                self.toast("... --- ...");
            }
            (MenuId::Settings, 0) => self.goto(Screen::SaverPick),
            (MenuId::Settings, 1) => {
                let on = self.snd.toggle();
                self.toast(if on { "sound ON" } else { "sound OFF" });
            }
            (MenuId::Settings, 2) => {
                self.snd.play(sound::SCALE);
                self.toast("playing a scale");
            }
            (MenuId::Settings, 3) => self.goto(Screen::ButtonTest),
            (MenuId::Settings, 4) => self.goto(Screen::Diag),
            (MenuId::Settings, 5) => self.goto(Screen::About),
            _ => {}
        }
    }

    fn roll(&mut self) {
        let mut text = String::new();
        if let Screen::Dice { p, .. } = &self.screen {
            let (n, sides, label) = DICE[*p];
            let mut total = 0;
            let mut parts: Vec<String> = Vec::new();
            for _ in 0..n {
                let r = rng::below(&self.trng, sides).unwrap_or(0) + 1;
                total += r;
                parts.push(r.to_string());
            }
            text = if label == "coin" {
                if total == 1 { "heads".to_string() } else { "tails".to_string() }
            } else if n > 1 {
                format!("{}={}", parts.join("+"), total)
            } else {
                total.to_string()
            };
        }
        if let Screen::Dice { cur, .. } = &mut self.screen {
            *cur = text;
        }
        self.snd.play(sound::CLICK);
        self.dirty = true;
    }

    // ------------------------------------------------------------------ timers
    fn tick(&mut self) {
        let now = Instant::now();
        let mut to_home = false;
        let mut alarm = false;
        // screensaver: after SAVER_S seconds without a button the screen goes to a moving bun, and games pause
        let idle_ok = !matches!(self.screen, Screen::Boot(_))
            && !matches!(self.screen, Screen::Timer { running: true, .. })
            && !matches!(self.screen, Screen::Timer { done: true, .. });
        if self.saver.is_none() && idle_ok && now.duration_since(self.last_input) > Duration::from_secs(SAVER_S) {
            self.saver = Some(now);
            if let Screen::Game { g, .. } = &mut self.screen {
                if g.can_pause() && !g.over() {
                    g.set_paused(true);
                }
            }
            self.snd.silence();
            self.dirty = true;
        }
        if self.saver.is_some() {
            if now.duration_since(self.anim_at) >= Duration::from_millis(60) {
                self.anim_at = now;
                self.dirty = true;
            }
            return;
        }
        if let Some((_, t0)) = &self.toast {
            if now.duration_since(*t0) > Duration::from_millis(1600) {
                self.toast = None;
            }
            self.dirty = true;
        }
        if self.wipe_from.is_some() {
            self.dirty = true;
        }
        match &mut self.screen {
            Screen::Boot(t0) => {
                let ms = t0.elapsed().as_millis() as u32;
                if !self.sound_done {
                    self.sound_done = true;
                    self.snd.play(sound::HELLO);
                }
                if ms >= logo::BOOT_MS {
                    to_home = true;
                } else {
                    self.dirty = true;
                }
            }
            Screen::Home => {
                if now.duration_since(self.anim_at) >= Duration::from_millis(120) {
                    self.anim_at = now;
                    self.dirty = true;
                }
            }
            Screen::Game { idx, g } => {
                g.held(&self.down);
                if g.update(&self.trng) {
                    self.dirty = true;
                }
                if let Some(s) = g.take_sound() {
                    self.snd.play(beeps_for(s));
                    self.dirty = true;
                }
                let sc = g.score();
                let b = &mut self.best[*idx];
                if g.lower_is_better() {
                    if g.over() && sc > 0 && (*b == 0 || sc < *b) {
                        *b = sc;
                    }
                } else if sc > *b {
                    *b = sc;
                }
            }
            Screen::PwGen { prof, len, pw, .. } => {
                // holding LEFT / RIGHT keeps changing the length
                let (l, r) = (self.down[bidx(Btn::Left)], self.down[bidx(Btn::Right)]);
                if l != r && now >= self.rep_at {
                    let p = &rng::PROFILES[*prof];
                    *len = if l { (*len).saturating_sub(1).max(p.len_min) } else { (*len + 1).min(p.len_max) };
                    *pw = rng::generate(&self.trng, p, *len).unwrap_or_default();
                    self.rep_at = now + Duration::from_millis(90);
                    self.dirty = true;
                }
            }
            Screen::Stopwatch { running, .. } => {
                if *running {
                    self.dirty = true;
                }
            }
            Screen::Timer { running, end, left_ms, done, .. } => {
                if *running {
                    if now >= *end {
                        *running = false;
                        *done = true;
                        *left_ms = 0;
                        alarm = true;
                    }
                    self.dirty = true;
                }
                if *done && now.duration_since(self.anim_at) >= Duration::from_millis(350) {
                    self.anim_at = now;
                    alarm = true;
                }
            }
            Screen::Entropy(e) => {
                for _ in 0..4 {
                    if let Ok(x) = self.trng.get_u32() {
                        e.total += 32;
                        e.ones += x.count_ones();
                        for k in 0..8 {
                            e.bins[((x >> (4 * k)) & 15) as usize] += 1;
                        }
                    }
                }
                if now.duration_since(self.anim_at) >= Duration::from_millis(100) {
                    self.anim_at = now;
                    self.dirty = true;
                }
            }
            Screen::Mandel(m) => {
                // render a slice of rows (about 15 ms), then once a frame is complete and auto is on, zoom in
                let changed = m.step_render(Duration::from_millis(15));
                if m.finished() && m.auto && now.duration_since(self.anim_at) >= Duration::from_millis(150) {
                    self.anim_at = now;
                    m.zoom(true);
                }
                if changed {
                    self.dirty = true;
                }
            }
            Screen::Clock { .. } | Screen::Player { .. } | Screen::SaverPick | Screen::Piano { .. } => {
                if now.duration_since(self.anim_at) >= Duration::from_millis(70) {
                    self.anim_at = now;
                    self.dirty = true;
                }
            }
            Screen::Diag | Screen::About => {
                let secs = self.started.elapsed().as_secs();
                if secs != self.last_secs {
                    self.last_secs = secs;
                    self.dirty = true;
                }
            }
            _ => {}
        }
        if alarm {
            self.snd.play(sound::HELLO);
            self.dirty = true;
        }
        if to_home {
            self.goto(Screen::Home);
        }
    }

    // ------------------------------------------------------------------ drawing
    fn draw(&mut self) {
        let Some(disp) = self.display.clone() else { return };
        let oled = &mut self.canvas;
        oled.clear();
        if let Some(t0) = self.saver {
            // screensaver: always moving, few lit pixels; much later the screen is fully dark
            let el = t0.elapsed().as_millis() as u32;
            if el < SAVER_BLANK_MS {
                crate::saver::draw(oled, self.saver_kind, el);
            }
            disp.submit(&self.canvas);
            return;
        }
        match &self.screen {
            Screen::Boot(t0) => logo::boot_frame(oled, t0.elapsed().as_millis() as u32),
            Screen::Home => logo::home_frame(oled, self.started.elapsed().as_millis() as u32),
            Screen::Menu(m, sel) => {
                let list = items(*m);
                oled.text(0, 0, menu_title(*m));
                let idx_txt = format!("{}/{}", sel + 1, list.len());
                oled.text(128 - 6 * idx_txt.len(), 0, &idx_txt);
                let start = if *sel >= 3 { *sel - 2 } else { 0 };
                for k in 0..3 {
                    let idx = start + k;
                    if idx < list.len() {
                        let y = 13 + 13 * k;
                        if idx != *sel {
                            oled.card(1, y, 124, 12); // raised soft button
                        }
                        oled.icon(4, y + 2, list[idx].icon);
                        oled.text(16, y, list[idx].label);
                        if idx == *sel {
                            oled.invert_rect(1, y, 124, 12); // the pressed one is lit
                        }
                    }
                }
                if list.len() > 3 {
                    let h = 39 * 3 / list.len();
                    let y = 13 + (39 - h) * *sel / (list.len() - 1);
                    oled.fill_rect(126, y, 2, h, true);
                }
                let footer = match &self.toast {
                    Some((t, _)) => t.as_str(),
                    None => list[*sel].desc,
                };
                oled.text(0, 52, cut(footer, 21));
            }
            Screen::Help { id, .. } => {
                let (title, lines) = help_text(*id);
                oled.rect(0, 0, 128, 64);
                oled.text(4, 3, title);
                for (i, l) in lines.iter().enumerate() {
                    oled.text(4, 16 + 11 * i, l);
                }
                oled.invert_rect(1, 1, 126, 14);
                oled.text(4, 52, "OK = start  BACK = no");
            }
            Screen::PwGen { prof, len, .. } => {
                let p = &rng::PROFILES[*prof];
                // raised panel: type (UP/DOWN) with its position in the list
                oled.card(0, 0, 128, 13);
                oled.arrow(3, 2, 0);
                oled.arrow(3, 7, 1);
                oled.text(12, 0, cut(p.name, 17));
                let c = format!("{}/{}", prof + 1, rng::PROFILES.len());
                oled.text(126 - 6 * c.len(), 0, &c);
                // raised panel: length (LEFT/RIGHT) and strength
                oled.card(0, 14, 128, 13);
                oled.arrow(3, 18, 2);
                oled.arrow(7, 18, 3);
                let unit = if p.kind == rng::Kind::Words { "words" } else { "chars" };
                oled.text(14, 14, &format!("{} {} ~{} bits", len, unit, rng::bits(p, *len)));
                // sunken panel: the secret is never shown; it is only typed into the PC
                oled.inset(0, 28, 128, 36);
                let masked = "*".repeat((*len).min(20));
                oled.text(4, 30, &masked);
                oled.text(4, 41, "Click the input box,");
                oled.text(4, 52, "then HOLD OK to type");
            }
            Screen::Seed { n, idx, page } => {
                oled.card(0, 0, 128, 13);
                oled.text(4, 0, &format!("Seed {} words", n));
                let pg = format!("{}/{}", page + 1, (*n + 3) / 4);
                oled.text(126 - 6 * pg.len(), 0, &pg);
                oled.inset(0, 14, 128, 50);
                for k in 0..4 {
                    let i = page * 4 + k;
                    if let Some(&w) = idx.get(i) {
                        oled.text(4, 17 + 11 * k, &format!("{:>2} {}", i + 1, rng::bip39_word(w)));
                    }
                }
                oled.text(4, 55, "OK=new  <>=count");
            }
            Screen::SaverPick => {
                crate::saver::draw(oled, self.saver_kind, self.started.elapsed().as_millis() as u32);
                let name = crate::saver::NAMES[self.saver_kind];
                oled.fill_rect(0, 56, 128, 8, false);
                oled.text(0, 56, &format!("{} {}/{} <>", name, self.saver_kind + 1, crate::saver::COUNT));
            }
            Screen::Clock { edit } => {
                let day = 24 * 3600;
                let s = (self.clock.0 + self.clock.1.elapsed().as_secs() as u32) % day;
                oled.card(0, 0, 128, 13);
                oled.text(4, 0, "Clock");
                oled.text(128 - 6 * 9, 0, if *edit { "SET: ^v <>" } else { "OK = set" });
                oled.inset(0, 16, 128, 34);
                oled.text_big(10, 22, &format!("{:02}:{:02}", s / 3600, (s / 60) % 60), 3);
                oled.text(112, 40, &format!("{:02}", s % 60));
                oled.text(0, 54, if *edit { "OK = done" } else { "starts 12:00 at boot" });
            }
            Screen::Player { idx, playing } => {
                let t = &crate::music::TUNES[*idx];
                oled.card(0, 0, 128, 13);
                oled.text(4, 0, t.title);
                oled.text(4, 16, t.by);
                // equalizer bars that dance while it plays
                let ms = self.started.elapsed().as_millis() as u32;
                for i in 0..16u32 {
                    let h = if *playing { 3 + ((ms / 90 + i * 37 + i * i).wrapping_mul(2654435761u32) >> 28) as usize * 2 } else { 2 };
                    oled.fill_rect((i * 8 + 1) as usize, 50 - h.min(24), 6, h.min(24), true);
                }
                oled.text(0, 54, if *playing { "OK = stop" } else { "OK = play" });
            }
            Screen::Piano { key, at } => {
                oled.text(0, 0, "Piano (BACK exits)");
                for (i, (name, hz)) in crate::music::KEYS.iter().enumerate() {
                    let x = 2 + i * 25;
                    let down = *key == Some(i) && at.elapsed() < Duration::from_millis(250);
                    oled.rect(x, 14, 22, 36);
                    if down {
                        oled.fill_rect(x + 1, 15, 20, 34, true);
                    }
                    oled.text(x + 1, 52, &name[..name.len().min(4)]);
                    let _ = hz;
                }
            }
            Screen::Mandel(m) => {
                for y in 0..crate::mandel::H {
                    for x in 0..crate::mandel::W {
                        if m.lit(x, y) {
                            oled.dot(x as i32, y as i32, true);
                        }
                    }
                }
                oled.fill_rect(0, 56, 128, 8, false);
                let z = m.zoom_x100;
                let zt = if z >= 100_000 { format!("{}k", z / 100_000) } else { format!("{}.{}", z / 100, (z % 100) / 10) };
                oled.text(0, 56, &format!("{} x{}{}", m.name(), zt, if m.auto { "" } else { " ||" }));
            }
            Screen::Dice { p, cur } => {
                oled.text(0, 0, "Dice and coin");
                oled.arrow(1, 15, 0);
                oled.arrow(1, 20, 1);
                oled.text(10, 13, &format!("Dice: {}   OK=roll", DICE[*p].2));
                if !cur.is_empty() {
                    oled.text_big(0, 28, cur, if cur.len() <= 7 { 3 } else { 2 });
                }
            }
            Screen::Rand { r, result } => {
                oled.text(0, 0, "Random number");
                oled.arrow(1, 15, 0);
                oled.arrow(1, 20, 1);
                oled.text(10, 13, &format!("1 to {}  OK=pick", RANGES[*r]));
                if let Some(v) = result {
                    let s = v.to_string();
                    oled.text_big(0, 28, &s, if s.len() <= 5 { 3 } else { 2 });
                }
            }
            Screen::Game { idx, g } => {
                g.draw(oled, self.best[*idx]);
                if g.is_paused() {
                    oled.fill_rect(6, 18, 116, 30, false);
                    oled.rect(6, 18, 116, 30);
                    oled.text(44, 22, "PAUSED");
                    oled.text(16, 34, "OK=go  BACK=quit");
                }
            }
            Screen::Stopwatch { running, started, acc } => {
                let t = *acc + if *running { started.elapsed() } else { Duration::ZERO };
                oled.text(0, 0, "Stopwatch");
                oled.text_big(4, 18, &fmt_time(t), 2);
                oled.text(0, 52, "OK start/stop  UP reset");
            }
            Screen::Timer { set_s, running, end, left_ms, done } => {
                oled.text(0, 0, "Timer");
                let ms = if *running {
                    end.saturating_duration_since(Instant::now()).as_millis() as u32
                } else if *left_ms > 0 {
                    *left_ms
                } else {
                    *set_s * 1000
                };
                let secs = (ms + 999) / 1000;
                oled.text_big(14, 18, &format!("{:02}:{:02}", secs / 60, secs % 60), 3);
                let hint = if *done {
                    "TIME'S UP!  OK=reset"
                } else if *running {
                    "OK = pause"
                } else {
                    "<>10s ^v1min OK=go"
                };
                oled.text(0, 52, hint);
                if *done && (self.started.elapsed().as_millis() / 350) % 2 == 0 {
                    oled.invert_rect(0, 0, 128, 64);
                }
            }
            Screen::Ball(a) => {
                oled.text(0, 0, "Magic 8-ball");
                match a {
                    None => {
                        oled.text(0, 20, "Think of a question,");
                        oled.text(0, 33, "then press OK.");
                    }
                    Some(n) => {
                        let t = BALL[*n];
                        if t.len() > 10 {
                            let cut_at = t[..t.len() / 2 + 3].rfind(' ').unwrap_or(t.len() / 2);
                            oled.text_big(0, 16, &t[..cut_at], 2);
                            oled.text_big(0, 36, t[cut_at..].trim_start(), 2);
                        } else {
                            oled.text_big(0, 22, t, 2);
                        }
                    }
                }
            }
            Screen::Entropy(e) => {
                oled.text(0, 0, "Entropy meter");
                let pct100 = if e.total > 0 { (e.ones as u64 * 10000 / e.total as u64) as u32 } else { 5000 };
                oled.text(0, 13, &format!("{} bits", e.total));
                oled.text(0, 26, &format!("ones {}.{:02}%", pct100 / 100, pct100 % 100));
                let max = e.bins.iter().copied().max().unwrap_or(1).max(1);
                for (i, &v) in e.bins.iter().enumerate() {
                    let h = (v as u64 * 22 / max as u64) as usize;
                    oled.fill_rect(i * 8 + 1, 63 - h, 6, h.max(1), true);
                }
            }
            Screen::ButtonTest => {
                oled.text(0, 0, "Button test (BACK exits)");
                for i in 0..5 {
                    let (col, row) = (i / 3, i % 3);
                    let state = if self.down[i] { "ON" } else { "--" };
                    oled.text(col * 66, 13 + 13 * row, &format!("{:<5}{}", BTN_NAMES[i], state));
                }
            }
            Screen::Diag => {
                let (ok, bad, slow) = disp.stats();
                let secs = self.started.elapsed().as_secs();
                oled.text(0, 0, "Diagnostics");
                oled.text(0, 13, &format!("up {}m{}s panics {}", secs / 60, secs % 60, self.panics));
                oled.text(0, 26, &format!("oled ok {} bad {}", ok, bad));
                oled.text(0, 39, &format!("slowest push {} ms", slow));
                oled.text(0, 52, &self.snd.status_text());
            }
            Screen::About => {
                let secs = self.started.elapsed().as_secs();
                oled.text(0, 0, concat!("ChocoBao v", env!("CARGO_PKG_VERSION")));
                oled.text(0, 13, "Baochip-1x + Xous");
                oled.text(0, 26, "350 MHz RISC-V, MMU");
                oled.text(0, 39, "4 BIO cores, TRNG");
                oled.text(0, 52, &format!("up {}m {}s", secs / 60, secs % 60));
            }
        }
        // short wipe from the previous picture to the new one
        if let Some(t0) = self.wipe_from {
            let el = t0.elapsed().as_millis();
            if el >= WIPE_MS {
                self.wipe_from = None;
            } else {
                oled.wipe(&self.prev, (el * 128 / WIPE_MS) as usize);
                self.dirty = true;
            }
        }
        disp.submit(&self.canvas);
    }
}
