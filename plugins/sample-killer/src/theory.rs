// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Music theory for Sample Killer: key estimation from a chromagram, the scales trap uses, and the 808 and melody notes
//! a key and scale suggest.

use crate::model::Suggestion;

pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const DEGREES: [&str; 12] = ["I", "bII", "II", "bIII", "III", "IV", "bV", "V", "bVI", "VI", "bVII", "VII"];

pub struct Scale {
    pub name: &'static str,
    pub intervals: &'static [u8],
    /// Read against the detected key's minor tonic (else its major tonic).
    pub minor: bool,
}

/// Index 0 is Auto: Natural Minor or Major, whichever mode the key was heard in.
pub const SCALES: [Scale; 9] = [
    Scale { name: "Auto", intervals: &[], minor: true },
    Scale { name: "Natural Minor", intervals: &[0, 2, 3, 5, 7, 8, 10], minor: true },
    Scale { name: "Harmonic Minor", intervals: &[0, 2, 3, 5, 7, 8, 11], minor: true },
    Scale { name: "Phrygian", intervals: &[0, 1, 3, 5, 7, 8, 10], minor: true },
    Scale { name: "Phrygian Dominant", intervals: &[0, 1, 4, 5, 7, 8, 10], minor: true },
    Scale { name: "Dorian", intervals: &[0, 2, 3, 5, 7, 9, 10], minor: true },
    Scale { name: "Minor Pentatonic", intervals: &[0, 3, 5, 7, 10], minor: true },
    Scale { name: "Blues", intervals: &[0, 3, 5, 6, 7, 10], minor: true },
    Scale { name: "Major", intervals: &[0, 2, 4, 5, 7, 9, 11], minor: false },
];
pub const SCALE_COUNT: i32 = SCALES.len() as i32;

/// The scale in effect: Auto resolves to Natural Minor or Major by the detected mode.
pub fn resolve_scale(index: i32, detected_minor: bool) -> &'static Scale {
    match index.clamp(0, SCALE_COUNT - 1) {
        0 if detected_minor => &SCALES[1],
        0 => &SCALES[8],
        i => &SCALES[i as usize],
    }
}

/// Krumhansl–Kessler key profiles, from the tonic upward: only used to pick the tonic inside a note set.
const MAJOR_PROFILE: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const MINOR_PROFILE: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];
const NATURAL: [u8; 7] = [0, 2, 3, 5, 7, 8, 10];
const HARMONIC: [u8; 7] = [0, 2, 3, 5, 7, 8, 11];
/// What a pitch class heard outside the candidate scale costs, against one inside.
const OUTSIDE_COST: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub root: u8,
    pub minor: bool,
    /// 0..1: how clearly this note set beats the next best one.
    pub confidence: f32,
}

fn correlation(a: &[f32; 12], b: &[f32; 12]) -> f32 {
    let ma = a.iter().sum::<f32>() / 12.0;
    let mb = b.iter().sum::<f32>() / 12.0;
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for i in 0..12 {
        let (x, y) = (a[i] - ma, b[i] - mb);
        num += x * y;
        da += x * x;
        db += y * y;
    }
    if da <= 0.0 || db <= 0.0 { 0.0 } else { num / (da * db).sqrt() }
}

/// How well the minor scale `intervals` on `root` covers `chroma` (summing to 1): energy inside minus energy outside.
fn fit(chroma: &[f32; 12], root: u8, intervals: &[u8]) -> f32 {
    (0..12u8).map(|pc| {
        let inside = intervals.contains(&((pc + 12 - root) % 12));
        if inside { chroma[pc as usize] } else { -OUTSIDE_COST * chroma[pc as usize] }
    }).sum()
}

