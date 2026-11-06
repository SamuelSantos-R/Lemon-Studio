// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Note ID editor: child window that paints the piano-keyboard view, following the same
//! Editor / os-plugkit pattern used by Tesla and the small effects.

use std::any::Any;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::Arc;

use nih_plug::prelude::*;

use crate::{view, NoteIdParams};

pub struct NoteIdEditor {
    pub params: Arc<NoteIdParams>,
    /// DPI scale as f32 bits (1.0 until set_scale_factor is called).
    pub scale: AtomicU32,
    /// Open Win32 HWND; 0 = closed. Used for Win32 repaint posts.
    #[cfg_attr(not(windows), allow(dead_code))]
    hwnd: Arc<AtomicIsize>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    slot: os_plugkit::gui::Slot,
}

impl NoteIdEditor {
    pub fn new(params: Arc<NoteIdParams>) -> Self {
        NoteIdEditor {
            params,
            scale: AtomicU32::new(1f32.to_bits()),
            hwnd: Arc::new(AtomicIsize::new(0)),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            slot: Default::default(),
        }
    }

    fn notify(&self) {
        #[cfg(windows)]
        {
            // Post WM_USER to the editor HWND so it repaints (see win32 module).
            let hwnd = self.hwnd.load(Ordering::Acquire);
            if hwnd != 0 {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::PostMessageA(hwnd as _, 0x0400, 0, 0);
                }
            }
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        os_plugkit::gui::notify(&self.slot);
    }
}

impl Editor for NoteIdEditor {
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        let scale = f32::from_bits(self.scale.load(Ordering::Relaxed));
        match parent {
            #[cfg(windows)]
            ParentWindowHandle::Win32Hwnd(_hwnd) => Box::new(()), // TODO: Win32 editor
            #[cfg(target_os = "linux")]
            ParentWindowHandle::X11Window(window) => {
                Box::new(unix::open(window, scale, self.params.clone(), context, &self.slot))
            }
            #[cfg(target_os = "macos")]
            ParentWindowHandle::AppKitNsView(view) => {
                let scale = os_plugkit::gui::backing_scale(view as usize);
                Box::new(unix::open(view as usize, scale, self.params.clone(), context, &self.slot))
            }
            _ => {
                let _ = (context, scale);
                Box::new(())
            }
        }
    }

    fn size(&self) -> (u32, u32) {
        (view::W, view::H)
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        self.scale.store(factor.to_bits(), Ordering::Relaxed);
        true
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        self.notify();
    }
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        self.notify();
    }
    fn param_values_changed(&self) {
        self.notify();
    }
}

// ── macOS / Linux editor thread ──────────────────────────────────────────────
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use nih_plug::prelude::{GuiContext, Param};
    use os_plugkit::gui::{self as px};
    use os_ui::Canvas;

    use crate::{theory, view, NoteIdParams};

    pub fn open(
        parent: px::Parent,
        scale: f32,
        params: Arc<NoteIdParams>,
        ctx: Arc<dyn GuiContext>,
        slot: &px::Slot,
    ) -> Option<px::EditorThread> {
        let scale = if scale > 0.0 { scale } else { 1.0 };
        let dw = (view::W as f32 * scale).round() as u32;
        let dh = (view::H as f32 * scale).round() as u32;
        px::spawn(parent, (dw, dh), slot, move |win| {
            let Ok(canvas) = Canvas::new(dw, dh, scale) else { return };
            Ed {
                win,
                canvas,
                buf: Vec::new(),
                scale,
                dw,
                dh,
                dirty: true,
                last_midi: 255,
                last_freq_bits: 0,
                params,
                ctx,
            }
            .run();
        })
    }

    struct Ed {
        win: px::Window,
        canvas: Canvas,
        buf: Vec<u32>,
        scale: f32,
        dw: u32,
        dh: u32,
        dirty: bool,
        last_midi: u8,
        last_freq_bits: u32,
        params: Arc<NoteIdParams>,
        ctx: Arc<dyn GuiContext>,
    }

    impl Ed {
        fn run(mut self) {
            while let Some(ev) = self.win.next() {
                use px::Input::*;
                match ev {
                    Expose => self.dirty = true,
                    Tick => self.tick(),
                    Down { x, y, .. } => self.click(x, y),
                    _ => {}
                }
                if self.dirty && self.win.idle() {
                    self.repaint();
                }
            }
        }

        fn tick(&mut self) {
            let midi = self.params.shared.midi_note.load(Ordering::Relaxed);
            let fb = self.params.shared.freq_bits.load(Ordering::Relaxed);
            if midi != self.last_midi || fb != self.last_freq_bits {
                self.last_midi = midi;
                self.last_freq_bits = fb;
                self.dirty = true;
            }
        }

        fn click(&mut self, px_x: f32, px_y: f32) {
            let lx = px_x / self.scale;
            let ly = px_y / self.scale;
            if view::SCALE_RECT.contains(lx, ly) {
                let cur = self.params.scale.value();
                let next = (cur + 1).rem_euclid(theory::SCALE_COUNT);
                // SAFETY: `scale` is a live param in a pinned Arc; normalized is in [0,1].
                unsafe { self.ctx.raw_set_parameter_normalized(self.params.scale.as_ptr(), self.params.scale.preview_normalized(next)) };
                self.dirty = true;
            }
        }

        fn repaint(&mut self) {
            self.dirty = false;
            let midi = self.last_midi;
            let freq = f32::from_bits(self.last_freq_bits);
            let scale_i = self.params.scale.value();
            view::paint(&mut self.canvas, midi, freq, scale_i);
            self.buf.resize((self.dw * self.dh) as usize, 0);
            self.canvas.copy_to_xrgb(&mut self.buf);
            self.win.blit(&self.buf, self.dw, self.dh);
        }
    }
}
