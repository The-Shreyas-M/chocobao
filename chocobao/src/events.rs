// events.rs - an I/O (BIO) core timestamps pulses on the EVENT pin (PC8 = BIO24) and the CPU only reads the results.
//
// The BIO core runs a tiny program: once per microsecond it looks at the pin; on a rising edge it reads its own free-running
// clock counter and pushes the 30-bit count to a FIFO. The CPU is not involved, so the timestamps do not depend on what
// the CPU is doing (drawing, USB, music).
//
// A sensor node (the ESP32, tools/esp32/sensor_node.py) sends one 8-bit value as a burst of 10 pulses:
//   edge 0 = start, edge 1 = one reference gap T after it, then 8 data edges, each a gap of T (bit 0) or 2T (bit 1).
// Because the reference gap travels with every frame, the decoder needs no knowledge of the clock speed.
// Each decoded frame becomes an `Event { value, ticks }`; the Notary signs it.
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use arbitrary_int::{u5, Number};
use bao1x_api::bio::*;
use bao1x_api::bio_code;
use bao1x_api::bio_resources::*;
use bao1x_api::{IoSetup, IoxDir, IoxEnable, IoxHal};
use bao1x_hal::bio::{Bio, CoreCsr};
use utralib::utra::bio_bdma;

const EVENT_BIT: u8 = 24; // PC8
const COUNT_MASK: u32 = 0x3FFF_FFFF;
const MARK: u32 = 0x4000_0000; // set on every timestamp so an empty FIFO read is recognised

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub value: u8,
    pub ticks: u32, // BIO clock count (30 bits) at the first edge of the frame
    pub bit_gap: u32, // reference gap in BIO clocks: the length of T on the sensor's own clock
}

#[rustfmt::skip]
bio_code!(edge_kernel, EDGE_START, EDGE_END,
    "li x5, 0x01000000",       // the EVENT pin (BIO24)
    "mv x26, x5",              // GPIO actions apply to that pin only
    "mv x25, x5",              // make it an input
    "mv x8, x0",               // previous level
    "mv x10, x0",              // quantum counter
"10:",
    "mv x20, x0",              // wait for the next quantum (1 MHz clock): we look at the pin once per microsecond
    "addi x10, x10, 1",
    "li x11, 0x3FFF",
    "and x12, x10, x11",
    "bne x12, x0, 15f",        // every 16384 quanta (16 ms) send a heartbeat: proves the core runs and shows the pin level
    "srli x13, x10, 14",
    "andi x13, x13, 0xFF",
    "slli x13, x13, 16",
    "srli x14, x21, 16",
    "li x15, 0xFFFF",
    "and x14, x14, x15",
    "or x13, x13, x14",
    "li x15, 0x80000000",
    "or x13, x13, x15",
    "mv x16, x13",
"15:",
    "mv x9, x21",              // read all pins
    "and x9, x9, x5",          // keep our pin
    "beq x9, x8, 10b",         // nothing changed
    "mv x8, x9",
    "beq x9, x0, 10b",         // it fell: not interesting, wait for the rising edge
    "li x7, 0x3FFFFFFF",
    "and x6, x31, x7",         // our clock counter, core id masked off
    "li x7, 0x40000000",
    "or x6, x6, x7",           // marker bit
    "mv x16, x6",              // timestamp into FIFO0
    "j 10b"
);

static RX: OnceLock<Mutex<Receiver<Event>>> = OnceLock::new();

/// The next decoded event, if any (never blocks).
pub fn poll() -> Option<Event> { RX.get().and_then(|m| m.lock().ok()).and_then(|r| r.try_recv().ok()) }

pub fn start() {
    let (tx, rx) = channel();
    let _ = RX.set(Mutex::new(rx));
    std::thread::Builder::new().name("events".into()).spawn(move || run(tx)).ok();
}

struct Capture {
    _bio: Bio,
    _handle: CoreHandle,
    fifo: CoreCsr,
}

