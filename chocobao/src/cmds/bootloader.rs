use String;

use crate::{CommonEnv, ShellCmdApi};

/// `bootloader`: put the board into the boot1 bootloader without touching any button.
/// See `bootmode.rs` for how (PC13 low + CPU-only reset).
#[derive(Debug)]
pub struct Bootloader {}

impl<'a> ShellCmdApi<'a> for Bootloader {
    cmd_api!(bootloader);

    fn process(&mut self, _args: String, _env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        crate::bootmode::enter_bootloader();
        Ok(Some(String::from("bootloader: HOLD PROG now. Chip resets in 5 s; release PROG when the flasher says so")))
    }
}
