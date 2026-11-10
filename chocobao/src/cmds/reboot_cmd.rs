use String;

use crate::{CommonEnv, ShellCmdApi};

/// `reboot`: reset the whole chip in 0.5 s (same as the RESET button). With bootwait enabled the bootloader
/// then waits for commands, which is how tools/flash_auto.py gets the board into flashing mode by itself.
#[derive(Debug)]
pub struct RebootCmd {}

impl<'a> ShellCmdApi<'a> for RebootCmd {
    cmd_api!(reboot);

    fn process(&mut self, _args: String, _env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            crate::bootmode::restart_soc(true);
        });
        Ok(Some(String::from("rebooting")))
    }
}
