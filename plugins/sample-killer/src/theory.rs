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

/// Krumhansl–Kessler key profiles, from the tonic upward.
const MAJOR_PROFILE: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const MINOR_PROFILE: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub root: u8,
    pub minor: bool,
    /// 0..1: how clearly this key beats the next unrelated one.
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

/// The best of the 24 major and minor keys for `chroma`, or None when it is flat (nothing heard).
pub fn estimate_key(chroma: &[f32; 12]) -> Option<Key> {
    if chroma.iter().sum::<f32>() <= 0.0 {
        return None;
    }
    let mut scores = [(0.0f32, 0u8, false); 24];
    for root in 0..12u8 {
        let rotated: [f32; 12] = std::array::from_fn(|i| chroma[(i + root as usize) % 12]);
        scores[root as usize] = (correlation(&rotated, &MAJOR_PROFILE), root, false);
        scores[12 + root as usize] = (correlation(&rotated, &MINOR_PROFILE), root, true);
    }
    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let (best, root, minor) = scores[0];
    // The relative key shares every note, so only a different note set counts as a rival.
    let relative = if minor { ((root + 3) % 12, false) } else { ((root + 9) % 12, true) };
    let rival = scores[1..].iter().find(|s| (s.1, s.2) != relative).map_or(0.0, |s| s.0);
    let confidence = (best.clamp(0.0, 1.0) * 0.5 + ((best - rival) * 6.0).clamp(0.0, 1.0) * 0.5).clamp(0.0, 1.0);
    Some(Key { root, minor, confidence })
}

/// The tonic the scale is read from: a detected major key's relative minor for a minor scale, and the other way round.
pub fn scale_root(key_root: u8, key_minor: bool, scale: &Scale) -> u8 {
    match (key_minor, scale.minor) {
        (false, true) => (key_root + 9) % 12,
        (true, false) => (key_root + 3) % 12,
        _ => key_root,
    }
}

pub fn note_name(midi: u8) -> String {
    format!("{}{}", NOTE_NAMES[(midi % 12) as usize], midi as i32 / 12 - 1)
}

pub fn midi_hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// 808 picks in trap order of use: root, fifth, then the minor-key moves (bVI, bVII, IV, bIII, bII) or the major ones.
const BASS_ORDER_MINOR: [u8; 7] = [0, 7, 8, 10, 5, 3, 1];
const BASS_ORDER_MAJOR: [u8; 6] = [0, 7, 5, 9, 2, 4];

/// Up to five 808 notes for `root` in `scale`, best first, each in the octave where 808s sit (D1 to C#2, 37–69 Hz).
pub fn bass_notes(root: u8, scale: &Scale) -> Vec<Suggestion> {
    let order: &[u8] = if scale.minor { &BASS_ORDER_MINOR } else { &BASS_ORDER_MAJOR };
    order
        .iter()
        .filter(|iv| scale.intervals.contains(iv))
        .take(5)
        .enumerate()
        .map(|(rank, &iv)| {
            let pc = (root + iv) % 12;
            let midi = 26 + (pc + 10) % 12;
            Suggestion { pc, degree: DEGREES[iv as usize], note: note_name(midi), freq: midi_hz(midi as f32), strong: rank < 2 }
        })
        .collect()
}

/// The scale's notes upward from the root in octave 4; the tonic chord's notes (root, third, fifth) are `strong`.
pub fn melody_notes(root: u8, scale: &Scale) -> Vec<Suggestion> {
    scale
        .intervals
        .iter()
        .map(|&iv| {
            let midi = 60 + root + iv;
            Suggestion {
                pc: (root + iv) % 12,
                degree: DEGREES[iv as usize],
                note: note_name(midi),
                freq: midi_hz(midi as f32),
                strong: matches!(iv, 0 | 3 | 4 | 7),
            }
        })
        .collect()
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

    #[test]
    fn a_minor_loop_reads_as_a_minor() {
        // Am - F - C - G weighting: A and E strongest.
        let c = chroma_of(&[(9, 1.0), (0, 0.7), (4, 0.9), (5, 0.4), (7, 0.4), (2, 0.2), (11, 0.15)]);
        let k = estimate_key(&c).unwrap();
        assert_eq!((k.root, k.minor), (9, true));
    }

    #[test]
    fn f_sharp_minor_triad_reads_as_f_sharp_minor() {
        let c = chroma_of(&[(6, 1.0), (9, 0.6), (1, 0.7)]);
        let k = estimate_key(&c).unwrap();
        assert_eq!((k.root, k.minor), (6, true));
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
    fn eight_oh_eight_notes_in_f_sharp_minor() {
        let b = bass_notes(6, &SCALES[1]);
        let names: Vec<_> = b.iter().map(|s| (s.note.as_str(), s.degree)).collect();
        assert_eq!(names, vec![("F#1", "I"), ("C#2", "V"), ("D1", "bVI"), ("E1", "bVII"), ("B1", "IV")]);
        assert!((b[0].freq - 46.25).abs() < 0.05);
        assert!(b[0].strong && b[1].strong && !b[2].strong);
    }

    #[test]
    fn melody_marks_the_tonic_chord() {
        let m = melody_notes(9, &SCALES[2]);
        assert_eq!(m.len(), 7);
        assert_eq!(m[0].note, "A4");
        let strong: Vec<_> = m.iter().filter(|s| s.strong).map(|s| s.degree).collect();
        assert_eq!(strong, vec!["I", "bIII", "V"]);
    }
}
