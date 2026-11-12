use String;

use crate::{CommonEnv, ShellCmdApi};

/// `mmu`: run the MMU demo from the console: the sandbox process reads the signer's key page, the kernel kills it.
#[derive(Debug)]
pub struct MmuCmd {}

impl<'a> ShellCmdApi<'a> for MmuCmd {
    cmd_api!(mmu);

    fn process(&mut self, _args: String, _env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        let intro = crate::demo::mmu_intro();
        let result = crate::demo::mmu_attack();
        Ok(Some(format!("{} | {}", intro.join(" / "), result.join(" / "))))
    }
}

/// `sign [text]`: ask the signer process for a Notary proof line (for testing without the screen). Verify it with tools/verify.py.
#[derive(Debug)]
pub struct SignCmd {}

impl<'a> ShellCmdApi<'a> for SignCmd {
    cmd_api!(sign);

    fn process(&mut self, args: String, _env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let text = args.trim();
        let evt = if text.is_empty() { "CONSOLE" } else { text };
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let msg = format!("CHOCONOTARY1|{}|0|{}", n, evt);
        match crate::signer::sign(msg.as_bytes()) {
            Some((sig, p)) => {
                let hex = |b: &[u8]| b.iter().map(|x| format!("{:02x}", x)).collect::<String>();
                Ok(Some(format!("{}|{}|{}", msg, hex(&sig), hex(&p))))
            }
            None => Ok(Some(String::from("signer does not answer"))),
        }
    }
}