/// The key of `chroma`: first the note set that covers it best (the 12 natural minor / major sets and the 12 harmonic
/// minor ones), so the notes decide (B against Bb tells A minor from D minor); then, for a natural set, whether it sits
/// on its minor or its relative major tonic. None when nothing was heard.
pub fn estimate_key(chroma: &[f32; 12]) -> Option<Key> {
    let sum: f32 = chroma.iter().sum();
    if sum <= 0.0 {
        return None;
    }
    let c = chroma.map(|v| v / sum);
    let mut sets: Vec<(f32, u8, bool)> = (0..12u8)
        .flat_map(|r| [(fit(&c, r, &NATURAL), r, false), (fit(&c, r, &HARMONIC), r, true)])
        .collect();
    sets.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let (best, root, harmonic) = sets[0];
    let second = sets[1].0;
    let confidence = ((best - second) / best.abs().max(1e-6) * 2.5).clamp(0.0, 1.0) * best.clamp(0.0, 1.0).sqrt();
    if harmonic {
        return Some(Key { root, minor: true, confidence });
    }
    let rotate = |r: u8| -> [f32; 12] { std::array::from_fn(|i| c[(i + r as usize) % 12]) };
    let major_root = (root + 3) % 12;
    let as_minor = correlation(&rotate(root), &MINOR_PROFILE);
    let as_major = correlation(&rotate(major_root), &MAJOR_PROFILE);
    // Trap leans minor: the major reading must win clearly.
    Some(if as_major > as_minor + 0.1 { Key { root: major_root, minor: false, confidence } } else { Key { root, minor: true, confidence } })
}

/// The tonic the scale is read from: a detected major key's relative minor for a minor scale, and the other way round.
pub fn scale_root(key_root: u8, key_minor: bool, scale: &Scale) -> u8 {
    match (key_minor, scale.minor) {
        (false, true) => (key_root + 9) % 12,
        (true, false) => (key_root + 3) % 12,
        _ => key_root,
    }
}

/// `midi` as Lemon Studio's and FL's piano roll name it: MIDI 60 is C5.
pub fn note_name(midi: u8) -> String {
    format!("{}{}", NOTE_NAMES[(midi % 12) as usize], midi as i32 / 12)
}

pub fn midi_hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// The MIDI note of pitch class `pc` in piano roll octave `octave` (C5 = 60).
fn in_octave(pc: u8, octave: i32) -> u8 {
    (octave * 12 + pc as i32).clamp(0, 127) as u8
}

/// The order 808 notes get used in trap: root, fifth, then the minor-key moves (bVI, bVII, IV, bIII, bII, II) or the
/// major ones; whatever the scale has left comes after.
const BASS_ORDER_MINOR: [u8; 12] = [0, 7, 8, 10, 5, 3, 1, 2, 11, 6, 4, 9];
const BASS_ORDER_MAJOR: [u8; 12] = [0, 7, 5, 9, 2, 4, 11, 10, 3, 8, 1, 6];

/// Every note of `scale` on `root` for the 808, best first, in piano roll octave `octave`.
pub fn bass_notes(root: u8, scale: &Scale, octave: i32, usage: &[f32; 12]) -> Vec<Suggestion> {
    let order: &[u8] = if scale.minor { &BASS_ORDER_MINOR } else { &BASS_ORDER_MAJOR };
    order
        .iter()
        .filter(|iv| scale.intervals.contains(iv))
        .enumerate()
        .map(|(rank, &iv)| suggestion(root, iv, octave, usage, rank < 2))
        .collect()
}

/// The notes of `scale` upward from `root` in octave `octave`, for a counter melody: `strong` marks the tonic chord
/// (root, third, fifth); `usage` says how much the sample plays each, so the little-used ones are the free space.
pub fn melody_notes(root: u8, scale: &Scale, octave: i32, usage: &[f32; 12]) -> Vec<Suggestion> {
    scale.intervals.iter().map(|&iv| suggestion(root, iv, octave, usage, matches!(iv, 0 | 3 | 4 | 7))).collect()
}

