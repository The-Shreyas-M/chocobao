// bootmode.rs - restart helper used by the display watchdog.
// (The "enter the bootloader from software" feature was removed on 10 Nov: flashing is PROG + RESET.)

/// Reset the whole chip (like the RESET button) or only the CPU.
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

/// Make the PC see a fresh USB connection: drive the USB pull-up switch pin (PC13, the same pin as PROG) low for
/// 400 ms, then give it back. Needed after the bootloader's `boot` command: without a disconnect Windows can
/// keep the stale bootloader device and never list the app. Harmless otherwise.
pub fn usb_reconnect() {
    use bao1x_api::{IoSetup, IoxDir, IoxDriveStrength, IoxEnable, IoxFunction, IoxHal, IoxPort, IoxValue};
    use bao1x_api::IoGpio;
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
    iox.set_gpio_pin_value(IoxPort::PC, 13, IoxValue::Low);
    std::thread::sleep(std::time::Duration::from_millis(400));
    iox.setup_pin(
        IoxPort::PC,
        13,
        Some(IoxDir::Input),
        Some(IoxFunction::Gpio),
        Some(IoxEnable::Enable),
        Some(IoxEnable::Enable),
        None,
        None,
    );
}
