// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

use crate::model::{Hit, ViewState};
use os_ui::{Canvas, Color, Font, Rect};

pub const W: u32 = 560;
pub const H: u32 = 400;

const BG: Color = Color::rgb(0x121315);
const PANEL: Color = Color::rgb(0x1B1D20);
const EDGE: Color = Color::rgb(0x303338);
const INK: Color = Color::rgb(0xF1EDE8);
const DIM: Color = Color::rgb(0x96999F);
const FAINT: Color = Color::rgb(0x62666D);
const RED: Color = Color::rgb(0xFF475B);
const RED_DARK: Color = Color::rgb(0x342026);
const MELODY: Color = Color::rgb(0xB9C8B9);
const SMALL: Font = Font::new(10);
const LABEL: Font = Font::new(11).bold();
const BODY: Font = Font::new(12);
const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

const SCALE_NAME: Rect = Rect::new(44.0, 360.0, 164.0, 28.0);
const ROOT_NAME: Rect = Rect::new(292.0, 360.0, 30.0, 28.0);
const CONTROLS: [(Hit, Rect, &str); 7] = [
    (Hit::ScalePrev, Rect::new(16.0, 360.0, 28.0, 28.0), "<"),
    (Hit::ScaleNext, Rect::new(208.0, 360.0, 28.0, 28.0), ">"),
    (Hit::RootPrev, Rect::new(264.0, 360.0, 28.0, 28.0), "<"),
    (Hit::RootNext, Rect::new(322.0, 360.0, 28.0, 28.0), ">"),
    (Hit::RootAuto, Rect::new(354.0, 360.0, 50.0, 28.0), "AUTO"),
    (Hit::Hold, Rect::new(424.0, 360.0, 56.0, 28.0), "HOLD"),
    (Hit::Reset, Rect::new(488.0, 360.0, 56.0, 28.0), "RESET"),
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

// Canvas uses cell-top coordinates; keep layout text positions as baselines.
fn text(c: &mut Canvas, f: Font, x: f32, baseline: f32, ink: Color, value: &str) {
    let top = baseline - c.ascent(f);
    c.text(f, x, top, ink, value);
}

fn right(c: &mut Canvas, f: Font, x: f32, baseline: f32, ink: Color, value: &str) {
    let top = baseline - c.ascent(f);
    c.text_right(f, x, top, ink, value);
}

fn centered(c: &mut Canvas, f: Font, r: Rect, ink: Color, value: &str) {
    let f = c.fit_font(f, value, r.w - 8.0);
    c.text_center(f, r, ink, value);
}

fn panel(c: &mut Canvas, r: Rect, fill: Color, edge: Color) {
    c.fill_rrect(r, 6.0, fill);
    c.stroke_rrect(r, 6.0, 1.0, edge);
}

/// Which control is at logical point (x, y), if any.
pub fn hit(x: f32, y: f32) -> Option<Hit> {
    CONTROLS
        .iter()
        .find(|(_, r, _)| r.contains(x, y))
        .map(|(id, _, _)| *id)
}

/// Paints a full frame (call c.begin_frame(None) and c.clear(..) first).
pub fn paint(c: &mut Canvas, s: &ViewState) {
    c.begin_frame(None);
    c.clear(BG);
    c.fill_rect(Rect::new(0.0, 0.0, W as f32, 32.0), PANEL);
    c.fill_rect(Rect::new(0.0, 31.0, W as f32, 1.0), EDGE);
    c.fill_rect(Rect::new(16.0, 10.0, 3.0, 12.0), RED);
    c.fill_rect(Rect::new(22.0, 10.0, 3.0, 12.0), RED);
    text(c, Font::new(14).bold(), 34.0, 22.0, INK, "SAMPLE KILLER");

    key(c, s);
    live(c, s);
    bass(c, s);
    melody(c, s);
    chroma(c, s);
    controls(c, s);
}

fn key(c: &mut Canvas, s: &ViewState) {
    text(c, LABEL, 16.0, 52.0, DIM, "SAMPLE KEY");
    if s.root_locked {
        panel(
            c,
            Rect::new(244.0, 39.0, 68.0, 18.0),
            RED_DARK,
            RED.with_alpha(90),
        );
        centered(
            c,
            SMALL.bold(),
            Rect::new(244.0, 39.0, 68.0, 18.0),
            RED,
            "LOCKED",
        );
    } else {
        let conf = unit(s.confidence);
        right(
            c,
            SMALL,
            312.0,
            51.0,
            DIM,
            &format!("{:.0}% CONF", conf * 100.0),
        );
        c.fill_rrect(Rect::new(172.0, 46.0, 48.0, 3.0), 1.5, EDGE);
        if conf > 0.0 {
            c.fill_rrect(Rect::new(172.0, 46.0, 48.0 * conf, 3.0), 1.5, RED);
        }
    }
    if let Some(root) = s.key_root {
        let name = format!(
            "{} {}",
            pc_name(root),
            if s.key_minor { "minor" } else { "major" }
        );
        let f = c.fit_font(Font::new(38).bold(), &name, 300.0);
        text(c, f, 14.0, 91.0, INK, &name);
        text(c, BODY, 17.0, 110.0, DIM, s.scale_name);
    } else {
        text(c, Font::new(28).bold(), 15.0, 88.0, INK, "Play your sample");
        text(c, BODY, 17.0, 109.0, DIM, "Listening for a tonal center");
    }
}

fn live(c: &mut Canvas, s: &ViewState) {
    c.fill_rect(Rect::new(328.0, 44.0, 1.0, 64.0), EDGE);
    text(c, LABEL, 344.0, 52.0, DIM, "LIVE NOTE");
    if let Some(midi) = s.live_midi {
        let name = format!("{}{}", pc_name(midi), i16::from(midi) / 12 - 1);
        text(c, Font::new(24).bold(), 344.0, 79.0, INK, &name);
        right(
            c,
            SMALL,
            496.0,
            75.0,
            DIM,
            &format!("{:.1} Hz", s.live_freq),
        );
    } else {
        text(c, Font::new(24).bold(), 344.0, 79.0, FAINT, "--");
        right(c, SMALL, 496.0, 75.0, FAINT, "-- Hz");
    }
    let (x, y, width) = (348.0, 91.0, 144.0);
    c.fill_rect(Rect::new(x, y, width, 2.0), EDGE);
    for i in 0..=4 {
        c.fill_rect(
            Rect::new(x + i as f32 * width / 4.0, y - 3.0, 1.0, 8.0),
            FAINT,
        );
    }
    text(c, SMALL, 344.0, 111.0, FAINT, "-50");
    centered(c, SMALL, Rect::new(397.0, 99.0, 48.0, 14.0), DIM, "CENTS");
    right(c, SMALL, 497.0, 111.0, FAINT, "+50");
    if s.live_midi.is_some() {
        let cents = if s.live_cents.is_finite() {
            s.live_cents.clamp(-50.0, 50.0)
        } else {
            0.0
        };
        let at = x + (cents + 50.0) / 100.0 * width;
        c.fill_rrect(Rect::new(at - 2.0, y - 4.0, 4.0, 10.0), 2.0, RED);
    }
    text(c, SMALL, 516.0, 52.0, DIM, "IN");
    for i in 0..10 {
        let on = unit(s.level) > i as f32 / 10.0;
        c.fill_rect(
            Rect::new(520.0, 102.0 - i as f32 * 4.5, 12.0, 3.0),
            if on { RED } else { EDGE },
        );
    }
}

fn bass(c: &mut Canvas, s: &ViewState) {
    text(c, Font::new(13).bold(), 16.0, 135.0, RED, "808 NOTES");
    right(c, SMALL, 544.0, 134.0, DIM, "ACCENTED PADS = FIRST PICKS");
    for i in 0..5 {
        let r = Rect::new(16.0 + i as f32 * 107.0, 144.0, 100.0, 72.0);
        let note = s.bass.get(i);
        let strong = note.is_some_and(|n| n.strong);
        panel(
            c,
            r,
            if strong { RED_DARK } else { PANEL },
            if strong { RED.with_alpha(150) } else { EDGE },
        );
        if let Some(n) = note {
            let col = if strong { RED } else { INK };
            let f = c.fit_font(Font::new(29).bold(), &n.note, 78.0);
            text(c, f, r.x + 10.0, r.y + 32.0, col, &n.note);
            text(c, LABEL, r.x + 11.0, r.y + 50.0, col, n.degree);
            text(
                c,
                SMALL,
                r.x + 11.0,
                r.y + 64.0,
                DIM,
                &format!("{:.1} Hz", n.freq),
            );
            if strong {
                c.fill_circle(r.right() - 10.0, r.y + 11.0, 2.0, RED);
            }
        } else {
            centered(c, Font::new(24), r, FAINT, "--");
        }
    }
}

fn melody(c: &mut Canvas, s: &ViewState) {
    text(c, LABEL, 16.0, 237.0, MELODY, "MELODY NOTES");
    right(c, SMALL, 544.0, 236.0, DIM, "BRIGHT = CHORD TONES");
    let count = s.melody.len().clamp(5, 7);
    let width = (528.0 - (count - 1) as f32 * 8.0) / count as f32;
    for i in 0..count {
        let r = Rect::new(16.0 + i as f32 * (width + 8.0), 244.0, width, 36.0);
        let note = s.melody.get(i);
        let strong = note.is_some_and(|n| n.strong);
        panel(
            c,
            r,
            PANEL,
            if strong { MELODY.with_alpha(125) } else { EDGE },
        );
        if let Some(n) = note {
            let col = if strong { MELODY } else { DIM };
            centered(
                c,
                Font::new(16).bold(),
                Rect::new(r.x, r.y + 1.0, r.w, 20.0),
                col,
                pc_name(n.pc),
            );
            centered(
                c,
                SMALL,
                Rect::new(r.x, r.y + 21.0, r.w, 13.0),
                col,
                n.degree,
            );
        } else {
            centered(c, BODY, r, FAINT, "--");
        }
    }
}

fn chroma(c: &mut Canvas, s: &ViewState) {
    text(c, SMALL.bold(), 16.0, 297.0, DIM, "PITCH ENERGY");
    right(c, SMALL, 544.0, 297.0, FAINT, "DOT / SCALE    RED / ROOT");
    for (i, name) in NOTES.iter().enumerate() {
        let x = 16.0 + i as f32 * 44.0;
        let root = s.key_root.is_some_and(|r| usize::from(r % 12) == i);
        let in_scale = s.melody.iter().any(|n| usize::from(n.pc % 12) == i);
        let height = unit(s.chroma[i]) * 24.0;
        c.fill_rrect(Rect::new(x, 304.0, 36.0, 24.0), 3.0, PANEL);
        if height > 0.0 {
            c.fill_rrect(
                Rect::new(x, 328.0 - height, 36.0, height),
                2.0,
                if root {
                    RED
                } else if in_scale {
                    MELODY.with_alpha(150)
                } else {
                    FAINT
                },
            );
        }
        if root {
            c.stroke_rrect(Rect::new(x, 304.0, 36.0, 24.0), 3.0, 1.0, RED);
        }
        centered(
            c,
            SMALL.bold(),
            Rect::new(x, 329.0, 36.0, 15.0),
            if root { RED } else { DIM },
            name,
        );
        if in_scale {
            c.fill_circle(x + 31.0, 337.0, 1.5, if root { RED } else { MELODY });
        }
    }
}

fn controls(c: &mut Canvas, s: &ViewState) {
    c.fill_rect(Rect::new(0.0, 352.0, W as f32, 48.0), PANEL);
    c.fill_rect(Rect::new(0.0, 352.0, W as f32, 1.0), EDGE);
    c.fill_rrect(Rect::new(16.0, 360.0, 220.0, 28.0), 5.0, BG);
    let scale = format!("Scale: {}", s.scale_name);
    centered(c, BODY, SCALE_NAME, INK, &scale);
    centered(
        c,
        LABEL,
        ROOT_NAME,
        INK,
        s.key_root.map(pc_name).unwrap_or("--"),
    );
    for &(id, r, label) in &CONTROLS {
        let active = match id {
            Hit::RootAuto => !s.root_locked,
            Hit::Hold => s.hold,
            _ => false,
        };
        let hover = s.hover == Some(id);
        let fill = if active {
            RED_DARK
        } else if hover {
            EDGE
        } else {
            BG
        };
        panel(
            c,
            r,
            fill,
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
