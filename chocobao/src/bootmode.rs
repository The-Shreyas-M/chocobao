// bootmode.rs - "go to the bootloader" from software, shared by the console command and the menu.
//
// With `bootwait` disabled, boot1 only stays in the bootloader if PROG (pin PC13) is held low when the chip
// resets. Driving PC13 from the app does not work (tested 9-10 Nov: boot1 re-configures that pin first and
// the board just restarts into the app). So the split is:
//   * the app does the RESET, after a short countdown (so it works even though pressing PROG drops USB),
//   * the user holds PROG during the countdown and releases it when the flasher says so.
// No RESET tap, no re-plugging. If PROG is not held, the board simply restarts into the app (harmless).
use std::time::Duration;

/// Seconds between the command and the reset: time to press and hold PROG.
pub const COUNTDOWN_S: u64 = 5;

/// Returns immediately; a thread resets the whole chip after the countdown.
pub fn enter_bootloader() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(COUNTDOWN_S));
        restart_soc(true);
    });
}

/// Reset the chip. `whole_soc = true` is the same as tapping RESET; false resets only the CPU.
pub fn restart_soc(whole_soc: bool) {
    let xns = xous_names::XousNames::new().unwrap();
    match xous_api_susres::Susres::new_without_hook(&xns) {
        Ok(susres) => {
            if let Err(e) = susres.reboot(whole_soc) {
                log::error!("reboot request failed: {:?}", e);
            }
        }
        Err(e) => log::error!("no susres service: {:?}", e),
    }
}

/// Restart the app (CPU-only reset, PROG not touched). Used as a last-resort watchdog if the display bus hangs.
pub fn restart() { restart_soc(false); }
