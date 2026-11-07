// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Sample Killer: listens to a sample or loop, works out its key from the pitch classes heard over time, and suggests the
//! 808 notes and melody notes that fit, in the scale chosen (trap scales, or Auto). Audio passes through untouched.
#![allow(semicolon_in_expressions_from_macros)]

pub mod dsp;
pub mod editor;
pub mod model;
pub mod theory;
pub mod view;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use nih_plug::prelude::*;

/// Analysis results, written by the audio thread and read by the editor.
pub struct Shared {
    /// Accumulated chroma, largest = 1.0 (f32 bits).
    pub chroma: [AtomicU32; 12],
    /// Detected key root 0..11, `NO_KEY` before enough audio.
    pub key_root: AtomicU32,
    pub key_minor: AtomicBool,
    pub confidence: AtomicU32,
    /// Peak input level, decaying (f32 bits).
    pub level: AtomicU32,
    /// Set by the editor's Reset; the audio thread clears what it heard and lowers it.
    pub reset: AtomicBool,
}

pub const NO_KEY: u32 = u32::MAX;

impl Default for Shared {
    fn default() -> Self {
        Shared {
            chroma: std::array::from_fn(|_| AtomicU32::new(0)),
            key_root: AtomicU32::new(NO_KEY),
            key_minor: AtomicBool::new(true),
            confidence: AtomicU32::new(0),
            level: AtomicU32::new(0),
            reset: AtomicBool::new(false),
        }
    }
}

pub fn load_f32(a: &AtomicU32) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

fn store_f32(a: &AtomicU32, v: f32) {
    a.store(v.to_bits(), Ordering::Relaxed);
}

pub struct KillerParams {
    /// Index into `theory::SCALES` (0 = Auto).
    pub scale: IntParam,
    /// 0 = detected; 1..=12 = C..B set by hand.
    pub root: IntParam,
    /// Piano roll octave the 808 and melody notes are written in (FL / Lemon Studio naming, C5 = middle C).
    pub octave: IntParam,
    /// Freezes what was heard: the key stops moving.
    pub hold: BoolParam,
    pub shared: Arc<Shared>,
}

impl Default for KillerParams {
    fn default() -> Self {
        KillerParams {
            scale: IntParam::new("Scale", 0, IntRange::Linear { min: 0, max: theory::SCALE_COUNT - 1 })
                .with_value_to_string(Arc::new(|v| theory::SCALES[v.clamp(0, theory::SCALE_COUNT - 1) as usize].name.to_string())),
            root: IntParam::new("Root", 0, IntRange::Linear { min: 0, max: 12 })
                .with_value_to_string(Arc::new(|v| if v <= 0 { "Auto".into() } else { theory::NOTE_NAMES[(v - 1).clamp(0, 11) as usize].into() })),
            octave: IntParam::new("Octave", 5, IntRange::Linear { min: 3, max: 7 }),
            hold: BoolParam::new("Hold", false),
            shared: Arc::new(Shared::default()),
        }
    }
}

// SAFETY: the pointers point into this struct, which the plugin keeps alive in an Arc for as long as the host uses them.
unsafe impl Params for KillerParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        vec![
            ("scale".into(), self.scale.as_ptr(), String::new()),
            ("root".into(), self.root.as_ptr(), String::new()),
            ("octave".into(), self.octave.as_ptr(), String::new()),
            ("hold".into(), self.hold.as_ptr(), String::new()),
        ]
    }
}

/// How long the chromagram remembers: about a minute, the whole of most loops.
const MEMORY_SECS: f32 = 60.0;
/// Seconds another key has to keep winning before the shown key changes to it.
const SWITCH_SECS: f32 = 3.0;
/// Frames' worth of sound (decayed) before a key is shown.
const MIN_WEIGHT: f32 = 4.0;

pub struct SampleKiller {
    params: Arc<KillerParams>,
    analyzer: dsp::Analyzer,
    acc: [f32; 12],
    weight: f32,
    frame_decay: f32,
    /// The key shown, and a challenger with how many frames in a row it has won.
    shown: Option<theory::Key>,
    challenger: Option<(theory::Key, u32)>,
    switch_frames: u32,
    level_decay_per_sample: f32,
    level: f32,
    sample_rate: f32,
}

impl Default for SampleKiller {
    fn default() -> Self {
        SampleKiller {
            params: Arc::new(KillerParams::default()),
            analyzer: dsp::Analyzer::new(),
            acc: [0.0; 12],
            weight: 0.0,
            frame_decay: 1.0,
            shown: None,
            challenger: None,
            switch_frames: 32,
            level_decay_per_sample: 1.0,
            level: 0.0,
            sample_rate: 44100.0,
        }
    }
}

