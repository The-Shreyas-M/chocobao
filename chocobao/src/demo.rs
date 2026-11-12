// demo.rs - the MMU demo (also runnable from the serial console with `mmu`, handy for testing without the buttons).
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Set once the kernel has killed the sandbox; it cannot be restarted until the chip restarts.
static KILLED: AtomicBool = AtomicBool::new(false);

use bao1x_hal_service::trng::Trng;

/// The MMU demo. The main app owns a key page (see vault.rs). The sandbox, a second Xous process, is told the
/// page's virtual address and reads it. That address is valid here and unmapped there, so the kernel kills the sandbox.
pub fn mmu_intro(trng: &Trng) -> Vec<String> {
    let addr = crate::vault::ensure(trng);
    let seed = crate::vault::seed(trng);
    vec![
        "Key page of Notary".to_string(),
        format!("address {:08x}", addr),
        format!("main reads it: crc {:04x}", crc32fast::hash(&seed) & 0xFFFF),
        "OK = sandbox attacks".to_string(),
    ]
}

fn sandbox_ping(cid: xous::CID) -> bool {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = xous::send_message(cid, xous::Message::new_blocking_scalar(1, 0, 0, 0, 0));
        tx.send(r.is_ok()).ok();
    });
    rx.recv_timeout(Duration::from_millis(800)).unwrap_or(false)
}

pub fn mmu_attack(trng: &Trng) -> Vec<String> {
    if KILLED.load(Ordering::Relaxed) {
        return vec!["sandbox was already".to_string(), "killed by the kernel.".to_string(), "Power -> Restart".to_string(), "to run it again".to_string()];
    }
    use ed25519_dalek::{Signer, SigningKey, Verifier};
    let addr = crate::vault::ensure(trng);
    let before = crate::vault::seed(trng);
    let xns = match xous_names::XousNames::new() {
        Ok(x) => x,
        Err(_) => return vec!["no name server".to_string()],
    };
    let cid = match xns.request_connection("_ChocoSandbox_") {
        Ok(c) => c,
        Err(_) => return vec!["sandbox not running".to_string(), "(it was killed already:".to_string(), "restart to run it again)".to_string()],
    };
    let alive_before = sandbox_ping(cid);
    let _ = xous::send_message(cid, xous::Message::new_scalar(2, addr, 0, 0, 0)); // the attack: no reply expected
    std::thread::sleep(Duration::from_millis(500));
    let alive_after = sandbox_ping(cid);
    if !alive_after {
        KILLED.store(true, Ordering::Relaxed);
    }
    // the key must be untouched and still sign
    let after = crate::vault::seed(trng);
    let sk = SigningKey::from_bytes(&after);
    let sig = sk.sign(b"mmu demo");
    let valid = after == before && sk.verifying_key().verify(b"mmu demo", &sig).is_ok();
    vec![
        format!("sandbox before: {}", if alive_before { "alive" } else { "?" }),
        format!("it read {:08x}", addr),
        if alive_after { "sandbox SURVIVED?!".to_string() } else { "KILLED by the MMU".to_string() },
        format!("key intact, sig {}", if valid { "VALID" } else { "BAD" }),
    ]
}

