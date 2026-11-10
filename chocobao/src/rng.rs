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

/// What kind of secret a password type makes.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// random characters from the pools below
    Chars,
    /// random BIP-39 words joined by '-'; the "length" is the number of words
    Words,
}

/// One entry in the Password maker's list of types (what a website might demand).
pub struct Profile {
    pub name: &'static str,
    pub kind: Kind,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: &'static str,
    /// if not empty, the ONLY characters used (hex, base32, base58...)
    pub custom: &'static str,
    /// drop look-alike characters (0 O o 1 l I) so it is easy to read or retype
    pub readable: bool,
    pub default_len: usize,
    pub len_min: usize,
    pub len_max: usize,
}

const SYMS_ALL: &str = "!@#$%^&*()-_=+[]{};:,.?/";
const SYMS_SAFE: &str = "!@#$%&*?-_";

const fn chars(name: &'static str, lower: bool, upper: bool, digits: bool, symbols: &'static str, readable: bool, len: usize) -> Profile {
    Profile { name, kind: Kind::Chars, lower, upper, digits, symbols, custom: "", readable, default_len: len, len_min: 4, len_max: 63 }
}

const fn only(name: &'static str, set: &'static str, len: usize) -> Profile {
    Profile { name, kind: Kind::Chars, lower: false, upper: false, digits: false, symbols: "", custom: set, readable: false, default_len: len, len_min: 4, len_max: 63 }
}

pub const PROFILES: [Profile; 17] = [
    chars("Everything", true, true, true, SYMS_ALL, false, 20),
    chars("Letters+digits", true, true, true, "", false, 16),
    chars("Safe symbols", true, true, true, SYMS_SAFE, false, 16),
    chars("Dash and underscore", true, true, true, "-_", false, 16),
    chars("Letters only", true, true, false, "", false, 20),
    chars("lowercase+digits", true, false, true, "", false, 16),
    chars("UPPERCASE+digits", false, true, true, "", false, 16),
    chars("lowercase only", true, false, false, "", false, 16),
    chars("UPPERCASE only", false, true, false, "", false, 16),
    chars("Digits only (PIN)", false, false, true, "", false, 6),
    chars("Digits+symbols", false, false, true, SYMS_SAFE, false, 12),
    chars("Readable", true, true, true, "", true, 16),
    chars("Readable+symbols", true, true, true, "!@#$%&*?", true, 16),
    only("Hex", "0123456789abcdef", 32),
    only("Base32", "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567", 26),
    only("Base58", "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz", 22),
    Profile {
        name: "Passphrase (words)",
        kind: Kind::Words,
        lower: false,
        upper: false,
        digits: false,
        symbols: "",
        custom: "",
        readable: false,
        default_len: 6,
        len_min: 3,
        len_max: 12,
    },
];

/// Index of the PIN type and of the passphrase type (used by the menu shortcuts).
pub const PROFILE_PIN: usize = 9;
pub const PROFILE_WORDS: usize = 16;

fn pools(p: &Profile) -> Vec<Vec<u8>> {
    let keep = |c: &u8| !p.readable || !b"0Oo1lI".contains(c);
    let mut v: Vec<Vec<u8>> = Vec::new();
    if !p.custom.is_empty() {
        v.push(p.custom.bytes().collect());
        return v;
    }
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

/// Random secret that follows the type: every enabled class appears at least once (so "must contain a number
/// and a symbol" is satisfied), it starts with a letter when letters are allowed (some sites insist), and it is
/// drawn uniformly from all strings that satisfy those rules. For the passphrase type `len` is the word count.
pub fn generate(trng: &Trng, p: &Profile, len: usize) -> Option<String> {
    let len = len.clamp(p.len_min, p.len_max);
    if p.kind == Kind::Words {
        let mut s = String::new();
        for i in 0..len {
            if i > 0 {
                s.push('-');
            }
            s.push_str(BIP39_TABLE[below(trng, BIP39_TABLE.len() as u32)? as usize]);
        }
        return Some(s);
    }
    let pl = pools(p);
    let all: Vec<u8> = pl.iter().flatten().copied().collect();
    if all.is_empty() {
        return None;
    }
    let nletter_classes = if p.custom.is_empty() { (p.lower as usize) + (p.upper as usize) } else { 0 };
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
    let len = len.clamp(p.len_min, p.len_max);
    if p.kind == Kind::Words {
        return len as u32 * 11;
    }
    let n = pools(p).iter().map(|c| c.len()).sum::<usize>().max(2);
    ((n as f32).log2() * len as f32) as u32
}

/// A valid BIP-39 recovery phrase of `n` words (12, 15, 18, 21 or 24) as word indexes, entropy from the TRNG.
pub fn seed_words(trng: &Trng, n: usize) -> Option<Vec<u16>> {
    let bytes = n * 11 * 32 / 33 / 8; // 12 words = 16 bytes ... 24 words = 32 bytes
    let mut e = vec![0u8; bytes];
    for chunk in e.chunks_mut(4) {
        let x = trng.get_u32().ok()?.to_le_bytes();
        chunk.copy_from_slice(&x[..chunk.len()]);
    }
    let idx = crate::seed::indexes(&e);
    e.iter_mut().for_each(|b| *b = 0);
    Some(idx)
}

pub fn bip39_word(i: u16) -> &'static str {
    BIP39_TABLE[(i as usize) & 2047]
}
