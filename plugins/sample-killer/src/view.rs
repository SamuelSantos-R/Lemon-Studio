// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

use crate::model::{Hit, ViewState};
use os_ui::{Canvas, Color, Font, Rect};

pub const W: u32 = 560;
pub const H: u32 = 384;

const BG: Color = Color::rgb(0x121315);
const PANEL: Color = Color::rgb(0x1B1D20);
const EDGE: Color = Color::rgb(0x303338);
const INK: Color = Color::rgb(0xF1EDE8);
const DIM: Color = Color::rgb(0x96999F);
const RED: Color = Color::rgb(0xFF475B);
const RED_DARK: Color = Color::rgb(0x342026);
const SAGE: Color = Color::rgb(0xB9C8B9);
const SAGE_DARK: Color = Color::rgb(0x252E29);
const SMALL: Font = Font::new(10);
const LABEL: Font = Font::new(11).bold();
const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

const SCALE: Rect = Rect::new(48.0, 352.0, 192.0, 24.0);
const ROOT: Rect = Rect::new(312.0, 352.0, 40.0, 24.0);
const OCTAVE: Rect = Rect::new(448.0, 136.0, 64.0, 24.0);
const CONTROLS: [(Hit, Rect, &str); 9] = [
    (Hit::ScalePrev, Rect::new(16.0, 352.0, 32.0, 24.0), "<"),
    (Hit::ScaleNext, Rect::new(240.0, 352.0, 32.0, 24.0), ">"),
    (Hit::RootPrev, Rect::new(280.0, 352.0, 32.0, 24.0), "<"),
    (Hit::RootNext, Rect::new(352.0, 352.0, 32.0, 24.0), ">"),
    (Hit::RootAuto, Rect::new(392.0, 352.0, 56.0, 24.0), "AUTO"),
    (Hit::OctaveDown, Rect::new(416.0, 136.0, 32.0, 24.0), "<"),
    (Hit::OctaveUp, Rect::new(512.0, 136.0, 32.0, 24.0), ">"),
    (Hit::Hold, Rect::new(416.0, 8.0, 56.0, 24.0), "HOLD"),
    (Hit::Reset, Rect::new(480.0, 8.0, 64.0, 24.0), "RESET"),
];

fn unit(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn pc_name(pc: u8) -> &'static str {
    NOTES[usize::from(pc % 12)]
}

fn centered(c: &mut Canvas, f: Font, r: Rect, ink: Color, value: &str) {
    let f = c.fit_font(f, value, (r.w - 8.0).max(1.0));
    c.text_center(f, r, ink, value);
}

fn panel(c: &mut Canvas, r: Rect, fill: Color, edge: Color) {
    c.fill_rrect(r, 6.0, fill);
    c.stroke_rrect(r, 6.0, 1.0, edge);
}

fn note_rect(index: usize, count: usize, y: f32, height: f32) -> Rect {
    let width = (528.0 - (count.saturating_sub(1)) as f32 * 8.0) / count.max(1) as f32;
    Rect::new(16.0 + index as f32 * (width + 8.0), y, width, height)
}

pub fn hit(x: f32, y: f32) -> Option<Hit> {
    CONTROLS
        .iter()
        .find(|(_, r, _)| r.contains(x, y))
        .map(|(id, _, _)| *id)
}

pub fn paint(c: &mut Canvas, s: &ViewState) {
    c.begin_frame(None);
    c.clear(BG);
    c.fill_rect(Rect::new(0.0, 0.0, W as f32, 40.0), PANEL);
    c.fill_rect(Rect::new(16.0, 14.0, 3.0, 12.0), RED);
    c.fill_rect(Rect::new(22.0, 14.0, 3.0, 12.0), RED);
    c.text(Font::new(14).bold(), 34.0, 11.0, INK, "SAMPLE KILLER");
    c.text(SMALL, 304.0, 14.0, DIM, "IN");
    c.fill_rrect(Rect::new(328.0, 17.0, 64.0, 6.0), 3.0, EDGE);
    let level = unit(s.level);
    if level > 0.0 {
        c.fill_rrect(Rect::new(328.0, 17.0, 64.0 * level, 6.0), 3.0, SAGE);
    }

    if s.key_root.is_some() {
        key(c, s);
        bass(c, s);
        melody(c, s);
    } else {
        c.text(Font::new(32).bold(), 24.0, 176.0, INK, "Play your sample");
        c.text(
            Font::new(12),
            26.0,
            224.0,
            DIM,
            "Your key, 808 picks and melody space will appear here.",
        );
    }
    controls(c, s);
}