fn suggestion(root: u8, iv: u8, octave: i32, usage: &[f32; 12], strong: bool) -> Suggestion {
    let pc = (root + iv) % 12;
    let midi = in_octave(root, octave) as i32 + iv as i32;
    let midi = midi.clamp(0, 127) as u8;
    Suggestion { pc, degree: DEGREES[iv as usize], note: note_name(midi), freq: midi_hz(midi as f32), strong, usage: usage[pc as usize] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chroma_of(pcs: &[(u8, f32)]) -> [f32; 12] {
        let mut c = [0.0; 12];
        for &(pc, w) in pcs {
            c[pc as usize] = w;
        }
        c
    }

    /// The BNYX "How Was NY" loop as Sample Killer hears it (`examples/analyze`): Bb and no B, so D minor, not A minor.
    const HOW_WAS_NY: [f32; 12] = [0.63, 0.15, 0.87, 0.16, 1.0, 0.49, 0.23, 0.71, 0.30, 0.89, 0.48, 0.15];

    #[test]
    fn the_notes_decide_d_minor_over_a_minor() {
        let k = estimate_key(&HOW_WAS_NY).unwrap();
        assert_eq!((k.root, k.minor), (2, true), "{k:?}");
    }

    #[test]
    fn a_minor_loop_reads_as_a_minor() {
        let c = chroma_of(&[(9, 1.0), (0, 0.7), (4, 0.9), (5, 0.4), (7, 0.4), (2, 0.2), (11, 0.15)]);
        let k = estimate_key(&c).unwrap();
        assert_eq!((k.root, k.minor), (9, true));
    }

    #[test]
    fn a_major_chord_loop_reads_as_major() {
        // C - F - G - C with the tonic triad heaviest.
        let c = chroma_of(&[(0, 1.0), (4, 0.8), (7, 0.9), (5, 0.5), (9, 0.4), (2, 0.4), (11, 0.4)]);
        let k = estimate_key(&c).unwrap();
        assert_eq!((k.root, k.minor), (0, false), "{k:?}");
    }

    #[test]
    fn a_raised_seventh_reads_as_harmonic_minor() {
        let c = chroma_of(&[(9, 1.0), (0, 0.6), (4, 0.8), (8, 0.6), (11, 0.4), (2, 0.4), (5, 0.4)]);
        let k = estimate_key(&c).unwrap();
        assert_eq!((k.root, k.minor), (9, true), "{k:?}");
    }

    #[test]
    fn silence_has_no_key() {
        assert!(estimate_key(&[0.0; 12]).is_none());
    }

    #[test]
    fn relative_minor_for_a_minor_scale() {
        assert_eq!(scale_root(0, false, &SCALES[1]), 9);
        assert_eq!(scale_root(9, true, &SCALES[8]), 0);
        assert_eq!(scale_root(9, true, &SCALES[2]), 9);
    }

    #[test]
    fn names_follow_the_piano_roll() {
        assert_eq!(note_name(60), "C5");
        assert_eq!(note_name(62), "D5");
    }

    #[test]
    fn every_scale_note_goes_to_the_808_best_first() {
        let b = bass_notes(2, &SCALES[1], 5, &HOW_WAS_NY);
        let names: Vec<_> = b.iter().map(|s| (s.note.as_str(), s.degree)).collect();
        assert_eq!(names, vec![("D5", "I"), ("A5", "V"), ("A#5", "bVI"), ("C6", "bVII"), ("G5", "IV"), ("F5", "bIII"), ("E5", "II")]);
        assert!(b[0].strong && b[1].strong && !b[2].strong);
        assert!((b[0].usage - 0.87).abs() < 1e-6);
    }

    #[test]
    fn the_octave_moves_every_note() {
        let b = bass_notes(2, &SCALES[1], 4, &[0.0; 12]);
        assert_eq!(b[0].note, "D4");
    }

    #[test]
    fn melody_marks_the_tonic_chord() {
        let m = melody_notes(9, &SCALES[2], 5, &[0.0; 12]);
        assert_eq!(m.len(), 7);
        assert_eq!(m[0].note, "A5");
        let strong: Vec<_> = m.iter().filter(|s| s.strong).map(|s| s.degree).collect();
        assert_eq!(strong, vec!["I", "bIII", "V"]);
    }
}
