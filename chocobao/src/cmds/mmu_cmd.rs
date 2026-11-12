use String;

use crate::{CommonEnv, ShellCmdApi};

/// `mmu`: run the MMU demo from the console: the sandbox process reads the Notary's key page, the kernel kills it.
#[derive(Debug)]
pub struct MmuCmd {}

impl<'a> ShellCmdApi<'a> for MmuCmd {
    cmd_api!(mmu);

    fn process(&mut self, _args: String, env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        let intro = crate::demo::mmu_intro(&env.trng);
        let result = crate::demo::mmu_attack(&env.trng);
        Ok(Some(format!("{} | {}", intro.join(" / "), result.join(" / "))))
    }
}
