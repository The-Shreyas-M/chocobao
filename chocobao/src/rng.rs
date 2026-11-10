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

/// A set of rules a website might impose on passwords.
pub struct Profile {
    pub name: &'static str,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: &'static str,
    /// drop look-alike characters (0 O o 1 l I) so it is easy to read or retype
    pub readable: bool,
    pub default_len: usize,
}

const SYMS_ALL: &str = "!@#$%^&*()-_=+[]{};:,.?/";
const SYMS_SAFE: &str = "!@#$%&*?-_";

pub const PROFILES: [Profile; 7] = [
    Profile { name: "Everything", lower: true, upper: true, digits: true, symbols: SYMS_ALL, readable: false, default_len: 20 },
    Profile { name: "Letters+digits", lower: true, upper: true, digits: true, symbols: "", readable: false, default_len: 16 },
    Profile { name: "Safe symbols", lower: true, upper: true, digits: true, symbols: SYMS_SAFE, readable: false, default_len: 16 },
    Profile { name: "Letters only", lower: true, upper: true, digits: false, symbols: "", readable: false, default_len: 20 },
    Profile { name: "lower+digits", lower: true, upper: false, digits: true, symbols: "", readable: false, default_len: 16 },
    Profile { name: "Digits (PIN)", lower: false, upper: false, digits: true, symbols: "", readable: false, default_len: 6 },
    Profile { name: "Readable", lower: true, upper: true, digits: true, symbols: "", readable: true, default_len: 16 },
];

pub const LEN_MIN: usize = 4;
pub const LEN_MAX: usize = 63;

fn pools(p: &Profile) -> Vec<Vec<u8>> {
    let keep = |c: &u8| !p.readable || !b"0Oo1lI".contains(c);
    let mut v: Vec<Vec<u8>> = Vec::new();
    if p.lower {
        v.push(b"abcdefghijklmnopqrstuvwxyz".iter().copied().filter(keep).collect());
    }
    if p.upper {
        v.push(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ".iter().copied().filter(keep).collect());
    }
    if p.digits {
        v.push(b"0123456789".iter().copied().filter(keep).collect());
    }
    if !p.symbols.is_empty() {
        v.push(p.symbols.bytes().collect());
    }
    v
}

/// Random password that follows the profile: it always contains at least one character of every enabled
/// class (sites that require a number and a symbol are satisfied), starts with a letter when letters are
/// allowed (some sites insist), and is drawn uniformly from all strings that satisfy those rules.
pub fn generate(trng: &Trng, p: &Profile, len: usize) -> Option<String> {
    let len = len.clamp(LEN_MIN, LEN_MAX);
    let pl = pools(p);
    let all: Vec<u8> = pl.iter().flatten().copied().collect();
    if all.is_empty() {
        return None;
    }
    let nletter_classes = (p.lower as usize) + (p.upper as usize);
    let letters: Vec<u8> = pl.iter().take(nletter_classes).flatten().copied().collect();
    for _ in 0..2000 {
        let mut s: Vec<u8> = Vec::with_capacity(len);
        for i in 0..len {
            let src = if i == 0 && !letters.is_empty() { &letters } else { &all };
            s.push(src[below(trng, src.len() as u32)? as usize]);
        }
        if pl.iter().all(|class| s.iter().any(|c| class.contains(c))) {
            return String::from_utf8(s).ok();
        }
    }
    None
}

/// Approximate strength in bits.
pub fn bits(p: &Profile, len: usize) -> u32 {
    let n = pools(p).iter().map(|c| c.len()).sum::<usize>().max(2);
    ((n as f32).log2() * len as f32) as u32
}