fn key(c: &mut Canvas, s: &ViewState) {
    let Some(root) = s.key_root else { return };
    let name = format!(
        "{} {}",
        pc_name(root),
        if s.key_minor { "minor" } else { "major" }
    );
    let font = c.fit_font(Font::new(48).bold(), &name, 360.0);
    c.text(font, 16.0, 48.0, INK, &name);
    if let Some(relative) = &s.relative {
        let text = format!("Relative: {relative}");
        let font = c.fit_font(Font::new(13), &text, 360.0);
        c.text(font, 18.0, 108.0, DIM, &text);
    }
    if s.root_locked {
        panel(
            c,
            Rect::new(464.0, 64.0, 80.0, 24.0),
            RED_DARK,
            RED.with_alpha(90),
        );
        centered(c, LABEL, Rect::new(464.0, 64.0, 80.0, 24.0), RED, "LOCKED");
    } else {
        c.text_right(
            SMALL,
            544.0,
            72.0,
            DIM,
            &format!("{:.0}% confidence", unit(s.confidence) * 100.0),
        );
    }
}

fn bass(c: &mut Canvas, s: &ViewState) {
    c.text(Font::new(14).bold(), 16.0, 140.0, RED, "808 NOTES");
    let count = s.bass.len().min(7);
    for (i, n) in s.bass.iter().take(count).enumerate() {
        let r = note_rect(i, count, 168.0, 64.0);
        panel(
            c,
            r,
            if n.strong { RED } else { PANEL },
            if n.strong { RED } else { EDGE },
        );
        let ink = if n.strong { BG } else { INK };
        centered(
            c,
            Font::new(if n.strong { 28 } else { 22 }).bold(),
            Rect::new(r.x, r.y + 6.0, r.w, 32.0),
            ink,
            &n.note,
        );
        centered(
            c,
            SMALL.bold(),
            Rect::new(r.x, r.y + 42.0, r.w, 14.0),
            if n.strong { BG } else { DIM },
            if n.strong { "TOP PICK" } else { n.degree },
        );
    }
}

fn melody(c: &mut Canvas, s: &ViewState) {
    c.text(Font::new(14).bold(), 16.0, 248.0, SAGE, "COUNTER MELODY");
    c.text_right(SMALL, 544.0, 252.0, DIM, "FREE = room to add notes");
    let count = s.melody.len().min(7);
    for (i, n) in s.melody.iter().take(count).enumerate() {
        let r = note_rect(i, count, 272.0, 64.0);
        let usage = unit(n.usage);
        let free = usage < 0.35;
        panel(
            c,
            r,
            if free { SAGE_DARK } else { PANEL },
            if free { SAGE.with_alpha(150) } else { EDGE },
        );
        centered(
            c,
            Font::new(20).bold(),
            Rect::new(r.x, r.y + 2.0, r.w, 26.0),
            if free { SAGE } else { INK },
            &n.note,
        );
        let tag = match (free, n.strong) {
            (true, true) => "FREE  CHORD",
            (true, false) => "FREE",
            (false, true) => "CHORD",
            (false, false) => "",
        };
        centered(
            c,
            Font::new(9).bold(),
            Rect::new(r.x, r.y + 30.0, r.w, 14.0),
            if free { SAGE } else { DIM },
            tag,
        );
        let bar = Rect::new(r.x + 8.0, r.y + 52.0, r.w - 16.0, 4.0);
        c.fill_rrect(bar, 2.0, EDGE);
        if usage > 0.0 {
            c.fill_rrect(
                Rect::new(bar.x, bar.y, bar.w * usage, bar.h),
                2.0,
                SAGE.with_alpha(150),
            );
        }
    }
}

fn controls(c: &mut Canvas, s: &ViewState) {
    c.fill_rect(Rect::new(0.0, 344.0, W as f32, 40.0), PANEL);
    centered(c, Font::new(12), SCALE, INK, s.scale_name);
    centered(
        c,
        LABEL,
        ROOT,
        INK,
        s.key_root.map(pc_name).unwrap_or("AUTO"),
    );
    centered(c, LABEL, OCTAVE, DIM, &format!("OCT {}", s.octave));
    c.text_right(SMALL, 544.0, 359.0, DIM, "SCALE / ROOT");
    for &(id, r, label) in &CONTROLS {
        let active = match id {
            Hit::RootAuto => !s.root_locked,
            Hit::Hold => s.hold,
            _ => false,
        };
        let hover = s.hover == Some(id);
        panel(
            c,
            r,
            if active {
                RED_DARK
            } else if hover {
                EDGE
            } else {
                BG
            },
            if hover {
                RED
            } else if active {
                RED.with_alpha(110)
            } else {
                EDGE
            },
        );
        centered(c, LABEL, r, if active || hover { RED } else { DIM }, label);
    }
}
