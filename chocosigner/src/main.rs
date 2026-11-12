// chocosigner - the Notary's signing key lives in THIS process and never leaves it.
//
// It is a separate Xous process in the system image, with its own address space. The ChocoBao app (screen, buttons,
// games, Notary screen) talks to it through messages: "sign these bytes" returns a signature and the public key. There is
// no message that returns the key. The key is generated here from the hardware random-number generator and kept on a
// page placed high in the address space, so another process cannot even name it, and the kernel's MMU faults any process
// that tries.
//
// Messages are memory lends (the caller lends a 4 KiB page, this process fills in the answer):
//   id 1  SIGN: [0..2] length (LE u16), [2..2+len] message  ->  [0..64] signature, [64..96] public key, [96] 1 = ok
//   id 2  INFO:                                             ->  [0..32] public key, [32..36] virtual address of the key page (LE u32)
use ed25519_dalek::{Signer, SigningKey, Verifier};

fn make_key_page() -> *mut u8 {
    // reserve pages first and free them afterwards: the key page then sits above everything another process has mapped
    let mut spacers = Vec::new();
    for _ in 0..48 {
        if let Ok(r) = xous::map_memory(None, None, 4096, xous::MemoryFlags::R | xous::MemoryFlags::W) {
            spacers.push(r);
        }
    }
    let page = xous::map_memory(None, None, 4096, xous::MemoryFlags::R | xous::MemoryFlags::W).expect("key page");
    for r in spacers {
        let _ = xous::unmap_memory(r);
    }
    let p = page.as_mut_ptr();
    std::mem::forget(page);
    p
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name("_ChocoSigner_", Some(8)).expect("register signer");
    let trng = bao1x_hal_service::trng::Trng::new(&xns).unwrap();
    // the seed comes from the hardware TRNG and goes straight onto the key page
    let page = make_key_page();
    for i in 0..8 {
        let x = trng.get_u32().unwrap_or(0).to_le_bytes();
        for (k, b) in x.iter().enumerate() {
            // safety: the page is 4096 bytes and exclusively ours
            unsafe { page.add(i * 4 + k).write_volatile(*b) };
        }
    }
    let read_seed = || {
        let mut s = [0u8; 32];
        for (i, b) in s.iter_mut().enumerate() {
            // safety: as above
            *b = unsafe { page.add(i).read_volatile() };
        }
        s
    };
    let public = SigningKey::from_bytes(&read_seed()).verifying_key().to_bytes();
    log::info!("signer up, pid {}, key page at {:x}", xous::process::id(), page as usize);
    let mut count = 0u32;
    loop {
        let mut msg = xous::receive_message(sid).unwrap();
        let id = msg.body.id();
        if let Some(mm) = msg.body.memory_message_mut() {
            // safety: the caller lent a page of at least 4096 bytes
            let buf = unsafe { mm.buf.as_slice_mut::<u8>() };
            if buf.len() < 128 {
                continue;
            }
            match id {
                1 => {
                    let len = u16::from_le_bytes([buf[0], buf[1]]) as usize;
                    if len > buf.len().saturating_sub(2) || len > 1024 {
                        buf[96] = 0;
                        continue;
                    }
                    let sk = SigningKey::from_bytes(&read_seed());
                    let text = buf[2..2 + len].to_vec();
                    let sig = sk.sign(&text);
                    let ok = sk.verifying_key().verify(&text, &sig).is_ok();
                    buf[0..64].copy_from_slice(&sig.to_bytes());
                    buf[64..96].copy_from_slice(&public);
                    buf[96] = ok as u8;
                    count += 1;
                    log::info!("signer: signed #{} ({} bytes)", count, len);
                }
                2 => {
                    buf[0..32].copy_from_slice(&public);
                    buf[32..36].copy_from_slice(&(page as usize as u32).to_le_bytes());
                }
                _ => {}
            }
        }
    }
}
