// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Note ID: pitch detector with 808 bass and melody note suggestions as a VST3 / CLAP plugin
//! (nih-plug). Receives audio, detects the fundamental frequency with YIN, and shows the note,
//! compatible 808 bass companions, and scale melody notes in a piano-keyboard UI.
#![allow(semicolon_in_expressions_from_macros)]

pub mod dsp;
pub mod editor;
pub mod theory;
pub mod view;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

use nih_plug::prelude::*;

use dsp::YinDetector;

/// State shared lock-free between the audio thread and the editor thread.
#[derive(Default)]
pub struct Shared {
    /// Detected MIDI note (0–127); 255 = no signal.
    pub midi_note: AtomicU8,
    /// Detected frequency as f32 bits; 0.0 = no signal.
    pub freq_bits: AtomicU32,
}

pub struct NoteIdParams {
    /// Selected scale (0=Minor … 4=Major). Hosts show its value_to_string label.
    pub scale: IntParam,
    /// Shared state: not a parameter, ignored by the Params impl.
    pub shared: Arc<Shared>,
}

impl Default for NoteIdParams {
    fn default() -> Self {
        NoteIdParams {
            scale: IntParam::new("Scale", 0, IntRange::Linear { min: 0, max: theory::SCALE_COUNT - 1 })
                .with_value_to_string(Arc::new(|v| theory::SCALE_NAMES[v.clamp(0, theory::SCALE_COUNT - 1) as usize].to_string())),
            shared: Arc::new(Shared::default()),
        }
    }
}

// SAFETY: `param_map` hands the host stable references into this Arc-pinned struct.
// `shared` is intentionally absent from the map — it carries no host-visible parameter.
unsafe impl Params for NoteIdParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        vec![("scale".into(), self.scale.as_ptr(), String::new())]
    }
}

pub struct NoteId {
    params: Arc<NoteIdParams>,
    yin: YinDetector,
    sample_rate: f32,
    block_count: u32,
}

impl Default for NoteId {
    fn default() -> Self {
        NoteId {
            params: Arc::new(NoteIdParams::default()),
            yin: YinDetector::new(),
            sample_rate: 44100.0,
            block_count: 0,
        }
    }
}

impl Plugin for NoteId {
    const NAME: &'static str = "Lemon Note ID";
    const VENDOR: &'static str = "ItsAlphaNeon";
    const URL: &'static str = "https://github.com/ItsAlphaNeon/Lemon-Studio";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        config: &BufferConfig,
        _ctx: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = config.sample_rate;
        self.yin = YinDetector::new();
        self.block_count = 0;
        true
    }

    fn reset(&mut self) {
        self.yin = YinDetector::new();
        self.block_count = 0;
        self.params.shared.midi_note.store(255, Ordering::Relaxed);
        self.params.shared.freq_bits.store(0f32.to_bits(), Ordering::Relaxed);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.block_count = self.block_count.wrapping_add(1);

        // Accumulate mono samples into the YIN ring buffer.
        let channels = buffer.as_slice();
        if !channels.is_empty() {
            let n_ch = channels.len() as f32;
            let n_frames = channels[0].len();
            for i in 0..n_frames {
                let mono: f32 = channels.iter().map(|ch| ch[i]).sum::<f32>() / n_ch;
                self.yin.push(mono);
            }
        }

        // Run pitch detection every 8 blocks (~40 ms at 44100 / 512) to bound CPU use.
        if self.block_count % 8 == 0 {
            match self.yin.detect(self.sample_rate) {
                Some(freq) => {
                    self.params.shared.freq_bits.store(freq.to_bits(), Ordering::Relaxed);
                    match theory::freq_to_midi(freq) {
                        Some(midi) => self.params.shared.midi_note.store(midi, Ordering::Relaxed),
                        None => self.params.shared.midi_note.store(255, Ordering::Relaxed),
                    }
                }
                None => {
                    self.params.shared.freq_bits.store(0f32.to_bits(), Ordering::Relaxed);
                    self.params.shared.midi_note.store(255, Ordering::Relaxed);
                }
            }
        }

        ProcessStatus::Normal
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(editor::NoteIdEditor::new(self.params.clone())))
    }
}

impl ClapPlugin for NoteId {
    const CLAP_ID: &'static str = "org.openstudio.lemon-note-id";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Pitch detector with 808 bass and melody note suggestions");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Analyzer, ClapFeature::Utility];
}

impl Vst3Plugin for NoteId {
    const VST3_CLASS_ID: [u8; 16] = *b"LemonNoteIDPlugn";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Analyzer, Vst3SubCategory::Tools];
}

#[cfg(not(feature = "linked"))]
nih_export_clap!(NoteId);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(NoteId);
os_plugkit::linked_clap_entry!(NoteId);
