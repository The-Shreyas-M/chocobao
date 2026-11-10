// typing.rs - make the board type the RIGHT characters on a normal (US QWERTY) PC.
//
// Why this exists: the stock Xous keyboard service on this board always reports its keyboard map as
// Dvorak (services/bao1x-hal-service/src/servers/keyboard.rs, `GetKeyMap` returns KeyMap::Dvorak),
// and `UsbHid::send_str` / `send_keycode` both convert characters through that map. On a PC set to
// US QWERTY the result is a substituted jumble ('W' comes out as '<', 'q' as ''', ...).
//
// Fix without touching the flashed kernel: for each character we want the PC to receive, send the
// character that sits on the SAME physical key on a Dvorak keyboard. The board then emits exactly the
// HID key press a US keyboard would, and the PC types what we meant.
//
// Verified offline (8 Nov) against the service's own tables (services/usb-bao1x/src/mappings.rs):
// all 94 printable ASCII characters map to identical key presses, 0 mismatches.
// If the service is ever fixed to report the real layout, delete this translation.

// physical key rows of a US QWERTY keyboard (unshifted / shifted) ...
const QWERTY_LOW: &str = "`1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./";
const QWERTY_UP: &str = "~!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>?";
// ... and what a Dvorak keyboard prints on the same physical keys
const DVORAK_LOW: &str = "`1234567890[]',.pyfgcrl/=\\aoeuidhtns-;qjkxbmwvz";
const DVORAK_UP: &str = "~!@#$%^&*(){}\"<>PYFGCRL?+|AOEUIDHTNS_:QJKXBMWVZ";

/// Translate text so that, typed through the board's Dvorak-assuming keyboard service, a US QWERTY
/// PC receives exactly `s`. Characters not on the main keys (space, newline, ...) pass through.
pub fn as_us_keyboard(s: &str) -> String {
    let ql = QWERTY_LOW.as_bytes();
    let qu = QWERTY_UP.as_bytes();
    let dl = DVORAK_LOW.as_bytes();
    let du = DVORAK_UP.as_bytes();
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                let b = c as u8;
                if let Some(i) = ql.iter().position(|&x| x == b) {
                    return dl[i] as char;
                }
                if let Some(i) = qu.iter().position(|&x| x == b) {
                    return du[i] as char;
                }
            }
            c
        })
        .collect()
}
