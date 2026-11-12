// vault.rs - the Notary's signing seed lives on its own page of memory that only THIS process has mapped.
//
// The page is placed high in the address space (after a block of throw-away pages), so its virtual address is one that
// no other process has mapped. The "MMU demo" tells the sandbox process that address and lets it try to read it: the
// kernel kills the sandbox, and the key stays intact (the Notary signs again with the same public key).
use std::sync::atomic::{AtomicUsize, Ordering};

use bao1x_hal_service::trng::Trng;

static PAGE: AtomicUsize = AtomicUsize::new(0);

/// Virtual address of the key page (0 until it has been made).
pub fn page_addr() -> usize { PAGE.load(Ordering::Relaxed) }

/// Make the key page the first time (random seed from the hardware TRNG) and return its address.
pub fn ensure(trng: &Trng) -> usize {
    let cur = PAGE.load(Ordering::Relaxed);
    if cur != 0 {
        return cur;
    }
    // reserve a run of pages first and free them afterwards: the key page then sits above everything the sandbox has
    let mut spacers = Vec::new();
    for _ in 0..48 {
        if let Ok(r) = xous::map_memory(None, None, 4096, xous::MemoryFlags::R | xous::MemoryFlags::W) {
            spacers.push(r);
        }
    }
    let key_page = xous::map_memory(None, None, 4096, xous::MemoryFlags::R | xous::MemoryFlags::W).expect("key page");
    for r in spacers {
        let _ = xous::unmap_memory(r);
    }
    let base = key_page.as_mut_ptr();
    for i in 0..8 {
        let x = trng.get_u32().unwrap_or(0).to_le_bytes();
        for (k, b) in x.iter().enumerate() {
            // safety: the page is 4096 bytes and exclusively ours
            unsafe { base.add(i * 4 + k).write_volatile(*b) };
        }
    }
    PAGE.store(base as usize, Ordering::Relaxed);
    std::mem::forget(key_page); // keep it mapped for the life of the app
    base as usize
}

/// The signing seed (copied out of the key page; the copy never leaves the Notary code).
pub fn seed(trng: &Trng) -> [u8; 32] {
    let p = ensure(trng) as *const u8;
    let mut s = [0u8; 32];
    for (i, b) in s.iter_mut().enumerate() {
        // safety: the page is mapped and at least 32 bytes long
        *b = unsafe { p.add(i).read_volatile() };
    }
    s
}
