// store.rs - keeps your settings, high scores and which help screens you have seen in the chip's ReRAM, so they
// survive power-off and even re-flashing the app.
//
// Where: the last 4 KiB page of the app region (offset 0x3D_9000 in the ReRAM array, just below the 0x3D_A000 limit
// where "normal" writes stop). The app image is built to stay below that page (tools/build-chocobao.ps1 checks it),
// and the flasher only writes the image's own blocks, so this page keeps its content when you flash a new app.
//
// How: the page is a log of 42 records of 96 bytes. A save writes the NEXT record (round robin), so each ReRAM cell is
// rewritten only once per 42 saves (ReRAM wears out after roughly 10,000 writes per cell: about 400,000 saves).
// The newest valid record (highest sequence number, matching CRC) wins at start-up. A torn or garbage record is ignored.
use std::pin::Pin;

use bao1x_hal::rram::Reram;

const PAGE: usize = 0x3D_9000;
const REC: usize = 96;
const SLOTS: usize = 4096 / REC; // 42
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
    rram: Pin<Box<Reram>>,
    next: usize,
    seq: u32,
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

impl Store {
    /// Map the page and read the newest record. Returns None if the ReRAM page cannot be mapped (then nothing is saved).
    pub fn open(stage: u8) -> Option<(Store, Option<Saved>)> {
        log::info!("store: stage {} start", stage);
        let mut rram = Reram::new();
        log::info!("store: Reram::new ok");
        if stage < 2 {
            return None;
        }
        let range = xous::map_memory(
            xous::MemoryAddress::new(utralib::HW_RERAM_MEM + PAGE),
            None,
            4096,
            xous::MemoryFlags::R | xous::MemoryFlags::W,
        )
        .ok()?;
        rram.add_range(PAGE, range);
        log::info!("store: page mapped");
        if stage < 3 {
            return None;
        }
        let mut best: Option<(u32, usize, Saved)> = None;
        for slot in 0..SLOTS {
            let mut rec = [0u8; REC];
            let mut ok = true;
            for part in 0..3 {
                match rram.array(PAGE + slot * REC + part * 32) {
                    Ok(chunk) => rec[part * 32..part * 32 + 32].copy_from_slice(&chunk[..32]),
                    Err(_) => ok = false,
                }
            }
            if !ok {
                return None;
            }
            if let Some((seq, s)) = decode(&rec) {
                if best.as_ref().map_or(true, |b| seq.wrapping_sub(b.0) < 0x8000_0000 && seq != b.0) {
                    best = Some((seq, slot, s));
                }
            }
        }
        let (seq, next, saved, last) = match best {
            Some((seq, slot, s)) => (seq, (slot + 1) % SLOTS, Some(s.clone()), s),
            None => (0, 0, None, Saved::default()),
        };
        Some((Store { rram, next, seq, last }, saved))
    }

    /// Write the settings into the next record. Returns true if the ReRAM accepted and verified the write.
    pub fn save(&mut self, s: &Saved) -> bool {
        let seq = self.seq.wrapping_add(1);
        let rec = encode(s, seq);
        match self.rram.write_slice(PAGE + self.next * REC, &rec) {
            Ok(_) => {
                self.seq = seq;
                self.next = (self.next + 1) % SLOTS;
                self.last = s.clone();
                true
            }
            Err(e) => {
                log::error!("settings save failed: {:?}", e);
                false
            }
        }
    }
}
