// chocosandbox - a second Xous process that stands in for untrusted code (a game, a plugin).
//
// Xous gives every process its own address space; the memory management unit (MMU) only maps the pages the kernel
// gave to that process. The main ChocoBao app owns a key page. Here the sandbox is told that page's virtual address
// and dereferences it. In the main app that address is valid; in this process nothing is mapped there, so the CPU
// raises a page fault and the kernel terminates THIS process. The main app, the OS and the key are untouched.
//
// Messages (scalar, to the server "_ChocoSandbox_"): opcode 1 = ping (replies 1), opcode 2 = "read this address".
fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name("_ChocoSandbox_", Some(4)).expect("register sandbox");
    log::info!("sandbox up, pid {}", xous::process::id());
    loop {
        let msg = xous::receive_message(sid).unwrap();
        if let Some(sc) = msg.body.scalar_message() {
            match sc.id {
                1 => {
                    xous::return_scalar(msg.sender, 1).ok();
                }
                2 => {
                    let addr = sc.arg1;
                    log::info!("sandbox: reading {:x}", addr);
                    // the attack: a plain read of an address that belongs to another process
                    let v = unsafe { core::ptr::read_volatile(addr as *const u8) };
                    // never reached when the MMU does its job
                    log::info!("sandbox: SURVIVED, read {:x}", v);
                    xous::return_scalar(msg.sender, 0x1000 + v as usize).ok();
                }
                _ => {}
            }
        }
    }
}
