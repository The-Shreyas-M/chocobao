// bootmode.rs - "press PROG and RESET" from software, shared by the console command and the menu.
//
// With `bootwait` disabled, boot1 stays in the bootloader only if PROG (pin PC13) is low when it starts.
// So: drive PC13 low (this also drops USB: SE0), hold it 150 ms, then reset ONLY THE CPU
// (susres `reboot(false)` = SFR_RCURST1) so the pads keep their state and PC13 is still low when boot1
// looks. (A whole-SoC reset, `reboot(true)`, puts PC13 back to "not pressed": tested 9 Nov, the board
// just restarted into the app.) The MicroPython port's `machine.bootloader()` uses the same idea.
// If PC13 is released anyway, the board simply restarts into the app (harmless).
use bao1x_api::{IoGpio, IoSetup, IoxDir, IoxDriveStrength, IoxFunction, IoxHal, IoxPort, IoxValue};

/// Returns immediately; the work happens on its own thread after a short delay so replies can get out.
pub fn enter_bootloader() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let iox = IoxHal::new();
        iox.setup_pin(
            IoxPort::PC,
            13,
            Some(IoxDir::Output),
            Some(IoxFunction::Gpio),
            None,
            None,
            None,
            Some(IoxDriveStrength::Drive2mA),
        );
        iox.set_gpio_pin_value(IoxPort::PC, 13, IoxValue::Low); // "PROG pressed"
        std::thread::sleep(std::time::Duration::from_millis(150));
        let xns = xous_names::XousNames::new().unwrap();
        match xous_api_susres::Susres::new_without_hook(&xns) {
            Ok(susres) => {
                if let Err(e) = susres.reboot(false) {
                    log::error!("reboot request failed: {:?}", e);
                }
            }
            Err(e) => log::error!("no susres service: {:?}", e),
        }
    });
}

/// Restart the app (CPU-only reset, PROG not touched). Used as a last-resort watchdog if the display bus hangs.
pub fn restart() {
    let xns = xous_names::XousNames::new().unwrap();
    if let Ok(susres) = xous_api_susres::Susres::new_without_hook(&xns) {
        susres.reboot(false).ok();
    }
}
