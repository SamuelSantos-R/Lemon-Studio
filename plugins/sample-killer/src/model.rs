// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! What the editor paints and what a click on it means: the contract between the editor logic and `view`.

/// A suggested note: an 808 note or a melody note.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    /// Pitch class, 0 = C … 11 = B.
    pub pc: u8,
    /// Scale degree relative to the key's root, ASCII: "I", "bII", "II", "bIII", "III", "IV", "bV", "V", "bVI", "VI", "bVII", "VII".
    pub degree: &'static str,
    /// Note with octave, ASCII sharps: "F#1", "C2".
    pub note: String,
    /// Frequency of `note` in Hz.
    pub freq: f32,
    /// 808: one of the two best picks. Melody: a chord tone of the key's tonic chord (I, III/bIII, V).
    pub strong: bool,
}

/// Everything one editor frame shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    /// Key root pitch class (0 = C), None until enough audio was heard.
    pub key_root: Option<u8>,
    /// Whether the scale in use is a minor-family scale (the key reads "F# minor" vs "A major").
    pub key_minor: bool,
    /// The root was set by hand (`Root` parameter), not detected.
    pub root_locked: bool,
    /// Detection confidence 0..1 (ignored when `root_locked`).
    pub confidence: f32,
    /// Energy heard per pitch class, normalized so the largest is 1.0; all zero before any audio.
    pub chroma: [f32; 12],
    /// Note playing right now (MIDI number), None when silent or unpitched.
    pub live_midi: Option<u8>,
    /// Its frequency in Hz (0 when none).
    pub live_freq: f32,
    /// Its tuning offset in cents, -50..50.
    pub live_cents: f32,
    /// Input level 0..1 (peak, display-scaled).
    pub level: f32,
    /// Name of the selected scale, e.g. "Harmonic Minor".
    pub scale_name: &'static str,
    /// Analysis frozen (Hold).
    pub hold: bool,
    /// Up to 5 808 notes, best first.
    pub bass: Vec<Suggestion>,
    /// The scale's notes from the root upward (5 to 7 notes).
    pub melody: Vec<Suggestion>,
    /// Control under the mouse, for hover feedback.
    pub hover: Option<Hit>,
}

/// A clickable control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    ScalePrev,
    ScaleNext,
    RootPrev,
    RootNext,
    /// Back to the detected root (clears the hand-set root).
    RootAuto,
    Hold,
    Reset,
}
