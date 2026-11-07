// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Sample Killer's editor: `Logic` turns the analysis and parameters into a `ViewState` and clicks into parameter edits;
//! a thin window per platform (the os-plugkit loop on macOS and Linux, a Win32 child window on Windows) drives it.

use std::any::Any;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::Arc;

use nih_plug::prelude::*;

use crate::model::{Hit, ViewState};
use crate::{load_f32, theory, view, KillerParams, NO_KEY};

pub struct KillerEditor {
    params: Arc<KillerParams>,
    scale: AtomicU32,
    #[cfg_attr(not(windows), allow(dead_code))]
    hwnd: Arc<AtomicIsize>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    slot: os_plugkit::gui::Slot,
}

impl KillerEditor {
    pub fn new(params: Arc<KillerParams>) -> Self {
        KillerEditor {
            params,
            scale: AtomicU32::new(1f32.to_bits()),
            hwnd: Arc::new(AtomicIsize::new(0)),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            slot: Default::default(),
        }
    }

    fn notify(&self) {
        #[cfg(windows)]
        win32::post_changed(self.hwnd.load(Ordering::Acquire));
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        os_plugkit::gui::notify(&self.slot);
    }
}

impl Editor for KillerEditor {
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        let scale = f32::from_bits(self.scale.load(Ordering::Relaxed));
        let logic = Logic::new(self.params.clone(), context);
        match parent {
            #[cfg(windows)]
            ParentWindowHandle::Win32Hwnd(hwnd) => Box::new(win32::open(hwnd, scale, logic, self.hwnd.clone())),
            #[cfg(target_os = "linux")]
            ParentWindowHandle::X11Window(window) => Box::new(unix::open(window as os_plugkit::gui::Parent, scale, logic, &self.slot)),
            #[cfg(target_os = "macos")]
            ParentWindowHandle::AppKitNsView(view) => {
                let scale = os_plugkit::gui::backing_scale(view as usize);
                Box::new(unix::open(view as usize, scale, logic, &self.slot))
            }
            _ => {
                let _ = (logic, scale);
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

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}

    fn param_values_changed(&self) {
        self.notify();
    }
}

/// Platform-neutral editor state.
pub struct Logic {
    params: Arc<KillerParams>,
    ctx: Arc<dyn GuiContext>,
    hover: Option<Hit>,
    shown: Option<ViewState>,
}

impl Logic {
    fn new(params: Arc<KillerParams>, ctx: Arc<dyn GuiContext>) -> Self {
        Logic { params, ctx, hover: None, shown: None }
    }

    pub fn state(&self) -> ViewState {
        let sh = &self.params.shared;
        let detected = sh.key_root.load(Ordering::Relaxed);
        let detected_minor = sh.key_minor.load(Ordering::Relaxed);
        let scale_i = self.params.scale.value();
        let scale = theory::resolve_scale(scale_i, detected_minor);
        let hand_root = self.params.root.value();
        let root = if hand_root > 0 {
            Some((hand_root - 1) as u8)
        } else if detected != NO_KEY {
            Some(theory::scale_root(detected as u8, detected_minor, scale))
        } else {
            None
        };
        let level = load_f32(&sh.level);
        let level = if level > 0.0 { ((20.0 * level.log10() + 60.0) / 60.0).clamp(0.0, 1.0) } else { 0.0 };
        let usage: [f32; 12] = std::array::from_fn(|i| load_f32(&sh.chroma[i]));
        let octave = self.params.octave.value();
        let relative = root.map(|r| {
            let (pc, mode) = if scale.minor { ((r + 3) % 12, "major") } else { ((r + 9) % 12, "minor") };
            format!("{} {mode}", theory::NOTE_NAMES[pc as usize])
        });
        ViewState {
            key_root: root,
            key_minor: scale.minor,
            relative,
            root_locked: hand_root > 0,
            confidence: load_f32(&sh.confidence),
            level,
            scale_name: match (scale_i, scale.minor) {
                (0, true) => "Auto (Natural Minor)",
                (0, false) => "Auto (Major)",
                _ => scale.name,
            },
            octave,
            hold: self.params.hold.value(),
            bass: root.map(|r| theory::bass_notes(r, scale, octave, &usage)).unwrap_or_default(),
            melody: root.map(|r| theory::melody_notes(r, scale, octave, &usage)).unwrap_or_default(),
            hover: self.hover,
        }
    }

