// rng.rs - passwords, passphrases and dice from the hardware TRNG (rejection sampling, no modulo bias).
// Nothing here is stored or logged.
use bao1x_hal_service::trng::Trng;

use crate::cmds::bip39_en::BIP39_TABLE;

const ALNUM: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Uniform number in 0..n.
pub fn below(trng: &Trng, n: u32) -> Option<u32> {
    if n == 0 {
        return None;
    }
    let zone = u32::MAX - (u32::MAX % n);
    loop {
        let x = trng.get_u32().ok()?;
        if x < zone {
            return Some(x % n);
        }
    }
}

/// 20 random letters and digits (about 119 bits).
pub fn password(trng: &Trng) -> Option<String> {
    let mut s = String::new();
    for _ in 0..20 {
        s.push(ALNUM[below(trng, ALNUM.len() as u32)? as usize] as char);
    }
    Some(s)
}

/// 6 random BIP-39 words joined by '-' (66 bits).
pub fn passphrase(trng: &Trng) -> Option<String> {
    let mut s = String::new();
    for i in 0..6 {
        if i > 0 {
            s.push('-');
        }
        s.push_str(BIP39_TABLE[below(trng, BIP39_TABLE.len() as u32)? as usize]);
    }
    Some(s)
}
