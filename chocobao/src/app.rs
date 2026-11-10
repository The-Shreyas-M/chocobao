// app.rs - the ChocoBao "OS": a small menu system on the 128x64 OLED, driven by six buttons.
//
//   HOME (default screen)   OK = type a password into the PC, hold OK = type 6 words, BACK = menu
//   MENU                    UP / DOWN move, OK selects, BACK goes back
//     Passwords  - type or show a password / 6 BIP-39 words
//     Dice       - UP/DOWN pick the dice, OK rolls
//     Games      - Mempool Dodger, Snake, Pong, Flappy Bao, Simon, Reaction (OK pauses / flaps)
//     Tools      - Stopwatch, Magic 8-ball, Entropy meter (live TRNG test), Button test, Beep test,
//                  SOS, Sound on/off, Bootloader (software PROG + RESET)
//     About
//
// Design rules (so it behaves like a small RTOS and never wedges):
//  * One fixed 10 ms tick. Nothing in the loop waits for hardware except that tick.
//  * The display lives on its own thread (display.rs): a stuck I2C transfer cannot freeze the buttons or
//    the game. If it stays stuck for 4 s the app restarts itself (watchdog).
//  * Each step runs inside catch_unwind: a bug on one screen drops you back to HOME instead of killing the app.
//  * Press = instant event (controls.rs); holding LEFT/RIGHT auto-repeats in the game.
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
use crate::oled::Canvas;
use crate::rng;
use crate::logo;
use crate::sound::{self, Sound};

const TICK_MS: u64 = 16;
const WATCHDOG_MS: u32 = 20_000; // only a truly dead display bus should restart the app

#[derive(Clone, Copy, PartialEq)]
enum MenuId {
    Main,
    Passwords,
    Games,
    Tools,
}

fn items(m: MenuId) -> &'static [&'static str] {
    match m {
        MenuId::Main => &["Passwords", "Dice", "Games", "Tools", "About"],
        MenuId::Passwords => &["Site-rules password", "Type 6 words", "Show 6 words", "Show PIN (6)"],
        MenuId::Games => &["Mempool Dodger", "Snake", "Pong", "Flappy Bao", "Simon", "Reaction", "ChocoDoom"],
        MenuId::Tools => &[
            "Stopwatch",
            "Magic 8-ball",
            "Entropy meter",
            "Button test",
            "Buzzer test",
            "SOS beeps",
            "Sound on/off",
            "Diagnostics",
            "Bootloader",
        ],
    }
}

fn title(m: MenuId) -> &'static str {
    match m {
        MenuId::Main => "ChocoBao menu",
        MenuId::Passwords => "Passwords",
        MenuId::Games => "Games",
        MenuId::Tools => "Tools",
    }
}

const DICE: [(u32, u32, &str); 5] =
    [(1, 6, "1d6"), (2, 6, "2d6"), (1, 20, "1d20"), (1, 100, "1d100"), (1, 2, "coin")];

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

enum Screen {
    Boot(Instant),
    Home,
    Menu(MenuId, usize),
    Dice(usize, String),
    Show { title: String, lines: Vec<String>, back: MenuId },
    Countdown { words: bool, until: Instant },
    PwGen { prof: usize, len: usize, pw: String },
    Game { idx: usize, g: Box<dyn Game> },
    Stopwatch { running: bool, started: Instant, acc: Duration },
    Ball(Option<usize>),
    Entropy(Entropy),
    ButtonTest,
    Diag,
    About,
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

struct App {
    display: Option<Display>,
    canvas: Canvas,
    trng: Trng,
    usb: UsbHid,
    snd: Sound,
    screen: Screen,
    gen: u32,    // bumped on every screen change
    ok_gen: u32, // the screen generation OK was pressed on
    status: String,
    started: Instant,
    down: [bool; 6],
    best: [u32; 8],
    sound_done: bool,
    anim_at: Instant,
    rep_at: Instant,
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
    // Safe-mode switches, read from buttons held while the device boots (see HANDOFF.md):
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
        trng,
        usb,
        snd,
        screen: Screen::Boot(Instant::now()),
        gen: 0,
        ok_gen: u32::MAX,
        status: String::new(),
        started: Instant::now(),
        down: held,
        best: [0; 8],
        sound_done: false,
        anim_at: Instant::now(),
        rep_at: Instant::now(),
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

fn beeps_for(s: Snd) -> &'static [sound::Note] {
    match s {
        Snd::Click => sound::CLICK,
        Snd::Coin => sound::COIN,
        Snd::Crash => sound::CRASH,
        Snd::Tick => sound::TICK,
    }
}

fn screen_code(s: &Screen) -> u32 {
    match s {
        Screen::Boot(_) => 0,
        Screen::Home => 1,
        Screen::Menu(..) => 2,
        Screen::Dice(..) => 3,
        Screen::Show { .. } => 4,
        Screen::Countdown { .. } => 5,
        Screen::PwGen { .. } => 6,
        Screen::Stopwatch { .. } => 7,
        Screen::Ball(_) => 8,
        Screen::Entropy(_) => 9,
        Screen::ButtonTest => 20,
        Screen::Diag => 21,
        Screen::About => 22,
        Screen::Game { idx, .. } => 10 + *idx as u32,
    }
}

fn fmt_time(d: Duration) -> String {
    let ms = d.as_millis() as u64;
    format!("{:02}:{:02}.{:02}", ms / 60000, (ms / 1000) % 60, (ms % 1000) / 10)
}

impl App {
    fn goto(&mut self, s: Screen) {
        crate::diag::SCREEN.store(screen_code(&s), std::sync::atomic::Ordering::Relaxed);
        self.screen = s;
        self.gen = self.gen.wrapping_add(1);
        self.dirty = true;
    }

