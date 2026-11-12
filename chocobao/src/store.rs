// store.rs - keeps your settings, high scores and which help screens you have seen in the chip's ReRAM, so they
// survive power-off and re-flashing.
//
// How: the Xous `keystore` service (the one process allowed to write the ReRAM) offers "application key slots":
// 32-byte slots in a data area made for apps. We never touch the ReRAM controller ourselves (a first attempt that
// mapped it directly crashed the whole app at boot).
//
// Layout: 40 records of 3 slots (96 bytes) form a ring in slots 0..120. A save writes the NEXT record, so each slot
// is rewritten once per 40 saves (ReRAM wears out after roughly 10,000 writes per cell: about 400,000 saves).
// A record is valid when its magic and CRC match; the highest sequence number wins at start-up, so a record torn by a
// power cut is ignored. All keystore calls happen on a worker thread: if the service ever hangs, only saving stops.
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

const REC: usize = 96;
const RECORDS: usize = 40;
const MAGIC: u16 = 0xC0B0;
const VERSION: u8 = 1;

#[derive(Clone, PartialEq)]
pub struct Saved {
    pub idle_idx: u8,
    pub saver: u8,
    pub sound: bool,
    pub doom: u8,
    pub maze: u8,
    pub ninja: u8,
    pub best: [u16; 32],
    pub help: [u8; 8], // one bit per first-open help screen
}

impl Default for Saved {
    fn default() -> Self { Saved { idle_idx: 2, saver: 1, sound: true, doom: 0, maze: 0, ninja: 0, best: [0; 32], help: [0; 8] } }
}

pub struct Store {
    tx: Sender<Saved>,
    pub last: Saved,
}

fn encode(s: &Saved, seq: u32) -> [u8; REC] {
    let mut r = [0u8; REC];
    r[0..2].copy_from_slice(&MAGIC.to_le_bytes());
    r[2] = VERSION;
    r[4..8].copy_from_slice(&seq.to_le_bytes());
    r[8] = s.idle_idx;
    r[9] = s.saver;
    r[10] = s.sound as u8;
    r[11] = s.doom;
    r[12] = s.maze;
    r[13] = s.ninja;
    for (i, b) in s.best.iter().enumerate() {
        r[16 + i * 2..18 + i * 2].copy_from_slice(&b.to_le_bytes());
    }
    r[80..88].copy_from_slice(&s.help);
    let crc = crc32fast::hash(&r[..92]);
    r[92..96].copy_from_slice(&crc.to_le_bytes());
    r
}

fn decode(r: &[u8]) -> Option<(u32, Saved)> {
    if r.len() < REC || u16::from_le_bytes([r[0], r[1]]) != MAGIC || r[2] != VERSION {
        return None;
    }
    let crc = u32::from_le_bytes([r[92], r[93], r[94], r[95]]);
    if crc != crc32fast::hash(&r[..92]) {
        return None;
    }
    let mut s = Saved { idle_idx: r[8], saver: r[9], sound: r[10] != 0, doom: r[11], maze: r[12], ninja: r[13], best: [0; 32], help: [0; 8] };
    for i in 0..32 {
        s.best[i] = u16::from_le_bytes([r[16 + i * 2], r[17 + i * 2]]);
    }
    s.help.copy_from_slice(&r[80..88]);
    Some((u32::from_le_bytes([r[4], r[5], r[6], r[7]]), s))
}

fn read_record(ks: &keystore::Keystore, rec: usize) -> Option<[u8; REC]> {
    let mut out = [0u8; REC];
    for part in 0..3 {
        let slot = ks.read_app_key(rec * 3 + part).ok()?;
        out[part * 32..part * 32 + 32].copy_from_slice(&slot);
    }
    Some(out)
}

fn worker(rx: std::sync::mpsc::Receiver<Saved>, ready: Sender<Option<Saved>>) {
    let xns = match xous_names::XousNames::new() {
        Ok(x) => x,
        Err(_) => return,
    };
    let ks = keystore::Keystore::new(&xns);
    // find the newest valid record; stop after two invalid ones in a row (the rest was never written)
    let mut best: Option<(u32, usize, Saved)> = None;
    let mut bad = 0;
    for rec in 0..RECORDS {
        match read_record(&ks, rec).and_then(|b| decode(&b)) {
            Some((seq, s)) => {
                bad = 0;
                if best.as_ref().map_or(true, |b| seq.wrapping_sub(b.0) < 0x8000_0000 && seq != b.0) {
                    best = Some((seq, rec, s));
                }
            }
            None => {
                bad += 1;
                if bad >= 2 {
                    break;
                }
            }
        }
    }
    let (mut seq, mut next, saved) = match best {
        Some((seq, rec, s)) => (seq, (rec + 1) % RECORDS, Some(s)),
        None => (0, 0, None),
    };
    log::info!("store: loaded {}", saved.is_some());
    ready.send(saved).ok();
    // from now on: write whatever the app sends us
    while let Ok(mut s) = rx.recv() {
        while let Ok(newer) = rx.try_recv() {
            s = newer; // several changes in a row cost one write
        }
        seq = seq.wrapping_add(1);
        let rec = encode(&s, seq);
        let mut ok = true;
        for part in 0..3 {
            let mut slot = [0u8; 32];
            slot.copy_from_slice(&rec[part * 32..part * 32 + 32]);
            if ks.write_app_key(next * 3 + part, &slot).is_err() {
                ok = false;
                break;
            }
        }
        log::info!("store: saved record {} ok={}", next, ok);
        next = (next + 1) % RECORDS;
    }
}

impl Store {
    /// Start the worker and read the newest saved settings. None if the keystore does not answer within 4 s.
    pub fn open() -> Option<(Store, Option<Saved>)> {
        let (tx, rx) = channel::<Saved>();
        let (ready_tx, ready_rx) = channel();
        std::thread::Builder::new().name("store".into()).spawn(move || worker(rx, ready_tx)).ok()?;
        let saved = ready_rx.recv_timeout(Duration::from_secs(4)).ok()?;
        let last = saved.clone().unwrap_or_default();
        Some((Store { tx, last }, saved))
    }

    /// Queue the settings for writing (done on the worker thread). Returns false if the worker is gone.
    pub fn save(&mut self, s: &Saved) -> bool {
        let ok = self.tx.send(s.clone()).is_ok();
        self.last = s.clone();
        ok
    }
}