    /// Whether the picture changed since the last `paint`.
    pub fn stale(&self) -> bool {
        self.shown.as_ref() != Some(&self.state())
    }

    pub fn paint(&mut self, c: &mut os_ui::Canvas) {
        let s = self.state();
        view::paint(c, &s);
        self.shown = Some(s);
    }

    /// Mouse at logical (x, y); true when the hover changed.
    pub fn moved(&mut self, x: f32, y: f32) -> bool {
        let h = view::hit(x, y);
        std::mem::replace(&mut self.hover, h) != h
    }

    pub fn leave(&mut self) -> bool {
        self.hover.take().is_some()
    }

    pub fn click(&mut self, x: f32, y: f32) {
        let Some(hit) = view::hit(x, y) else { return };
        let p = &self.params;
        match hit {
            Hit::ScalePrev | Hit::ScaleNext => {
                let step = if hit == Hit::ScaleNext { 1 } else { -1 };
                self.set_int(&p.scale, (p.scale.value() + step).rem_euclid(theory::SCALE_COUNT));
            }
            Hit::RootPrev | Hit::RootNext => {
                let step = if hit == Hit::RootNext { 1 } else { -1 };
                let cur = self.state().key_root.map_or(if step > 0 { 11 } else { 1 }, i32::from);
                self.set_int(&p.root, (cur + step).rem_euclid(12) + 1);
            }
            Hit::RootAuto => self.set_int(&p.root, 0),
            Hit::OctaveDown => self.set_int(&p.octave, (p.octave.value() - 1).max(3)),
            Hit::OctaveUp => self.set_int(&p.octave, (p.octave.value() + 1).min(7)),
            Hit::Hold => {
                let setter = ParamSetter::new(self.ctx.as_ref());
                setter.begin_set_parameter(&p.hold);
                setter.set_parameter(&p.hold, !p.hold.value());
                setter.end_set_parameter(&p.hold);
            }
            Hit::Reset => p.shared.reset.store(true, Ordering::Relaxed),
        }
    }

    fn set_int(&self, param: &IntParam, v: i32) {
        let setter = ParamSetter::new(self.ctx.as_ref());
        setter.begin_set_parameter(param);
        setter.set_parameter(param, v);
        setter.end_set_parameter(param);
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use os_plugkit::gui::{self as px};
    use os_ui::Canvas;

    use super::Logic;
    use crate::view;

    pub fn open(parent: px::Parent, scale: f32, logic: Logic, slot: &px::Slot) -> Option<px::EditorThread> {
        let scale = if scale > 0.0 { scale } else { 1.0 };
        let (dw, dh) = ((view::W as f32 * scale).round() as u32, (view::H as f32 * scale).round() as u32);
        px::spawn(parent, (dw, dh), slot, move |win| {
            let Ok(canvas) = Canvas::new(dw, dh, scale) else { return };
            Ed { win, canvas, buf: vec![0; (dw * dh) as usize], scale, size: (dw, dh), dirty: true, logic }.run();
        })
    }

    struct Ed {
        win: px::Window,
        canvas: Canvas,
        buf: Vec<u32>,
        scale: f32,
        size: (u32, u32),
        dirty: bool,
        logic: Logic,
    }

    impl Ed {
        fn run(mut self) {
            while let Some(ev) = self.win.next() {
                use px::Input::*;
                match ev {
                    Expose => self.dirty = true,
                    Tick => self.dirty |= self.logic.stale(),
                    Down { x, y, .. } => {
                        self.logic.click(x / self.scale, y / self.scale);
                        self.dirty = true;
                    }
                    Move { x, y } => self.dirty |= self.logic.moved(x / self.scale, y / self.scale),
                    Leave => self.dirty |= self.logic.leave(),
                    _ => {}
                }
                if self.dirty && self.win.idle() {
                    self.dirty = false;
                    self.logic.paint(&mut self.canvas);
                    self.canvas.copy_to_xrgb(&mut self.buf);
                    self.win.blit(&self.buf, self.size.0, self.size.1);
                }
            }
        }
    }
}

#[cfg(windows)]
mod win32 {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::Arc;

