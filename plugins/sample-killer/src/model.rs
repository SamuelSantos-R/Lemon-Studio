// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! What the editor paints and what a click on it means: the contract between the editor logic and `view`.

/// A suggested note: an 808 note or a counter melody note.
#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    /// Pitch class, 0 = C … 11 = B.
    pub pc: u8,
    /// Scale degree relative to the key's root, ASCII: "I", "bII", "II", "bIII", "III", "IV", "bV", "V", "bVI", "VI", "bVII", "VII".
    pub degree: &'static str,
    /// Note with its piano roll octave (FL / Lemon Studio naming, C5 = middle C), ASCII sharps: "F#5", "C6".
    pub note: String,
    /// Frequency of `note` in Hz.
    pub freq: f32,
    /// 808: one of the two best picks. Melody: a note of the tonic chord (I, III/bIII, V).
    pub strong: bool,
    /// How much the sample plays this pitch class, 0..1 (1 = its most played note). Low = free space for a counter melody.
    pub usage: f32,
}

/// Everything one editor frame shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    /// Key root pitch class (0 = C), None until enough audio was heard.
    pub key_root: Option<u8>,
    /// Whether the scale in use is a minor-family scale ("D minor" vs "F major").
    pub key_minor: bool,
    /// The same notes read from the other tonic, e.g. "F major" for D minor; None when there is no key.
    pub relative: Option<String>,
    /// The root was set by hand (`Root` parameter), not detected.
    pub root_locked: bool,
    /// Detection confidence 0..1 (ignored when `root_locked`).
    pub confidence: f32,
    /// Input level 0..1 (peak, display-scaled): whether audio is coming in.
    pub level: f32,
    /// Name of the selected scale, e.g. "Harmonic Minor" or "Auto (Natural Minor)".
    pub scale_name: &'static str,
    /// Piano roll octave the notes are written in (3..=7, FL / Lemon Studio naming).
    pub octave: i32,
    /// Analysis frozen (Hold).
    pub hold: bool,
    /// Every scale note for the 808, best first (5 to 7 notes).
    pub bass: Vec<Suggestion>,
    /// The scale's notes from the root upward, for counter melodies (5 to 7 notes).
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
    OctaveDown,
    OctaveUp,
    Hold,
    Reset,
}
