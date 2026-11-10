use String;

use crate::{CommonEnv, ShellCmdApi};

/// `diag`: one line of counters (uptime, heartbeat, display stats, panics). Polled by tools/watch_diag.py to
/// find out when and why the firmware misbehaves.
#[derive(Debug)]
pub struct DiagCmd {}

impl<'a> ShellCmdApi<'a> for DiagCmd {
    cmd_api!(diag);

    fn process(&mut self, _args: String, _env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        Ok(Some(crate::diag::line()))
    }
}