fn init() -> Result<Capture, String> {
    let pin = u5::new(EVENT_BIT);
    let (port, pin_no) = bio_bit_to_port_and_pin(pin);
    // the pin is a plain input (schmitt trigger, pull-up so it never floats); the ESP32 drives it
    let iox = IoxHal::new();
    iox.setup_pin(port, pin_no, Some(IoxDir::Input), Some(bao1x_api::IoxFunction::Gpio), Some(IoxEnable::Enable), Some(IoxEnable::Enable), None, None);
    let mut bio = Bio::new();
    let spec = ResourceSpec {
        claimer: "ChocoEvents".to_string(),
        cores: vec![CoreRequirement::Any],
        fifos: vec![Fifo::Fifo0],
        static_pins: vec![],
        dynamic_pin_count: 1,
    };
    let grant = bio.claim_resources(&spec).map_err(|e| format!("claim {:?}", e))?;
    let config = CoreConfig { clock_mode: ClockMode::TargetFreqInt(1_000_000) };
    bio.init_core(grant.cores[0], edge_kernel(), config).map_err(|e| format!("init {:?}", e))?;
    bio.claim_dynamic_pin(EVENT_BIT, &spec.claimer).map_err(|e| format!("pin {:?}", e))?;
    let mut io = IoConfig::default();
    io.mapped = 1 << pin.as_u32();
    io.mode = IoConfigMode::SetOnly; // add our pins; never wipe the other core's mapping
    bio.setup_io_config(io).map_err(|e| format!("io {:?}", e))?;
    bio.set_core_run_state(&grant, true);
    // safety: the handle is stored next to the CSR view so it outlives it
    let handle = unsafe { bio.get_core_handle(Fifo::Fifo0) }
        .map_err(|e| format!("fifo {:?}", e))?
        .ok_or_else(|| "no FIFO0 handle".to_string())?;
    let fifo = CoreCsr::from_handle(&handle);
    Ok(Capture { _bio: bio, _handle: handle, fifo })
}

fn decode(edges: &[u32]) -> Option<Event> {
    // 10 edges: start, reference, 8 data
    if edges.len() < 10 {
        return None;
    }
    let gap = |i: usize| edges[i + 1].wrapping_sub(edges[i]) & COUNT_MASK;
    let reference = gap(0);
    if reference == 0 {
        return None;
    }
    let mut value = 0u8;
    for i in 1..9 {
        let g = gap(i) as u64;
        // a bit is 0 when the gap is about T and 1 when it is about 2T; anything outside 0.5T..2.6T is noise
        if g * 2 < reference as u64 || g * 10 > reference as u64 * 26 {
            return None;
        }
        value = (value << 1) | if g * 2 > reference as u64 * 3 { 1 } else { 0 };
    }
    Some(Event { value, ticks: edges[0], bit_gap: reference })
}

fn run(tx: Sender<Event>) {
    let tt = ticktimer::Ticktimer::new().unwrap();
    tt.sleep_ms(4500).ok(); // after the rest of the system (and the buzzer core) has started
    let cap = match init() {
        Ok(c) => c,
        Err(e) => {
            log::error!("events: BIO capture failed: {}", e);
            if let Ok(mut m) = crate::diag::EV_MSG.lock() {
                *m = format!("FAIL:{}", e);
            }
            return;
        }
    };
    log::info!("events: BIO capture running on PC8");
    if let Ok(mut m) = crate::diag::EV_MSG.lock() {
        *m = "running".to_string();
    }
    let iox_probe = IoxHal::new();
    let mut probe_at = Instant::now();
    let mut last_raw = 0u32;
    let mut edges: Vec<u32> = Vec::new();
    let mut last_edge = Instant::now();
    loop {
        // drain what the BIO core has queued (each read pops one entry; an empty FIFO returns a value without the marker)
        for _ in 0..8 {
            let raw = cap.fifo.csr.r(bio_bdma::SFR_RXF0);
            if raw == last_raw || raw == 0 {
                break;
            }
            last_raw = raw;
            if raw & 0x8000_0000 != 0 {
                // heartbeat: low 16 bits = the pins as the BIO core sees them (bit 8 = EVENT)
                crate::diag::EV_HB.store(raw & 0xFFFF, Ordering::Relaxed);
                crate::diag::EV_HBN.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            if raw & MARK == 0 {
                continue;
            }
            edges.push(raw & COUNT_MASK);
            last_edge = Instant::now();
            crate::diag::EV_EDGES.fetch_add(1, Ordering::Relaxed);
        }
        // a frame is over when no edge has arrived for 25 ms
        if !edges.is_empty() && last_edge.elapsed() > Duration::from_millis(25) {
            match decode(&edges) {
                Some(ev) => {
                    crate::diag::EV_FRAMES.fetch_add(1, Ordering::Relaxed);
                    crate::diag::EV_LAST.store(ev.value as u32, Ordering::Relaxed);
                    crate::diag::EV_GAP.store(ev.bit_gap, Ordering::Relaxed);
                    tx.send(ev).ok();
                }
                None => {
                    crate::diag::EV_BAD.fetch_add(1, Ordering::Relaxed);
                }
            }
            edges.clear();
        }
        if probe_at.elapsed() > Duration::from_millis(200) {
            probe_at = Instant::now();
            let (port, pin_no) = bio_bit_to_port_and_pin(u5::new(EVENT_BIT));
            let level = (iox_probe.get_gpio_bank_value(port) >> pin_no) & 1;
            crate::diag::EV_PC8.store(level, Ordering::Relaxed);
        }
        tt.sleep_ms(1).ok();
    }
}