    use os_plugkit::win as pw;
    use os_ui::Canvas;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    use super::Logic;
    use crate::view;

    const TIMER: usize = 1;
    const WM_CHANGED: u32 = WM_APP + 1;

    struct Win {
        canvas: Canvas,
        buf: Vec<u32>,
        scale: f32,
        size: (u32, u32),
        logic: Logic,
        tracking: bool,
        slot: Arc<AtomicIsize>,
    }

    /// Owns the window; dropping it closes the editor.
    pub struct Handle(isize, Arc<AtomicIsize>);

    // SAFETY: only an HWND value; the host drops the editor handle on its GUI thread.
    unsafe impl Send for Handle {}

    impl Drop for Handle {
        fn drop(&mut self) {
            self.1.store(0, Ordering::Release);
            unsafe { DestroyWindow(self.0 as HWND) };
        }
    }

    pub fn post_changed(hwnd: isize) {
        if hwnd != 0 {
            unsafe { PostMessageW(hwnd as HWND, WM_CHANGED, 0, 0) };
        }
    }

    pub fn open(parent: *mut c_void, scale: f32, logic: Logic, slot: Arc<AtomicIsize>) -> Handle {
        let scale = if scale > 0.0 { scale } else { 1.0 };
        let size = ((view::W as f32 * scale).round() as u32, (view::H as f32 * scale).round() as u32);
        let win = Box::new(Win {
            canvas: Canvas::new(size.0, size.1, scale).expect("canvas"),
            buf: vec![0; (size.0 * size.1) as usize],
            scale,
            size,
            logic,
            tracking: false,
            slot: slot.clone(),
        });
        unsafe {
            let class = pw::register("SampleKillerView", Some(wndproc));
            let title = pw::wide("Sample Killer");
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                0,
                0,
                size.0 as i32,
                size.1 as i32,
                parent as HWND,
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                Box::into_raw(win) as *const c_void,
            );
            slot.store(hwnd as isize, Ordering::Release);
            SetTimer(hwnd, TIMER, 33, None);
            Handle(hwnd as isize, slot)
        }
    }

    unsafe fn state(hwnd: HWND) -> Option<&'static mut Win> {
        let p = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut Win;
        if p.is_null() { None } else { Some(unsafe { &mut *p }) }
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe {
            if msg == WM_NCCREATE {
                let cs = lp as *const CREATESTRUCTW;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            if msg == WM_NCDESTROY {
                let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Win;
                if !p.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    (*p).slot.store(0, Ordering::Release);
                    drop(Box::from_raw(p));
                }
                return DefWindowProcW(hwnd, msg, wp, lp);
            }
            let Some(w) = state(hwnd) else { return DefWindowProcW(hwnd, msg, wp, lp) };
            let at = |w: &Win| {
                let (x, y) = pw::xy(lp);
                (x as f32 / w.scale, y as f32 / w.scale)
            };
            match msg {
                WM_PAINT => {
                    let (hdc, ps) = pw::begin_paint(hwnd);
                    w.logic.paint(&mut w.canvas);
                    w.canvas.copy_to_xrgb(&mut w.buf);
                    pw::blit(hdc, &w.buf, w.size.0, w.size.1);
                    pw::end_paint(hwnd, &ps);
                    0
                }
                WM_ERASEBKGND => 1,
                WM_TIMER | WM_CHANGED => {
                    if w.logic.stale() {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    0
                }
                WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                    let (x, y) = at(w);
                    w.logic.click(x, y);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                    0
                }
                WM_MOUSEMOVE => {
                    if !w.tracking {
                        let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                        TrackMouseEvent(&mut tme);
                        w.tracking = true;
                    }
                    let (x, y) = at(w);
                    if w.logic.moved(x, y) {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    0
                }
                WM_MOUSELEAVE => {
                    w.tracking = false;
                    if w.logic.leave() {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    0
                }
                _ => DefWindowProcW(hwnd, msg, wp, lp),
            }
        }
    }
}
