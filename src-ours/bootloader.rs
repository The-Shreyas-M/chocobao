use String;

use crate::{CommonEnv, ShellCmdApi};

/// `bootloader`: reset the whole SoC. With `bootwait` enabled (our board's state), boot1 then
/// stays in bootloader mode (BAOCHIP drive + serial console), ready for `baoflash.py`.
/// Uses the hal-service susres reboot path (RebootRequest + RebootSocConfirm), the same register
/// write that boot1's own `reset` command performs. Runs in its own thread so a missing service
/// cannot hang the shell, and waits a moment so the reply reaches the host first.
#[derive(Debug)]
pub struct Bootloader {}

impl<'a> ShellCmdApi<'a> for Bootloader {
    cmd_api!(bootloader);

    fn process(&mut self, _args: String, _env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let xns = xous_names::XousNames::new().unwrap();
            match xous_api_susres::Susres::new_without_hook(&xns) {
                Ok(susres) => {
                    if let Err(e) = susres.reboot(true) {
                        log::error!("reboot request failed: {:?}", e);
                    }
                }
                Err(e) => log::error!("no susres service: {:?}", e),
            }
        });
        Ok(Some(String::from("bootloader: resetting the SoC in 0.3 s")))
    }
}
