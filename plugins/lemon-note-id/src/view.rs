// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Note ID editor view: piano keyboard with colour-coded note highlights.

use os_ui::{Canvas, Color, Font, Rect};

use crate::theory;

// ── Window ──────────────────────────────────────────────────────────────────
pub const W: u32 = 290;
pub const H: u32 = 232;

// ── Fonts ───────────────────────────────────────────────────────────────────
const F_TITLE: Font = Font::condensed(9, 0).bold();
const F_NOTE_BIG: Font = Font::condensed(26, 0).bold();
const F_NOTE_SMALL: Font = Font::new(10);
const F_LABEL: Font = Font::new(9).bold();
const F_BODY: Font = Font::new(9);

// ── Colours ──────────────────────────────────────────────────────────────────
const BG: Color = Color::rgb(0x17171E);
const CARD: Color = Color::rgb(0x22222C);
const BORDER: Color = Color::rgb(0x38384E);
const TEXT_HI: Color = Color::rgb(0xE0E0EE);
const TEXT_DIM: Color = Color::rgb(0x666688);
const C_DETECT: Color = Color::rgb(0xFFD600); // yellow – detected note
const C_BASS: Color = Color::rgb(0xFF7A00);   // orange – 808 bass 4th/5th
const C_SCALE: Color = Color::rgb(0x00C4DC);  // cyan   – scale melody notes
const KEY_WHITE: Color = Color::rgb(0xCCCCC4);
const KEY_BLACK: Color = Color::rgb(0x1E1E28);
const KEY_SEP: Color = Color::rgb(0x444455);

// ── Piano keyboard layout ────────────────────────────────────────────────────
// 7 white keys × 35 px = 245 px; centred in 290 px → left margin = 22.5 → 23 px.
const KB_X: f32 = 23.0;
const KB_Y: f32 = 68.0;
const WK_W: f32 = 35.0;
const WK_H: f32 = 86.0;
const BK_W: f32 = 22.0;
const BK_H: f32 = 54.0;

// (pitch-class, x offset from KB_X)
const WHITE_KEYS: [(u8, f32); 7] = [
    (0, 0.0),   // C
    (2, 35.0),  // D
    (4, 70.0),  // E
    (5, 105.0), // F
    (7, 140.0), // G
    (9, 175.0), // A
    (11, 210.0),// B
];
const BLACK_KEYS: [(u8, f32); 5] = [
    (1, 24.0),  // C#  (C offset + WK_W – BK_W/2)
    (3, 59.0),  // D#
    (6, 129.0), // F#
    (8, 164.0), // G#
    (10, 199.0),// A#
];

/// Clickable area for the scale selector (logical px).
pub const SCALE_RECT: Rect = Rect::new(10.0, 203.0, 270.0, 22.0);

// ── Entry point ──────────────────────────────────────────────────────────────
/// Paint the full editor frame.
/// * `midi` – detected MIDI note (255 = no signal)
/// * `freq` – detected frequency in Hz (0.0 = no signal)
/// * `scale_i` – selected scale index (0–4)
pub fn paint(c: &mut Canvas, midi: u8, freq: f32, scale_i: i32) {
    c.begin_frame(None);
    c.clear(BG);

    // Title bar
    let title_r = Rect::new(0.0, 0.0, W as f32, 22.0);
    c.fill_rect(title_r, CARD);
    c.fill_rect(Rect::new(0.0, 21.0, W as f32, 1.0), BORDER);
    c.text(F_TITLE, 10.0, 14.0, TEXT_DIM, "LEMON NOTE ID");

    // ── Detected note display ──────────────────────────────────────────────
    if midi < 255 {
        let pc = theory::midi_to_pc(midi);
        let oct = theory::midi_to_octave(midi);
        let name = theory::NOTE_NAMES[pc as usize];
        let note_str = format!("{}{}", name, oct);
        let freq_str = format!("{:.1} Hz", freq);

        let nw = c.text_width(F_NOTE_BIG, &note_str);
        let fw = c.text_width(F_NOTE_SMALL, &freq_str);
        let total = nw + 8.0 + fw;
        let x0 = (W as f32 - total) / 2.0;
        c.text(F_NOTE_BIG, x0, 52.0, C_DETECT, &note_str);
        c.text(F_NOTE_SMALL, x0 + nw + 8.0, 50.0, TEXT_DIM, &freq_str);

        let root_pc = pc;
        let bass = theory::bass_pcs(root_pc);
        let scale = theory::scale_pcs(root_pc, scale_i);
        draw_keyboard(c, Some(root_pc), &bass, &scale);
        draw_legend(c);
        draw_note_lists(c, root_pc, &bass, &scale, scale_i);
    } else {
        c.text_center(F_NOTE_SMALL, Rect::new(0.0, 30.0, W as f32, 28.0), TEXT_DIM, "— no signal —");
        draw_keyboard(c, None, &[false; 12], &[false; 12]);
        draw_legend(c);
    }

    // ── Scale selector ────────────────────────────────────────────────────
    let sr = SCALE_RECT;
    c.fill_rect(sr, CARD);
    c.stroke_rrect(sr, 3.0, 1.0, BORDER);
    let name = theory::SCALE_NAMES[scale_i.clamp(0, 4) as usize];
    let label = format!("Escala: {}  ▾", name);
    c.text_center(F_LABEL, sr, TEXT_HI, &label);
}