impl SampleKiller {
    fn clear(&mut self) {
        self.acc = [0.0; 12];
        self.weight = 0.0;
        self.shown = None;
        self.challenger = None;
        let s = &self.params.shared;
        s.chroma.iter().for_each(|c| store_f32(c, 0.0));
        s.key_root.store(NO_KEY, Ordering::Relaxed);
        store_f32(&s.confidence, 0.0);
    }

    fn on_frame(&mut self) {
        let frame = self.analyzer.frame_chroma;
        if self.params.hold.value() || frame.iter().all(|&v| v == 0.0) {
            return;
        }
        for (a, f) in self.acc.iter_mut().zip(frame) {
            *a = *a * self.frame_decay + f;
        }
        self.weight = self.weight * self.frame_decay + 1.0;
        let s = self.params.shared.clone();
        let max = self.acc.iter().copied().fold(0.0f32, f32::max);
        if max > 0.0 {
            for (c, a) in s.chroma.iter().zip(self.acc) {
                store_f32(c, a / max);
            }
        }
        if self.weight < MIN_WEIGHT {
            return;
        }
        let Some(k) = theory::estimate_key(&self.acc) else { return };
        let same = |a: &theory::Key| (a.root, a.minor) == (k.root, k.minor);
        match self.shown {
            Some(cur) if same(&cur) => {
                self.shown = Some(k);
                self.challenger = None;
            }
            Some(_) => {
                let won = match self.challenger {
                    Some((c, n)) if same(&c) => n + 1,
                    _ => 1,
                };
                self.challenger = Some((k, won));
                if won >= self.switch_frames {
                    self.shown = Some(k);
                    self.challenger = None;
                }
            }
            None => self.shown = Some(k),
        }
        if let Some(k) = self.shown {
            s.key_root.store(k.root as u32, Ordering::Relaxed);
            s.key_minor.store(k.minor, Ordering::Relaxed);
            store_f32(&s.confidence, k.confidence);
        }
    }
}

impl Plugin for SampleKiller {
    const NAME: &'static str = "Sample Killer";
    const VENDOR: &'static str = "ItsAlphaNeon";
    const URL: &'static str = "https://github.com/ItsAlphaNeon/Lemon-Studio";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout { main_input_channels: NonZeroU32::new(2), main_output_channels: NonZeroU32::new(2), ..AudioIOLayout::const_default() },
        AudioIOLayout { main_input_channels: NonZeroU32::new(1), main_output_channels: NonZeroU32::new(1), ..AudioIOLayout::const_default() },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, config: &BufferConfig, _ctx: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = config.sample_rate;
        self.analyzer.configure(config.sample_rate);
        let hop = self.analyzer.hop() as f32;
        self.frame_decay = (-hop / (config.sample_rate * MEMORY_SECS)).exp();
        self.switch_frames = ((SWITCH_SECS * config.sample_rate / hop) as u32).max(1);
        self.level_decay_per_sample = (-1.0 / (config.sample_rate * 0.3)).exp();
        self.clear();
        true
    }

    fn reset(&mut self) {
        self.analyzer.reset();
        self.level = 0.0;
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, _ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        if self.params.shared.reset.swap(false, Ordering::Relaxed) {
            self.clear();
        }
        let channels = buffer.as_slice();
        if channels.is_empty() {
            return ProcessStatus::Normal;
        }
        let gain = 1.0 / channels.len() as f32;
        for i in 0..channels[0].len() {
            let mono = channels.iter().map(|ch| ch[i]).sum::<f32>() * gain;
            self.level = (self.level * self.level_decay_per_sample).max(mono.abs());
            if self.analyzer.push(mono) {
                self.on_frame();
            }
        }
        store_f32(&self.params.shared.level, self.level);
        ProcessStatus::Normal
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(editor::KillerEditor::new(self.params.clone())))
    }
}

impl ClapPlugin for SampleKiller {
    const CLAP_ID: &'static str = "org.lemonstudio.sample-killer";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Finds a sample's key and suggests the 808 and melody notes that fit");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Analyzer, ClapFeature::Utility];
}

impl Vst3Plugin for SampleKiller {
    const VST3_CLASS_ID: [u8; 16] = *b"SampleKillerLmSt";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Analyzer, Vst3SubCategory::Tools];
}

#[cfg(not(feature = "linked"))]
nih_export_clap!(SampleKiller);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(SampleKiller);
os_plugkit::linked_clap_entry!(SampleKiller);
