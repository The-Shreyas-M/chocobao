// signer.rs - client of the chocosigner process. The signing key is NOT in this process: we can only ask for signatures.
use std::sync::{Mutex, OnceLock};

struct Client {
    cid: xous::CID,
    page: xous::MemoryRange, // a page we lend to the signer for each request
}
// safety: the page is only touched while the global lock is held
unsafe impl Send for Client {}

static CLIENT: OnceLock<Mutex<Option<Client>>> = OnceLock::new();

fn connect() -> Option<Client> {
    let xns = xous_names::XousNames::new().ok()?;
    let cid = xns.request_connection("_ChocoSigner_").ok()?;
    let page = xous::map_memory(None, None, 4096, xous::MemoryFlags::R | xous::MemoryFlags::W).ok()?;
    Some(Client { cid, page })
}

fn with_client<R>(f: impl FnOnce(&mut Client) -> Option<R>) -> Option<R> {
    let m = CLIENT.get_or_init(|| Mutex::new(None));
    let mut g = m.lock().ok()?;
    if g.is_none() {
        *g = connect();
    }
    let c = g.as_mut()?;
    f(c)
}

fn call(c: &mut Client, id: usize) -> bool {
    xous::send_message(c.cid, xous::Message::new_lend_mut(id, c.page, None, None)).is_ok()
}

/// Ask the signer to sign `text`. Returns (signature, public key), or None if the signer does not answer.
pub fn sign(text: &[u8]) -> Option<([u8; 64], [u8; 32])> {
    if text.len() > 1000 {
        return None;
    }
    with_client(|c| {
        // safety: the page is 4096 bytes and ours until we lend it
        let buf = unsafe { c.page.as_slice_mut::<u8>() };
        buf[0..2].copy_from_slice(&(text.len() as u16).to_le_bytes());
        buf[2..2 + text.len()].copy_from_slice(text);
        buf[96] = 0;
        if !call(c, 1) {
            return None;
        }
        let buf = unsafe { c.page.as_slice::<u8>() };
        if buf[96] != 1 {
            return None;
        }
        let mut sig = [0u8; 64];
        let mut pubk = [0u8; 32];
        sig.copy_from_slice(&buf[0..64]);
        pubk.copy_from_slice(&buf[64..96]);
        Some((sig, pubk))
    })
}

/// The signer's public key and the virtual address of its key page (for the MMU demo).
pub fn info() -> Option<([u8; 32], u32)> {
    with_client(|c| {
        if !call(c, 2) {
            return None;
        }
        let buf = unsafe { c.page.as_slice::<u8>() };
        let mut pubk = [0u8; 32];
        pubk.copy_from_slice(&buf[0..32]);
        Some((pubk, u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]])))
    })
}