    /// Something went wrong on this screen: go home, keep running.
    fn recover(&mut self) {
        log::error!("recovered from a panic, back to HOME");
        self.panics += 1;
        crate::diag::bump(&crate::diag::PANICS);
        self.status = "recovered".into();
        self.goto(Screen::Home);
    }

    fn start_game(&mut self, idx: usize) {
        let g = (GAMES[idx].make)(&self.trng);
        let tune: Option<&'static [sound::Note]> = match idx {
            0 => Some(sound::TUNE_DODGER),
            1 => Some(sound::TUNE_SNAKE),
            2 => Some(sound::TUNE_PONG),
            3 => Some(sound::TUNE_FLAPPY),
            6 => Some(sound::TUNE_DOOM),
            _ => None,
        };
        self.snd.tune(tune);
        self.goto(Screen::Game { idx, g });
    }

    // ------------------------------------------------------------------ input
    fn on_input(&mut self, inp: Input) {
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
            self.go_back();
            return;
        }
        let mut redraw = false;
        let mut click = false;
        let mut restart = None;
        let mut rep = false;
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
            Screen::Dice(p, _) => match b {
                Btn::Up => {
                    *p = (*p + DICE.len() - 1) % DICE.len();
                    redraw = true;
                    click = true;
                }
                Btn::Down => {
                    *p = (*p + 1) % DICE.len();
                    redraw = true;
                    click = true;
                }
                _ => {}
            },
            Screen::PwGen { prof, len, pw } => {
                let n = rng::PROFILES.len();
                match b {
                    Btn::Up => {
                        *prof = (*prof + n - 1) % n;
                        *len = rng::PROFILES[*prof].default_len;
                    }
                    Btn::Down => {
                        *prof = (*prof + 1) % n;
                        *len = rng::PROFILES[*prof].default_len;
                    }
                    Btn::Left => *len = (*len).saturating_sub(1).max(rng::LEN_MIN),
                    Btn::Right => *len = (*len + 1).min(rng::LEN_MAX),
                    _ => {}
                }
                if matches!(b, Btn::Up | Btn::Down | Btn::Left | Btn::Right) {
                    *pw = rng::generate(&self.trng, &rng::PROFILES[*prof], *len).unwrap_or_default();
                    redraw = true;
                    rep = true;
                }
            }
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
            _ => {}
        }
        if rep {
            self.rep_at = Instant::now() + Duration::from_millis(400);
        }
        if click {
            self.snd.play(sound::CLICK);
        }
        if let Some(idx) = restart {
            self.start_game(idx);
        } else if redraw {
            self.dirty = true;
        }
    }

    fn on_ok(&mut self, held_ms: u32) {
        let _ = held_ms;
        enum Act {
            None,
            Select(MenuId, usize),
            Roll(usize),
            Shake,
            PwOk(bool),
        }
        let act = match &self.screen {
            Screen::Menu(m, sel) => Act::Select(*m, *sel),
            Screen::Dice(p, _) => Act::Roll(*p),
            Screen::Ball(_) => Act::Shake,
            Screen::PwGen { .. } => Act::PwOk(held_ms >= 800),
            _ => Act::None,
        };
        match act {
            Act::None => {}
            Act::Select(m, sel) => self.select(m, sel),
            Act::Roll(p) => self.roll(p),
            Act::PwOk(type_it) => {
                let mut to_type: Option<String> = None;
                if let Screen::PwGen { prof, len, pw } = &mut self.screen {
                    if type_it {
                        to_type = Some(pw.clone());
                    } else {
                        *pw = rng::generate(&self.trng, &rng::PROFILES[*prof], *len).unwrap_or_default();
                    }
                }
                if let Some(text) = to_type {
                    self.snd.play(sound::COIN);
                    self.type_text(&text);
                    self.status = "typed site pw".into();
                } else {
                    self.snd.play(sound::CLICK);
                }
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

    fn go_back(&mut self) {
        let to = match &self.screen {
            Screen::Boot(_) => Screen::Home,
            Screen::Home => Screen::Menu(MenuId::Main, 0),
            Screen::Menu(MenuId::Main, _) => Screen::Home,
            Screen::Menu(MenuId::Passwords, _) => Screen::Menu(MenuId::Main, 0),
            Screen::Menu(MenuId::Games, _) => Screen::Menu(MenuId::Main, 2),
            Screen::Menu(MenuId::Tools, _) => Screen::Menu(MenuId::Main, 3),
            Screen::Dice(..) => Screen::Menu(MenuId::Main, 1),
            Screen::Show { back, .. } => Screen::Menu(*back, 0),
            Screen::Countdown { .. } | Screen::PwGen { .. } => Screen::Menu(MenuId::Passwords, 0),
            Screen::Game { idx, .. } => Screen::Menu(MenuId::Games, *idx),
            Screen::Stopwatch { .. } => Screen::Menu(MenuId::Tools, 0),
            Screen::Ball(_) => Screen::Menu(MenuId::Tools, 1),
            Screen::Entropy(_) => Screen::Menu(MenuId::Tools, 2),
            Screen::ButtonTest => Screen::Menu(MenuId::Tools, 3),
            Screen::Diag => Screen::Menu(MenuId::Tools, 7),
            Screen::About => Screen::Menu(MenuId::Main, 4),
        };
        self.snd.silence();
        self.goto(to);
    }

    // ------------------------------------------------------------------ actions
    fn type_text(&mut self, text: &str) {
        // errors (no USB host connected) are ignored on purpose
        self.usb.send_str(&crate::typing::as_us_keyboard(text)).ok();
    }

    /// HOME + OK: type a secret straight away (short = password, long = 6 words).
    fn quick_type(&mut self, words: bool) {
        let secret = if words { rng::passphrase(&self.trng) } else { rng::password(&self.trng) };
        if let Some(text) = secret {
            self.type_text(&text);
            self.status = if words { "typed 6 words 66b".into() } else { "typed 20 chars 119b".into() };
            self.snd.play(sound::COIN);
        }
        self.dirty = true;
    }

    fn select(&mut self, m: MenuId, sel: usize) {
        self.snd.play(sound::CLICK);
        match (m, sel) {
            (MenuId::Main, 0) => self.goto(Screen::Menu(MenuId::Passwords, 0)),
            (MenuId::Main, 1) => self.goto(Screen::Dice(0, String::new())),
            (MenuId::Main, 2) => self.goto(Screen::Menu(MenuId::Games, 0)),
            (MenuId::Main, 3) => self.goto(Screen::Menu(MenuId::Tools, 0)),
            (MenuId::Main, 4) => self.goto(Screen::About),
            (MenuId::Passwords, 0) => {
                let (prof, len) = (0, rng::PROFILES[0].default_len);
                let pw = rng::generate(&self.trng, &rng::PROFILES[prof], len).unwrap_or_default();
                self.goto(Screen::PwGen { prof, len, pw });
            }
            (MenuId::Passwords, 1) => self.start_countdown(true),
            (MenuId::Passwords, 2) => {
                let phrase = rng::passphrase(&self.trng).unwrap_or_default();
                let parts: Vec<&str> = phrase.split('-').collect();
                let mut lines: Vec<String> = parts.chunks(2).map(|c| c.join("-")).collect();
                lines.push("6 words, 66 bits".into());
                self.goto(Screen::Show { title: "6 words".into(), lines, back: MenuId::Passwords });
            }
            (MenuId::Passwords, 3) => {
                let mut pin = String::new();
                for _ in 0..6 {
                    pin.push((b'0' + rng::below(&self.trng, 10).unwrap_or(0) as u8) as char);
                }
                self.goto(Screen::Show {
                    title: "PIN".into(),
                    lines: vec![pin, "".into(), "6 digits, 20 bits".into()],
                    back: MenuId::Passwords,
                });
            }
            (MenuId::Games, i) if i < GAMES.len() => self.start_game(i),
            (MenuId::Tools, 0) => {
                self.goto(Screen::Stopwatch { running: false, started: Instant::now(), acc: Duration::ZERO })
            }
            (MenuId::Tools, 1) => self.goto(Screen::Ball(None)),
            (MenuId::Tools, 2) => self.goto(Screen::Entropy(Entropy { total: 0, ones: 0, bins: [0; 16] })),
            (MenuId::Tools, 3) => self.goto(Screen::ButtonTest),
            (MenuId::Tools, 4) => {
                self.snd.play(sound::SCALE);
                let st = self.snd.status_text();
                self.goto(Screen::Show {
                    title: "Buzzer test".into(),
                    lines: vec!["scale from C#4".into(), "buzzer + on pin 32".into(), "(PB3), - on GND".into(), st],
                    back: MenuId::Tools,
                });
            }
            (MenuId::Tools, 5) => self.snd.play(sound::SOS),
            (MenuId::Tools, 6) => {
                let on = self.snd.toggle();
                self.status = if on { "sound on".into() } else { "sound off".into() };
                self.goto(Screen::Show {
                    title: "Sound".into(),
                    lines: vec![if on { "sound is ON".into() } else { "sound is OFF".into() }, "BACK = menu".into()],
                    back: MenuId::Tools,
                });
            }
            (MenuId::Tools, 7) => self.goto(Screen::Diag),
            (MenuId::Tools, 8) => {
                self.goto(Screen::Show {
                    title: "Bootloader".into(),
                    lines: vec![
                        "HOLD PROG now!".into(),
                        "reset in 5 s".into(),
                        "release when flasher".into(),
                        "says so".into(),
                    ],
                    back: MenuId::Tools,
                });
                self.draw();
                self.dirty = false;
                crate::bootmode::enter_bootloader();
            }
            _ => {}
        }
    }

    /// Menu -> "Type ...": give the user 3 seconds to click the target field, then type. Never blocks;
    /// BACK cancels.
    fn start_countdown(&mut self, words: bool) {
        self.goto(Screen::Countdown { words, until: Instant::now() + Duration::from_secs(3) });
    }

    fn roll(&mut self, p: usize) {
        let (n, sides, label) = DICE[p];
        let mut total = 0;
        let mut parts: Vec<String> = Vec::new();
        for _ in 0..n {
            let r = rng::below(&self.trng, sides).unwrap_or(0) + 1;
            total += r;
            parts.push(r.to_string());
        }
        let text = if label == "coin" {
            if total == 1 { "heads".to_string() } else { "tails".to_string() }
        } else if n > 1 {
            format!("{}+{}={}", parts[0], parts[1], total)
        } else {
            total.to_string()
        };
        self.snd.play(sound::CLICK);
        self.screen = Screen::Dice(p, text); // same screen, keep `gen`
        self.dirty = true;
    }

    // ------------------------------------------------------------------ timers
    fn tick(&mut self) {
        let now = Instant::now();
        let mut finish_countdown = None;
        let mut to_home = false;
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
                if now.duration_since(self.anim_at) >= Duration::from_millis(250) {
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
            Screen::PwGen { prof, len, pw } => {
                // holding LEFT / RIGHT keeps changing the length
                let (l, r) = (self.down[bidx(Btn::Left)], self.down[bidx(Btn::Right)]);
                if l != r && now >= self.rep_at {
                    *len = if l { (*len).saturating_sub(1).max(rng::LEN_MIN) } else { (*len + 1).min(rng::LEN_MAX) };
                    *pw = rng::generate(&self.trng, &rng::PROFILES[*prof], *len).unwrap_or_default();
                    self.rep_at = now + Duration::from_millis(90);
                    self.dirty = true;
                }
            }
            Screen::Stopwatch { running, .. } => {
                if *running {
                    self.dirty = true;
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
            Screen::Countdown { words, until } => {
                if now >= *until {
                    finish_countdown = Some(*words);
                } else {
                    self.dirty = true; // redraw the seconds
                }
            }
            Screen::About | Screen::Diag => {
                let secs = self.started.elapsed().as_secs();
                if secs != self.last_secs {
                    self.last_secs = secs;
                    self.dirty = true;
                }
            }
            _ => {}
        }
        if to_home {
            self.goto(Screen::Home);
        }
        if let Some(words) = finish_countdown {
            let secret = if words { rng::passphrase(&self.trng) } else { rng::password(&self.trng) };
            let msg = match secret {
                Some(text) => {
                    self.type_text(&text);
                    if words { "typed 6 words" } else { "typed 20 chars" }
                }
                None => "random number error",
            };
            self.goto(Screen::Show {
                title: "Done".into(),
                lines: vec![msg.to_string(), "BACK = menu".into()],
                back: MenuId::Passwords,
            });
        }
    }

    // ------------------------------------------------------------------ drawing
    fn draw(&mut self) {
        let Some(disp) = self.display.clone() else { return };
        let oled = &mut self.canvas;
        oled.clear();
        match &self.screen {
            Screen::Boot(t0) => logo::boot_frame(oled, t0.elapsed().as_millis() as u32),
            Screen::Home => logo::home_frame(oled, self.started.elapsed().as_millis() as u32),
            Screen::Menu(m, sel) => {
                oled.text(0, 0, title(*m));
                let list = items(*m);
                let start = if *sel >= 4 { *sel - 3 } else { 0 };
                for k in 0..4 {
                    let idx = start + k;
                    if idx < list.len() {
                        let y = 13 + 13 * k;
                        oled.text(6, y, list[idx]);
                        if idx == *sel {
                            oled.invert_rect(0, y - 1, 128, 13);
                        }
                    }
                }
                // scroll hint on the right edge
                if list.len() > 4 {
                    let h = 48 * 4 / list.len();
                    let y = 13 + (48 - h) * *sel / (list.len() - 1);
                    oled.fill_rect(126, y, 2, h, true);
                }
            }
            Screen::Dice(p, text) => {
                oled.text(0, 0, &format!("Dice: {}  (UP/DOWN)", DICE[*p].2));
                oled.text(0, 13, "OK rolls, BACK exits");
                if !text.is_empty() {
                    // scale 3 is 18 px per character: 7 characters fit, otherwise use scale 2
                    oled.text_big(0, 28, text, if text.len() <= 7 { 3 } else { 2 });
                }
            }
            Screen::Show { title, lines, .. } => {
                oled.text(0, 0, title);
                for (i, l) in lines.iter().take(4).enumerate() {
                    oled.text(0, 13 + 13 * i, l);
                }
            }
            Screen::Countdown { words, until } => {
                let left = until.saturating_duration_since(Instant::now()).as_millis() as u64 / 1000 + 1;
                oled.text(0, 0, if *words { "Typing 6 words in" } else { "Typing password in" });
                oled.text_big(52, 16, &left.to_string(), 3);
                oled.text(0, 52, "click target field");
            }
            Screen::PwGen { prof, len, pw } => {
                let p = &rng::PROFILES[*prof];
                oled.text(0, 0, p.name);
                oled.text(0, 13, &format!("len {} ~{} bits", len, rng::bits(p, *len)));
                for (i, chunk) in pw.as_bytes().chunks(21).enumerate().take(3) {
                    oled.text(0, 26 + 13 * i, &String::from_utf8_lossy(chunk));
                }
            }
            Screen::Game { idx, g } => g.draw(oled, self.best[*idx]),
            Screen::Stopwatch { running, started, acc } => {
                let t = *acc + if *running { started.elapsed() } else { Duration::ZERO };
                oled.text(0, 0, "Stopwatch");
                oled.text_big(4, 18, &fmt_time(t), 2);
                oled.text(0, 52, "OK start/stop  UP reset");
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
                        // split long answers over two lines at a space
                        if t.len() > 10 {
                            let cut = t[..t.len() / 2 + 3].rfind(' ').unwrap_or(t.len() / 2);
                            oled.text_big(0, 16, &t[..cut], 2);
                            oled.text_big(0, 36, t[cut..].trim_start(), 2);
                        } else {
                            oled.text_big(0, 22, t, 2);
                        }
                    }
                }
            }
            Screen::Entropy(e) => {
                oled.text(0, 0, "TRNG live test");
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
                oled.text(0, 0, "ChocoBao v0.6");
                oled.text(0, 13, "Baochip-1x + Xous");
                oled.text(0, 26, "hardware TRNG");
                oled.text(0, 39, &format!("up {}m {}s", secs / 60, secs % 60));
                oled.text(0, 52, if self.snd.enabled.load(Ordering::Relaxed) { "sound on" } else { "sound off" });
            }
        }
        disp.submit(&self.canvas);
    }
}
