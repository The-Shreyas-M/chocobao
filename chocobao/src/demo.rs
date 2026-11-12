// demo.rs - the MMU demo (also runnable from the serial console with `mmu`, handy for testing without the buttons).
//
// Three processes: this app (screen, buttons), chocosigner (holds the Notary key), chocosandbox (stands in for
// untrusted code). The sandbox is told the virtual address of the signer's key page and reads it. That address is
// valid in the signer and unmapped in the sandbox, so the kernel kills the sandbox. The app and the signer carry on.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Set once the kernel has killed the sandbox; it cannot be restarted until the chip restarts.
static KILLED: AtomicBool = AtomicBool::new(false);

fn hex8(b: &[u8]) -> String { b.iter().take(4).map(|x| format!("{:02x}", x)).collect() }

pub fn mmu_intro() -> Vec<String> {
    match crate::signer::info() {
        Some((pubk, va)) => vec![
            "Key lives in the".to_string(),
            format!("SIGNER process @{:08x}", va),
            format!("public key {}..", hex8(&pubk)),
            "OK = sandbox attacks".to_string(),
        ],
        None => vec!["signer process".to_string(), "does not answer".to_string()],
    }
}

fn sandbox_ping(cid: xous::CID) -> bool {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = xous::send_message(cid, xous::Message::new_blocking_scalar(1, 0, 0, 0, 0));
        tx.send(r.is_ok()).ok();
    });
    rx.recv_timeout(Duration::from_millis(800)).unwrap_or(false)
}

pub fn mmu_attack() -> Vec<String> {
    if KILLED.load(Ordering::Relaxed) {
        return vec!["sandbox was already".to_string(), "killed by the kernel.".to_string(), "Power -> Restart".to_string(), "to run it again".to_string()];
    }
    let (pub_before, va) = match crate::signer::info() {
        Some(x) => x,
        None => return vec!["signer does not answer".to_string()],
    };
    let xns = match xous_names::XousNames::new() {
        Ok(x) => x,
        Err(_) => return vec!["no name server".to_string()],
    };
    let cid = match xns.request_connection("_ChocoSandbox_") {
        Ok(c) => c,
        Err(_) => return vec!["sandbox not running".to_string(), "Power -> Restart".to_string(), "and try again".to_string()],
    };
    let alive_before = sandbox_ping(cid);
    let _ = xous::send_message(cid, xous::Message::new_scalar(2, va as usize, 0, 0, 0)); // the attack: no reply expected
    std::thread::sleep(Duration::from_millis(500));
    let alive_after = sandbox_ping(cid);
    if !alive_after {
        KILLED.store(true, Ordering::Relaxed);
    }
    // the signer must still answer, with the same key
    let still = crate::signer::sign(b"mmu demo");
    let same_key = still.map_or(false, |(_, p)| p == pub_before);
    vec![
        format!("sandbox before: {}", if alive_before { "alive" } else { "?" }),
        format!("it read signer {:08x}", va),
        if alive_after { "sandbox SURVIVED?!".to_string() } else { "KILLED by the MMU".to_string() },
        format!("signer {}", if same_key { "signs, same key" } else { "PROBLEM" }),
    ]
}
