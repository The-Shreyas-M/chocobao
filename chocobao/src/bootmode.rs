// bootmode.rs - restart helper used by the display watchdog.
// (The "enter the bootloader from software" feature was removed on 10 Nov: flashing is PROG + RESET.)

/// Restart the app (CPU-only reset, PROG not touched). Last resort if the display bus ever hangs for good.
pub fn restart() {
    let xns = xous_names::XousNames::new().unwrap();
    match xous_api_susres::Susres::new_without_hook(&xns) {
        Ok(susres) => {
            if let Err(e) = susres.reboot(false) {
                log::error!("reboot request failed: {:?}", e);
            }
        }
        Err(e) => log::error!("no susres service: {:?}", e),
    }
}