// ── Piano keyboard ───────────────────────────────────────────────────────────
fn key_color(pc: u8, is_black: bool, detected: Option<u8>, bass: &[bool; 12], scale: &[bool; 12]) -> Color {
    if detected == Some(pc) {
        C_DETECT
    } else if bass[pc as usize] {
        C_BASS
    } else if scale[pc as usize] {
        C_SCALE
    } else if is_black {
        KEY_BLACK
    } else {
        KEY_WHITE
    }
}

fn draw_keyboard(c: &mut Canvas, detected: Option<u8>, bass: &[bool; 12], scale: &[bool; 12]) {
    // White keys first
    for &(pc, ox) in &WHITE_KEYS {
        let r = Rect::new(KB_X + ox, KB_Y, WK_W - 1.0, WK_H);
        let col = key_color(pc, false, detected, bass, scale);
        c.fill_rect(r, col);
        c.fill_rect(Rect::new(KB_X + ox + WK_W - 1.0, KB_Y, 1.0, WK_H), KEY_SEP);
    }
    // Keyboard outline
    c.stroke_rrect(Rect::new(KB_X - 1.0, KB_Y - 1.0, WK_W * 7.0 + 2.0, WK_H + 2.0), 2.0, 1.0, BORDER);

    // Black keys on top
    for &(pc, ox) in &BLACK_KEYS {
        let r = Rect::new(KB_X + ox, KB_Y, BK_W, BK_H);
        let col = key_color(pc, true, detected, bass, scale);
        c.fill_rect(r, col);
        c.stroke_rrect(r, 1.0, 1.0, KEY_SEP);
    }

    // Note names on highlighted keys
    for &(pc, ox) in &WHITE_KEYS {
        if detected == Some(pc) || bass[pc as usize] || scale[pc as usize] {
            let lx = KB_X + ox + WK_W / 2.0 - c.text_width(F_BODY, theory::NOTE_NAMES[pc as usize]) / 2.0;
            let ly = KB_Y + WK_H - 10.0;
            let tc = if detected == Some(pc) { BG } else { BG };
            c.text(F_BODY, lx, ly, tc, theory::NOTE_NAMES[pc as usize]);
        }
    }
    for &(pc, ox) in &BLACK_KEYS {
        if detected == Some(pc) || bass[pc as usize] || scale[pc as usize] {
            let lx = KB_X + ox + BK_W / 2.0 - c.text_width(F_BODY, theory::NOTE_NAMES[pc as usize]) / 2.0;
            let ly = KB_Y + BK_H - 8.0;
            c.text(F_BODY, lx, ly, BG, theory::NOTE_NAMES[pc as usize]);
        }
    }
}

fn draw_legend(c: &mut Canvas) {
    let items = [
        (C_DETECT, "Detectada"),
        (C_BASS,   "808 (4ª/5ª)"),
        (C_SCALE,  "Melodia"),
    ];
    let mut x = 10.0_f32;
    let y = 161.0_f32;
    for (col, label) in &items {
        c.fill_rect(Rect::new(x, y - 7.0, 8.0, 8.0), *col);
        x += 11.0;
        x += c.text(F_BODY, x, y, TEXT_DIM, label) + 10.0;
    }
}

fn draw_note_lists(c: &mut Canvas, root: u8, bass: &[bool; 12], scale: &[bool; 12], _scale_i: i32) {
    // 808 bass line
    let bass_notes: Vec<&str> = (0u8..12)
        .filter(|&pc| bass[pc as usize] || pc == root)
        .map(|pc| theory::NOTE_NAMES[pc as usize])
        .collect();
    let bass_str = bass_notes.join("  ");

    // Scale melody notes (excluding root and bass companions)
    let mel_notes: Vec<&str> = (0u8..12)
        .filter(|&pc| scale[pc as usize] && pc != root && !bass[pc as usize])
        .map(|pc| theory::NOTE_NAMES[pc as usize])
        .collect();
    let mel_str = mel_notes.join("  ");

    c.text(F_LABEL, 10.0, 180.0, C_BASS, "808:");
    c.text(F_BODY, 38.0, 180.0, TEXT_HI, &bass_str);

    c.text(F_LABEL, 10.0, 194.0, C_SCALE, "Melodia:");
    c.text(F_BODY, 56.0, 194.0, TEXT_HI, &mel_str);
}
