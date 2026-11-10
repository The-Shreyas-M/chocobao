// dice.rs - ChocoBao "Dice": passwords, passphrases and dice from the chip's hardware TRNG.
//
// * Every random value comes from the hardware true random number generator service.
// * Values are drawn with rejection sampling, so there is no modulo bias.
// * No floating point: the entropy figure uses an integer-only log2 in Q8 fixed point.
// * Nothing is written to ReRAM. Nothing is stored. The result exists only in RAM.
// * `type` mode sends the result to the PC as USB keyboard keystrokes instead of printing it.
//   The text is translated (see typing.rs) so a US QWERTY PC receives exactly what was generated.
use core::fmt::Write;

use String;
use usb_bao1x::UsbHid;

use super::bip39_en::BIP39_TABLE;
use crate::{CommonEnv, ShellCmdApi};

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*-_=+?";

const HELP: &str = "dice commands:\n\
  dice roll [NdM]            roll N dice with M sides (default 1d6)\n\
  dice pw [len] [set]        random password, len 4..128 (default 20)\n\
                             sets: full (default), alnum, lower, digits, hex\n\
  dice words [n]             n random BIP-39 words joined by '-' (default 6, 11 bits each)\n\
  add `type` to pw/words     send the result as USB keystrokes (3 s to focus the target field)\n\
  Entropy is shown in bits. Nothing is stored; it exists only in RAM.";

#[derive(Debug)]
pub struct Dice {
    usb: Option<UsbHid>,
}
impl Dice {
    pub fn new() -> Self { Dice { usb: None } }
}

/// log2(n) as an unsigned Q8 fixed-point number (value * 256), integer maths only.
fn log2_q8(n: u32) -> u32 {
    if n < 2 {
        return 0;
    }
    let k = 31 - n.leading_zeros();
    // normalise n to the range [1, 2) in Q30
    let mut x: u64 = ((n as u64) << 30) >> k;
    let mut frac = 0u32;
    for i in 0..8 {
        x = (x * x) >> 30;
        if x >= (2u64 << 30) {
            x >>= 1;
            frac |= 1 << (7 - i);
        }
    }
    (k << 8) | frac
}

fn bits_string(count: u32, alphabet: u32) -> String {
    let q8 = count * log2_q8(alphabet);
    format!("{}.{} bits", q8 >> 8, ((q8 & 0xff) * 10) >> 8)
}

/// Uniform random number in 0..n using rejection sampling over the hardware TRNG.
fn below(env: &CommonEnv, n: u32) -> Result<u32, xous::Error> {
    let zone = u32::MAX - (u32::MAX % n); // anything at or above this would bias the result
    loop {
        let x = env.trng.get_u32()?;
        if x < zone {
            return Ok(x % n);
        }
    }
}

fn parse_dice(s: &str) -> Option<(u32, u32)> {
    let (a, b) = s.split_once('d')?;
    let n: u32 = if a.is_empty() { 1 } else { a.parse().ok()? };
    let m: u32 = b.parse().ok()?;
    if n >= 1 && n <= 100 && m >= 2 && m <= 1000 { Some((n, m)) } else { None }
}

impl<'a> ShellCmdApi<'a> for Dice {
    cmd_api!(dice);

    fn process(&mut self, args: String, env: &mut CommonEnv) -> Result<Option<String>, xous::Error> {
        let mut ret = String::new();
        let mut tokens: Vec<&str> = args.split_whitespace().collect();
        let type_it = tokens.iter().any(|t| *t == "type");
        tokens.retain(|t| *t != "type");
        let sub = tokens.first().copied().unwrap_or("help");

        let mut text = String::new(); // the secret, if any
        match sub {
            "roll" => {
                let spec = tokens.get(1).copied().unwrap_or("1d6");
                match parse_dice(spec) {
                    Some((n, m)) => {
                        let mut sum = 0u32;
                        write!(ret, "{}:", spec).ok();
                        for _ in 0..n {
                            let r = below(env, m)? + 1;
                            sum += r;
                            write!(ret, " {}", r).ok();
                        }
                        if n > 1 {
                            write!(ret, "  (sum {})", sum).ok();
                        }
                    }
                    None => {
                        write!(ret, "usage: dice roll [NdM], N 1..100, M 2..1000").ok();
                    }
                }
            }
            "pw" => {
                let len: u32 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
                let set = tokens.get(2).copied().unwrap_or("full");
                let mut alphabet = String::new();
                match set {
                    "full" => {
                        alphabet.push_str(LOWER);
                        alphabet.push_str(UPPER);
                        alphabet.push_str(DIGITS);
                        alphabet.push_str(SYMBOLS);
                    }
                    "alnum" => {
                        alphabet.push_str(LOWER);
                        alphabet.push_str(UPPER);
                        alphabet.push_str(DIGITS);
                    }
                    "lower" => alphabet.push_str(LOWER),
                    "digits" => alphabet.push_str(DIGITS),
                    "hex" => alphabet.push_str("0123456789abcdef"),
                    _ => {}
                }
                if alphabet.is_empty() || len < 4 || len > 128 {
                    write!(ret, "usage: dice pw [len 4..128] [full|alnum|lower|digits|hex]").ok();
                } else {
                    let chars: Vec<char> = alphabet.chars().collect();
                    for _ in 0..len {
                        text.push(chars[below(env, chars.len() as u32)? as usize]);
                    }
                    write!(ret, "{} chars from a {}-symbol set = {}", len, chars.len(), bits_string(len, chars.len() as u32))
                        .ok();
                }
            }
            "words" => {
                let n: u32 = tokens.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
                if n < 1 || n > 24 {
                    write!(ret, "usage: dice words [1..24]").ok();
                } else {
                    for i in 0..n {
                        if i > 0 {
                            text.push('-');
                        }
                        text.push_str(BIP39_TABLE[below(env, BIP39_TABLE.len() as u32)? as usize]);
                    }
                    write!(ret, "{} words from a 2048-word list = {}", n, bits_string(n, 2048)).ok();
                }
            }
            _ => {
                write!(ret, "{}", HELP).ok();
            }
        }

        if !text.is_empty() {
            if type_it {
                write!(ret, "\ntyping {} characters in 3 s - click the target field now...", text.len()).ok();
                env.ticktimer.sleep_ms(3000).ok();
                let usb = self.usb.get_or_insert_with(UsbHid::new);
                match usb.send_str(&crate::typing::as_us_keyboard(&text)) {
                    Ok(n) => write!(ret, "\nsent {} keystrokes (not shown here)", n).ok(),
                    Err(_) => write!(ret, "\ncan't type: is the board connected to a host?").ok(),
                };
            } else {
                write!(ret, "\n{}", text).ok();
            }
        }
        Ok(Some(ret))
    }
}
