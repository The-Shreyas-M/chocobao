// music.rs - tunes for the Music player. Written by ear from memory, so they are approximations: every tune is a
// plain list of (frequency in Hz, duration in ms) and easy to correct note by note. The buzzer is a single voice, so
// these are the melody only.
#![allow(dead_code)]
use crate::sound::Note;
use crate::sound::{A5, AS5, B5, C5, C6, DS5, E5, F5, G5, GS5, REST};

const CS6: u32 = 1109;
const D6: u32 = 1175;
const DS6: u32 = 1245;
const E6: u32 = 1319;
const F6: u32 = 1397;
const FS6: u32 = 1480;
const G6: u32 = 1568;
const GS6: u32 = 1661;

pub struct Tune {
    pub title: &'static str,
    pub by: &'static str,
    pub notes: &'static [Note],
}

// one beat = 175 ms (about 171 bpm, eighth notes)
const E8: u32 = 175;
const Q: u32 = 350;
const H: u32 = 700;

/// The synth riff and the first hook, in the original key (F minor), approximate.
pub const BLINDING_LIGHTS: &[Note] = &[
    // riff
    (C6, E8), (C6, E8), (C6, E8), (GS5, E8), (AS5, E8), (AS5, E8), (AS5, E8), (G5, E8),
    (GS5, E8), (GS5, E8), (GS5, E8), (F5, E8), (G5, E8), (G5, E8), (G5, E8), (DS5, E8),
    (C6, E8), (C6, E8), (C6, E8), (GS5, E8), (AS5, E8), (AS5, E8), (AS5, E8), (G5, E8),
    (GS5, E8), (G5, E8), (F5, E8), (G5, E8), (GS5, Q), (REST, Q),
    // "I said ooh, I'm blinded by the lights"
    (C6, E8), (C6, E8), (DS6, Q), (C6, E8), (AS5, E8), (GS5, Q), (REST, E8),
    (C6, E8), (C6, E8), (DS6, Q), (D6, E8), (C6, E8), (AS5, H), (REST, Q),
];

/// The chorus rhythm of "Attention", approximate.
pub const ATTENTION: &[Note] = &[
    (E6, E8), (E6, E8), (E6, E8), (D6, E8), (CS6, Q), (REST, E8),
    (E6, E8), (E6, E8), (E6, E8), (D6, E8), (CS6, Q), (REST, E8),
    (E6, E8), (E6, E8), (FS6, E8), (E6, E8), (D6, E8), (CS6, E8), (B5, Q), (REST, Q),
    (A5, E8), (A5, E8), (B5, E8), (CS6, E8), (D6, Q), (CS6, Q), (B5, Q), (REST, Q),
    (E6, E8), (E6, E8), (E6, E8), (D6, E8), (CS6, Q), (REST, E8),
    (E6, E8), (E6, E8), (E6, E8), (D6, E8), (CS6, Q), (REST, E8),
    (FS6, E8), (E6, E8), (D6, E8), (CS6, E8), (B5, H), (REST, Q),
];

pub const TUNES: [Tune; 2] = [
    Tune { title: "Blinding Lights", by: "The Weeknd", notes: BLINDING_LIGHTS },
    Tune { title: "Attention", by: "Charlie Puth", notes: ATTENTION },
];

/// Piano keys: (button name, frequency)
pub const KEYS: [(&str, u32); 5] = [("LEFT", C5), ("UP", E5), ("RIGHT", G5), ("DOWN", A5), ("OK", C6)];
