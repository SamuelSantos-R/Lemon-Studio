// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Music theory helpers for Lemon Note ID: note names, scale intervals, 808 bass companion notes.

pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

pub const SCALE_COUNT: i32 = 5;
pub const SCALE_NAMES: [&str; 5] = ["Minor", "Minor Penta.", "Phrygian Dom.", "Blues", "Major"];

/// Semitone intervals from root for each scale.
const MINOR: &[u8] = &[0, 2, 3, 5, 7, 8, 10];
const MINOR_PENTA: &[u8] = &[0, 3, 5, 7, 10];
const PHRYGIAN_DOM: &[u8] = &[0, 1, 4, 5, 7, 8, 10];
const BLUES: &[u8] = &[0, 3, 5, 6, 7, 10];
const MAJOR: &[u8] = &[0, 2, 4, 5, 7, 9, 11];

/// Semitone intervals for the given scale index (0=Minor…4=Major).
pub fn scale_intervals(scale_i: i32) -> &'static [u8] {
    match scale_i {
        0 => MINOR,
        1 => MINOR_PENTA,
        2 => PHRYGIAN_DOM,
        3 => BLUES,
        _ => MAJOR,
    }
}

/// Convert frequency to MIDI note number (0–127). Returns None for out-of-range.
pub fn freq_to_midi(freq: f32) -> Option<u8> {
    let midi = 69.0 + 12.0 * (freq / 440.0_f32).log2();
    if midi >= 0.0 && midi <= 127.0 { Some(midi.round() as u8) } else { None }
}

/// Pitch class (0=C … 11=B) of a MIDI note.
#[inline]
pub fn midi_to_pc(midi: u8) -> u8 {
    midi % 12
}

/// Scientific octave number of a MIDI note (C4 = MIDI 60 → octave 4).
#[inline]
pub fn midi_to_octave(midi: u8) -> i8 {
    (midi / 12) as i8 - 1
}

/// Which pitch classes are 808 bass "power companions" of `root_pc`: perfect 4th and 5th.
/// Returns a 12-element boolean array indexed by pitch class.
pub fn bass_pcs(root_pc: u8) -> [bool; 12] {
    let mut set = [false; 12];
    set[((root_pc + 5) % 12) as usize] = true; // perfect 4th
    set[((root_pc + 7) % 12) as usize] = true; // perfect 5th
    set
}

/// Which pitch classes belong to the given scale starting on `root_pc`.
/// Returns a 12-element boolean array indexed by pitch class.
pub fn scale_pcs(root_pc: u8, scale_i: i32) -> [bool; 12] {
    let mut set = [false; 12];
    for &iv in scale_intervals(scale_i) {
        set[(root_pc as usize + iv as usize) % 12] = true;
    }
    set
}
